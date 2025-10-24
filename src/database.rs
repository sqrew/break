use serde::{Deserialize, Serialize};
use std::fs;
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
    next_id: u32,
}

impl Database {
    pub fn new() -> Self {
        Self {
            timers: Vec::new(),
            next_id: 1,
        }
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::db_path()?;

        if !path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(path)?;
        let db: Database = serde_json::from_str(&contents)?;
        Ok(db)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::db_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn add_timer(&mut self, message: String, duration_seconds: u64, urgent: bool, sound: bool, recurring: bool) -> Timer {
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
        timer
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

    pub fn clear_all(&mut self) {
        self.timers.clear();
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
