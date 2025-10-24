//! Timer database with persistence and concurrency control.
//!
//! This module provides a JSON-based database for storing active timers and
//! timer history, with file locking to prevent corruption from concurrent access.

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timer {
    pub uuid: Uuid,
    pub id: u32,
    pub message: String,
    pub duration_seconds: u64,
    #[serde(with = "time::serde::timestamp")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::timestamp")]
    pub due_at: OffsetDateTime,
    #[serde(default)]
    pub urgent: bool,
    #[serde(default)]
    pub sound: bool,
    #[serde(default)]
    pub recurring: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    pub timers: Vec<Timer>,
    #[serde(default)]
    pub history: Vec<Timer>,
    next_id: u32,
}

impl Database {
    pub fn new() -> Self {
        Self {
            timers: Vec::new(),
            history: Vec::new(),
            next_id: 1,
        }
    }

    /// Loads the database from disk with a shared lock for read-only access.
    ///
    /// Multiple readers can access the database simultaneously. This is suitable for
    /// operations like listing timers or checking status that don't modify the database.
    ///
    /// # Returns
    ///
    /// Returns a new `Database` instance if the file doesn't exist, or loads the
    /// existing database from `~/.local/share/break/timers.json`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database file is corrupted or contains invalid JSON
    /// - File permissions prevent reading
    /// - The data directory cannot be accessed
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::db_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        // Open file with shared lock (multiple readers allowed)
        let file = File::open(&path)?;
        FileExt::lock_shared(&file)?;

        let mut contents = String::new();
        let mut reader = std::io::BufReader::new(&file);
        reader.read_to_string(&mut contents)?;

        // Parse JSON with better error messages
        let db: Database = serde_json::from_str(&contents).map_err(|e| {
            format!(
                "Database file is corrupted or invalid. Error: {}\nLocation: {}\nTo fix: Delete the file and restart.",
                e,
                path.display()
            )
        })?;

