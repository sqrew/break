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

    /// Load database for read-only access (list, status, etc.)
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

    /// Load-Modify-Save transaction with exclusive lock held throughout
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
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)?;
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

    pub fn add_timer(&mut self, message: String, duration_seconds: u64, urgent: bool, sound: bool, recurring: bool) -> Result<Timer, String> {
        // Validate duration is reasonable (max 1 year = 31,536,000 seconds)
        const MAX_DURATION_SECONDS: u64 = 365 * 24 * 60 * 60; // 1 year
        if duration_seconds > MAX_DURATION_SECONDS {
            return Err(format!("Duration too large (max {} days)", MAX_DURATION_SECONDS / 86400));
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

    pub fn remove_timer(&mut self, id: u32) -> Option<Timer> {
        if let Some(pos) = self.timers.iter().position(|t| t.id == id) {
            Some(self.timers.remove(pos))
        } else {
            None
        }
    }

    pub fn complete_timer(&mut self, id: u32) -> Option<Timer> {
        if let Some(pos) = self.timers.iter().position(|t| t.id == id) {
            let timer = self.timers.remove(pos);
            self.add_to_history(timer.clone());
            Some(timer)
        } else {
            None
        }
    }

    pub fn add_to_history(&mut self, timer: Timer) {
        const MAX_HISTORY: usize = 20;

        // Add to front of history (most recent first)
        self.history.insert(0, timer);

        // Keep only last MAX_HISTORY entries
        if self.history.len() > MAX_HISTORY {
            self.history.truncate(MAX_HISTORY);
        }
    }

    pub fn clear_all(&mut self) {
        self.timers.clear();
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn get_expired_timers(&self) -> Vec<Timer> {
        let now = OffsetDateTime::now_utc();
        self.timers
            .iter()
            .filter(|t| t.due_at <= now)
            .cloned()
            .collect()
    }

    fn db_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let data_dir = dirs::data_dir()
            .ok_or("Could not find data directory")?;
        Ok(data_dir.join("break").join("timers.json"))
    }
}
