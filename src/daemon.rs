use crate::database::Database;
use notify_rust::Notification;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn pid_file_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_dir = dirs::data_dir().ok_or("Could not find data directory")?;
    Ok(data_dir.join("break").join("daemon.pid"))
}

pub fn is_daemon_running() -> Result<bool, Box<dyn std::error::Error>> {
    let pid_file = pid_file_path()?;

    if !pid_file.exists() {
        return Ok(false);
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    let pid: u32 = pid_str.trim().parse().unwrap_or(0);

    if pid == 0 {
        return Ok(false);
    }

    // Check if process is actually running
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("ps")
            .arg("-p")
            .arg(pid.to_string())
            .output()?;
        Ok(output.status.success())
    }

    #[cfg(not(unix))]
    {
        // On non-Unix, just assume it's running if PID file exists
        Ok(true)
    }
}

pub fn ensure_daemon_running() -> Result<(), Box<dyn std::error::Error>> {
    if !is_daemon_running()? {
        start_daemon_process()?;
    }
    Ok(())
}

pub fn start_daemon_process() -> Result<(), Box<dyn std::error::Error>> {
    if is_daemon_running()? {
        return Ok(());
    }

    // Get the current executable path
    let exe = std::env::current_exe()?;

    // Spawn daemon as a detached background process
    #[cfg(unix)]
    {
        Command::new(exe)
            .arg("--daemon-mode")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    #[cfg(not(unix))]
    {
        Command::new(exe)
            .arg("--daemon-mode")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }

    Ok(())
}

pub fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    // Write PID file
    let pid_file = pid_file_path()?;
    if let Some(parent) = pid_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&pid_file, std::process::id().to_string())?;

    // Main daemon loop
    loop {
        // Check for expired timers
        let mut db = Database::load()?;
        let expired = db.get_expired_timers();

        for timer in &expired {
            // Build notification with appropriate settings
            // Use the timer message as the title for immediate visibility
            let notification = Notification::new()
                .summary(&timer.message)
                .body("Break timer completed")
                .urgency(if timer.urgent {
                    notify_rust::Urgency::Critical
                } else {
                    notify_rust::Urgency::Normal
                })
                .sound_name(if timer.sound {
                    "message-new-instant"
                } else {
                    // Empty string means no sound
                    ""
                })
                .finalize();

            // Show notification
            let _ = notification.show();

            // Handle recurring vs one-time timers
            if timer.recurring {
                // Add to history and reset the timer for the next interval
                db.add_to_history(timer.clone());
                db.reset_timer(timer.id);
            } else {
                // Complete the timer (moves to history)
                db.complete_timer(timer.id);
            }
        }

        if !expired.is_empty() {
            db.save()?;
        }

        // If no more timers, exit daemon
        if db.timers.is_empty() {
            break;
        }

        // Calculate sleep time until next timer
        let now = time::OffsetDateTime::now_utc();
        let next_timer = db.timers.iter()
            .min_by_key(|t| t.due_at);

        let sleep_duration = if let Some(next) = next_timer {
            let time_until = next.due_at - now;
            let seconds = time_until.whole_seconds();
            if seconds > 0 {
                // Sleep until just past the timer (add 1 second buffer)
                Duration::from_secs((seconds + 1) as u64)
            } else {
                // Timer already expired, check immediately
                Duration::from_secs(1)
            }
        } else {
            // Fallback to 30 seconds if no timer found
            Duration::from_secs(30)
        };

        // Cap sleep duration at 1 hour for safety
        let sleep_duration = sleep_duration.min(Duration::from_secs(3600));

        thread::sleep(sleep_duration);
    }

    // Clean up PID file
    let _ = fs::remove_file(&pid_file);

    Ok(())
}