        FileExt::unlock(&file)?;
        Ok(db)
    }

    /// Executes a load-modify-save transaction with an exclusive lock held throughout.
    ///
    /// This ensures atomic database updates by holding an exclusive file lock for the
    /// entire operation. Only one writer can execute a transaction at a time, preventing
    /// race conditions and data corruption from concurrent modifications.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that receives a mutable reference to the database and returns
    ///   a result. The closure can modify the database freely, and changes are
    ///   automatically saved when the closure completes successfully.
    ///
    /// # Returns
    ///
    /// Returns the value returned by the closure on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database file cannot be opened or locked
    /// - The database file is corrupted
    /// - The closure returns an error
    /// - Saving the modified database fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use breakrs::database::Database;
    /// Database::with_transaction(|db| {
    ///     db.add_timer("Coffee break".to_string(), 300, false, false, false)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_transaction<F, T>(mut f: F) -> Result<T, Box<dyn std::error::Error>>
    where
        F: FnMut(&mut Database) -> Result<T, Box<dyn std::error::Error>>,
    {
        let path = Self::db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Open/create file with exclusive lock for entire transaction
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false) // Don't truncate - we need to read existing data first
            .open(&path)?;

        FileExt::lock_exclusive(&file)?;

        // Load database
        let mut db = if file.metadata()?.len() == 0 {
            // Empty file, create new database
            Self::new()
        } else {
            let mut contents = String::new();
            let mut reader = std::io::BufReader::new(&file);
            reader.read_to_string(&mut contents)?;

            serde_json::from_str(&contents).map_err(|e| {
                format!(
                    "Database file is corrupted or invalid. Error: {}\nLocation: {}\nTo fix: Delete the file and restart.",
                    e,
                    path.display()
                )
            })?
        };

        // Run the transaction function
        let result = f(&mut db)?;

        // Save database
        let contents = serde_json::to_string_pretty(&db)?;
        let file = OpenOptions::new().write(true).truncate(true).open(&path)?;
        let mut writer = std::io::BufWriter::new(&file);
        writer.write_all(contents.as_bytes())?;
        writer.flush()?;

        FileExt::unlock(&file)?;

        Ok(result)
    }

    /// Save database (use with_transaction instead for modifications)
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::db_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Open/create file with exclusive lock (only one writer)
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        FileExt::lock_exclusive(&file)?;

        let contents = serde_json::to_string_pretty(self)?;
        let mut writer = std::io::BufWriter::new(&file);
        writer.write_all(contents.as_bytes())?;
        writer.flush()?;

        FileExt::unlock(&file)?;
        Ok(())
    }

    /// Adds a new timer to the database.
    ///
    /// # Arguments
    ///
    /// * `message` - The timer message to display when it expires
    /// * `duration_seconds` - Duration in seconds (max 1 year)
    /// * `urgent` - Whether to mark notification as urgent/critical
    /// * `sound` - Whether to play sound with notification
    /// * `recurring` - Whether timer should repeat after completion
    ///
    /// # Returns
    ///
    /// Returns the created `Timer` with assigned ID and calculated due time.
    ///
    /// # Errors
    ///
    /// Returns an error if the duration exceeds 1 year (31,536,000 seconds).
    pub fn add_timer(
        &mut self,
        message: String,
        duration_seconds: u64,
        urgent: bool,
        sound: bool,
        recurring: bool,
    ) -> Result<Timer, String> {
        // Validate duration is reasonable (max 1 year = 31,536,000 seconds)
        const MAX_DURATION_SECONDS: u64 = 365 * 24 * 60 * 60; // 1 year
        if duration_seconds > MAX_DURATION_SECONDS {
            return Err(format!(
                "Duration too large (max {} days)",
                MAX_DURATION_SECONDS / 86400
            ));
        }

        let now = OffsetDateTime::now_utc();
        let due_at = now + time::Duration::seconds(duration_seconds as i64);

        let timer = Timer {
            uuid: Uuid::new_v4(),
            id: self.next_id,
            message,
            duration_seconds,
            created_at: now,
            due_at,
            urgent,
            sound,
            recurring,
        };

        self.next_id += 1;
        self.timers.push(timer.clone());
        Ok(timer)
    }

    /// Resets a timer to start over from the current time.
    ///
    /// This is primarily used for recurring timers that need to repeat after completion.
    /// The timer's `created_at` is set to now and `due_at` is recalculated based on
    /// the original duration.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the timer to reset
    ///
    /// # Returns
    ///
    /// Returns `Some(Timer)` with the updated timer if found, `None` if no timer
    /// with the given ID exists.
    pub fn reset_timer(&mut self, id: u32) -> Option<Timer> {
        if let Some(timer) = self.timers.iter_mut().find(|t| t.id == id) {
            let now = OffsetDateTime::now_utc();
            timer.due_at = now + time::Duration::seconds(timer.duration_seconds as i64);
            timer.created_at = now;
            Some(timer.clone())
        } else {
            None
        }
    }

    /// Removes a timer from the active timers list without adding it to history.
    ///
    /// This is used when a user explicitly cancels/removes a timer. For timers that
    /// complete naturally, use `complete_timer()` instead to add them to history.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the timer to remove
    ///
    /// # Returns
    ///
    /// Returns `Some(Timer)` containing the removed timer if found, `None` if no
    /// timer with the given ID exists.
    pub fn remove_timer(&mut self, id: u32) -> Option<Timer> {
        if let Some(pos) = self.timers.iter().position(|t| t.id == id) {
            Some(self.timers.remove(pos))
        } else {
            None
        }
    }

    /// Completes a timer by removing it from active timers and adding it to history.
    ///
    /// This is the proper way to handle timer expiration. The timer is removed from
    /// the active list and added to the front of the history list for tracking purposes.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the timer to complete
    ///
    /// # Returns
    ///
    /// Returns `Some(Timer)` containing the completed timer if found, `None` if no
    /// timer with the given ID exists.
    pub fn complete_timer(&mut self, id: u32) -> Option<Timer> {
        if let Some(pos) = self.timers.iter().position(|t| t.id == id) {
            let timer = self.timers.remove(pos);
            self.add_to_history(timer.clone());
            Some(timer)
        } else {
            None
        }
    }

    /// Adds a completed timer to the history list.
    ///
    /// History is maintained as a most-recent-first list with a maximum of 20 entries.
    /// When the limit is exceeded, the oldest entries are removed.
    ///
    /// This allows users to see recently completed timers even if they missed the
    /// notification.
    ///
    /// # Arguments
    ///
    /// * `timer` - The timer to add to history
    pub fn add_to_history(&mut self, timer: Timer) {
        const MAX_HISTORY: usize = 20;

        // Add to front of history (most recent first)
        self.history.insert(0, timer);

        // Keep only last MAX_HISTORY entries
        if self.history.len() > MAX_HISTORY {
            self.history.truncate(MAX_HISTORY);
        }
    }

    /// Clears all active timers.
    ///
    /// This removes all timers from the active list without adding them to history.
    /// Used when the user wants to cancel all pending timers at once.
    pub fn clear_all(&mut self) {
        self.timers.clear();
    }

    /// Clears the history of completed timers.
    ///
    /// This removes all entries from the history list, providing a fresh start
    /// for tracking recently completed timers.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Returns all timers that have expired (due_at is in the past).
    ///
    /// This is used by the daemon to identify which timers need to fire notifications.
    /// Timers are considered expired when their `due_at` time is less than or equal
    /// to the current UTC time.
    ///
    /// # Returns
    ///
    /// A vector containing clones of all expired timers. Returns an empty vector
    /// if no timers have expired.
    pub fn get_expired_timers(&self) -> Vec<Timer> {
        let now = OffsetDateTime::now_utc();
        self.timers
            .iter()
            .filter(|t| t.due_at <= now)
            .cloned()
            .collect()
    }

    fn db_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let data_dir = dirs::data_dir().ok_or("Could not find data directory")?;
        Ok(data_dir.join("break").join("timers.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_database() {
        let db = Database::new();
        assert_eq!(db.timers.len(), 0);
        assert_eq!(db.history.len(), 0);
        assert_eq!(db.next_id, 1);
    }

    #[test]
    fn test_add_timer() {
        let mut db = Database::new();
        let timer = db
            .add_timer("Test".to_string(), 300, false, false, false)
            .unwrap();

        assert_eq!(timer.id, 1);
        assert_eq!(timer.message, "Test");
        assert_eq!(timer.duration_seconds, 300);
        assert_eq!(db.timers.len(), 1);
        assert_eq!(db.next_id, 2);
    }

    #[test]
    fn test_add_timer_max_duration() {
        let mut db = Database::new();
        let max_duration = 365 * 24 * 60 * 60; // 1 year

        // Should succeed at max duration
        let result = db.add_timer("Max".to_string(), max_duration, false, false, false);
        assert!(result.is_ok());

        // Should fail above max duration
        let result = db.add_timer(
            "Too long".to_string(),
            max_duration + 1,
            false,
            false,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duration too large"));
    }

    #[test]
    fn test_remove_timer() {
        let mut db = Database::new();
        let timer = db
            .add_timer("Test".to_string(), 300, false, false, false)
            .unwrap();

        let removed = db.remove_timer(timer.id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, timer.id);
        assert_eq!(db.timers.len(), 0);

        // Removing non-existent timer should return None
        let removed = db.remove_timer(999);
        assert!(removed.is_none());
    }

    #[test]
    fn test_complete_timer() {
        let mut db = Database::new();
        let timer = db
            .add_timer("Test".to_string(), 300, false, false, false)
            .unwrap();

        let completed = db.complete_timer(timer.id);
        assert!(completed.is_some());
        assert_eq!(db.timers.len(), 0);
        assert_eq!(db.history.len(), 1);
        assert_eq!(db.history[0].id, timer.id);
    }

    #[test]
    fn test_reset_timer() {
        let mut db = Database::new();
        let timer = db
            .add_timer("Test".to_string(), 300, false, false, false)
            .unwrap();
        let original_due = timer.due_at;

        // Wait a tiny bit and reset
        std::thread::sleep(std::time::Duration::from_millis(10));

        let reset = db.reset_timer(timer.id);
        assert!(reset.is_some());

        // Due time should be updated (different from original)
        let reset_timer = reset.unwrap();
        assert!(reset_timer.created_at > timer.created_at);
        assert!(reset_timer.due_at > original_due);
    }

    #[test]
    fn test_history_max_entries() {
        let mut db = Database::new();

        // Add 25 timers and complete them all
        for i in 1..=25 {
            let timer = db
                .add_timer(format!("Timer {}", i), 10, false, false, false)
                .unwrap();
            db.complete_timer(timer.id);
        }

        // Should only keep last 20
        assert_eq!(db.history.len(), 20);

        // Most recent should be first (Timer 25)
        assert_eq!(db.history[0].message, "Timer 25");
        // Oldest in history should be Timer 6 (25 - 20 + 1)
        assert_eq!(db.history[19].message, "Timer 6");
    }

    #[test]
    fn test_clear_all() {
        let mut db = Database::new();
        db.add_timer("Test 1".to_string(), 300, false, false, false)
            .unwrap();
        db.add_timer("Test 2".to_string(), 600, false, false, false)
            .unwrap();

        assert_eq!(db.timers.len(), 2);
        db.clear_all();
        assert_eq!(db.timers.len(), 0);

        // History should not be affected
        db.add_to_history(Timer {
            uuid: uuid::Uuid::new_v4(),
            id: 1,
            message: "History".to_string(),
            duration_seconds: 100,
            created_at: OffsetDateTime::now_utc(),
            due_at: OffsetDateTime::now_utc(),
            urgent: false,
            sound: false,
            recurring: false,
        });
        assert_eq!(db.history.len(), 1);
        db.clear_all();
        assert_eq!(db.history.len(), 1); // Still there
    }

    #[test]
    fn test_clear_history() {
        let mut db = Database::new();
        let timer = db
            .add_timer("Test".to_string(), 300, false, false, false)
            .unwrap();
        db.complete_timer(timer.id);

        assert_eq!(db.history.len(), 1);
        db.clear_history();
        assert_eq!(db.history.len(), 0);
    }

    #[test]
    fn test_get_expired_timers() {
        let mut db = Database::new();

        // Add a timer that's already expired (0 seconds)
        let expired_timer = db
            .add_timer("Expired".to_string(), 0, false, false, false)
            .unwrap();

        // Add a future timer
        db.add_timer("Future".to_string(), 3600, false, false, false)
            .unwrap();

        // Small delay to ensure the 0-second timer is definitely expired
        std::thread::sleep(std::time::Duration::from_millis(10));

        let expired = db.get_expired_timers();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].id, expired_timer.id);
    }

    #[test]
    fn test_timer_flags() {
        let mut db = Database::new();

        // Test all flags
        let timer = db
            .add_timer("Urgent sound recurring".to_string(), 300, true, true, true)
            .unwrap();
        assert!(timer.urgent);
        assert!(timer.sound);
        assert!(timer.recurring);

        // Test default flags
        let timer = db
            .add_timer("Default".to_string(), 300, false, false, false)
            .unwrap();
        assert!(!timer.urgent);
        assert!(!timer.sound);
        assert!(!timer.recurring);
    }

    #[test]
    fn test_sequential_ids() {
        let mut db = Database::new();

        let timer1 = db
            .add_timer("First".to_string(), 300, false, false, false)
            .unwrap();
        let timer2 = db
            .add_timer("Second".to_string(), 300, false, false, false)
            .unwrap();
        let timer3 = db
            .add_timer("Third".to_string(), 300, false, false, false)
            .unwrap();

        assert_eq!(timer1.id, 1);
        assert_eq!(timer2.id, 2);
        assert_eq!(timer3.id, 3);

        // Even after removing, next ID should continue
        db.remove_timer(timer2.id);
        let timer4 = db
            .add_timer("Fourth".to_string(), 300, false, false, false)
            .unwrap();
        assert_eq!(timer4.id, 4);
    }
}
