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
            // Send notification
            let _ = Notification::new()
                .summary("Break Timer")
                .body(&timer.message)
                .show();

            // Remove the expired timer
            db.remove_timer(timer.id);
        }

        if !expired.is_empty() {
            db.save()?;
        }

        // If no more timers, exit daemon
        if db.timers.is_empty() {
            break;
        }

        // Sleep for 30 seconds before next check
        thread::sleep(Duration::from_secs(30));
    }

    // Clean up PID file
    let _ = fs::remove_file(&pid_file);

    Ok(())
}
