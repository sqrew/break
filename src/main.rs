use clap::{Parser, Subcommand};
use std::process;

mod database;
mod daemon;
mod parser;

use database::Database;

#[derive(Parser)]
#[command(name = "break")]
#[command(about = "A simple CLI timer for breaks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Duration (e.g., "5m", "1h30m")
    duration: Option<String>,

    /// Timer message (remaining arguments will be joined)
    #[arg(trailing_var_arg = true)]
    message: Vec<String>,

    /// Run in daemon mode (internal use)
    #[arg(long, hide = true)]
    daemon_mode: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List all active timers
    List,
    /// Remove a timer by ID
    Remove { id: u32 },
    /// Clear all timers
    Clear,
    /// Show daemon status
    Status,
    /// Manually start the daemon
    Daemon,
}

fn main() {
    let cli = Cli::parse();

    // Handle daemon mode (internal use)
    if cli.daemon_mode {
        if let Err(e) = daemon::run_daemon() {
            eprintln!("Daemon error: {}", e);
            process::exit(1);
        }
        return;
    }

    let result = match cli.command {
        Some(Commands::List) => list_timers(),
        Some(Commands::Remove { id }) => remove_timer(id),
        Some(Commands::Clear) => clear_timers(),
        Some(Commands::Status) => show_status(),
        Some(Commands::Daemon) => start_daemon(),
        None => {
            // Default: add a timer
            if let Some(duration) = cli.duration {
                if cli.message.is_empty() {
                    eprintln!("Error: Please provide a message");
                    eprintln!("Usage: break <duration> <message>");
                    eprintln!("Example: break 5m Tea is ready");
                    process::exit(1);
                }
                let message = cli.message.join(" ");
                add_timer(&duration, &message)
            } else {
                eprintln!("Error: Please provide duration and message");
                eprintln!("Usage: break <duration> <message>");
                eprintln!("Example: break 5m Tea is ready");
                process::exit(1);
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn add_timer(duration_str: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let duration_seconds = parser::parse_duration(duration_str)?;

    let mut db = Database::load()?;
    let timer = db.add_timer(message.to_string(), duration_seconds);
    db.save()?;

    println!("Timer #{} set for {} ({} seconds)", timer.id, message, duration_seconds);
    println!("Break will notify you at {}", timer.due_at.format(&time::format_description::well_known::Rfc3339)?);

    // Ensure daemon is running
    daemon::ensure_daemon_running()?;

    Ok(())
}

fn list_timers() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::load()?;

    if db.timers.is_empty() {
        println!("No active timers");
        return Ok(());
    }

    println!("Active timers:");
    for timer in &db.timers {
        let now = time::OffsetDateTime::now_utc();
        let remaining = timer.due_at - now;
        let remaining_secs = remaining.whole_seconds();

        if remaining_secs > 0 {
            let hours = remaining_secs / 3600;
            let minutes = (remaining_secs % 3600) / 60;
            let seconds = remaining_secs % 60;

            print!("  #{}: \"{}\" - ", timer.id, timer.message);
            if hours > 0 {
                print!("{}h ", hours);
            }
            if minutes > 0 || hours > 0 {
                print!("{}m ", minutes);
            }
            println!("{}s remaining", seconds);
        } else {
            println!("  #{}: \"{}\" - EXPIRED", timer.id, timer.message);
        }
    }

    Ok(())
}

fn remove_timer(id: u32) -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Database::load()?;

    if let Some(timer) = db.remove_timer(id) {
        db.save()?;
        println!("Removed timer #{}: \"{}\"", timer.id, timer.message);
    } else {
        println!("Timer #{} not found", id);
    }

    Ok(())
}

fn clear_timers() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Database::load()?;
    let count = db.timers.len();
    db.clear_all();
    db.save()?;

    println!("Cleared {} timer(s)", count);

    Ok(())
}

fn show_status() -> Result<(), Box<dyn std::error::Error>> {
    if daemon::is_daemon_running()? {
        println!("Daemon is running");
        let db = Database::load()?;
        println!("Active timers: {}", db.timers.len());
    } else {
        println!("Daemon is not running");
    }

    Ok(())
}

fn start_daemon() -> Result<(), Box<dyn std::error::Error>> {
    daemon::start_daemon_process()?;
    println!("Daemon started");
    Ok(())
}
