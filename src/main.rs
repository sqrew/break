use clap::{Parser, Subcommand};
use std::process;

mod daemon;
mod database;
mod parser;

use database::Database;

#[derive(Parser)]
#[command(name = "breakrs")]
#[command(about = "A simple CLI timer for breaks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Input mixing duration and message (e.g., "5m get coffee", "15mins 1h 20s take a break")
    #[arg(trailing_var_arg = true)]
    input: Vec<String>,

    /// Mark notification as urgent/critical
    #[arg(long, short = 'u')]
    urgent: bool,

    /// Play sound with notification
    #[arg(long, short = 's')]
    sound: bool,

    /// Make timer recurring (repeats after completion)
    #[arg(long, short = 'r')]
    recurring: bool,

    /// Run in daemon mode (internal use)
    #[arg(long, hide = true)]
    daemon_mode: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List all active timers
    #[command(aliases = ["l", "li", "lis"])]
    List,
    /// Show recently completed timers
    #[command(aliases = ["h", "hi", "his", "hist", "histo", "histor"])]
    History,
    /// Remove a timer by ID
    #[command(aliases = ["r", "rm", "rem", "remo", "remov"])]
    Remove { id: u32 },
    /// Clear all timers
    #[command(aliases = ["c", "cl", "cle", "clea",])]
    Clear,
    /// Clear history
    #[command(aliases = ["ch", "clh", "clear-h", "clear-hi", "clear-his", "clear-hist", "clear-histo", "clear-histor"])]
    ClearHistory,
    /// Show daemon status
    #[command(aliases = ["s", "st", "sta", "stat", "statu", "stats"])]
    Status,
    /// Manually start the daemon
    #[command(aliases = ["d", "da", "dae", "daem", "daemo"])]
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
        Some(Commands::History) => show_history(),
        Some(Commands::Remove { id }) => remove_timer(id),
        Some(Commands::Clear) => clear_timers(),
        Some(Commands::ClearHistory) => clear_history(),
        Some(Commands::Status) => show_status(),
        Some(Commands::Daemon) => start_daemon(),
        None => {
            // Default: add a timer
            if cli.input.is_empty() {
                eprintln!("Error: Please provide duration and message");
                eprintln!("Usage: break [FLAGS] <input with duration and message>");
                eprintln!("Examples:");
                eprintln!("  break 5m Tea is ready");
                eprintln!("  break 15mins 1 hour 20s take a break");
                eprintln!("  break --urgent 5m get coffee");
                eprintln!("  break 5m get coffee --urgent");
                eprintln!("  break --recurring --sound 1h stretch");
                process::exit(1);
            }

            // Extract flags from input if present
            let (input_cleaned, urgent_flag, sound_flag, recurring_flag) =
                extract_flags_from_input(&cli.input);

            // Combine with CLI flags (either source works)
            let urgent = cli.urgent || urgent_flag;
            let sound = cli.sound || sound_flag;
            let recurring = cli.recurring || recurring_flag;

            add_timer(&input_cleaned, urgent, sound, recurring)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Extract flag arguments from input and return cleaned input plus flag states
fn extract_flags_from_input(input: &[String]) -> (String, bool, bool, bool) {
    let mut urgent = false;
    let mut sound = false;
    let mut recurring = false;
    let mut cleaned_input = Vec::new();

    for arg in input {
        match arg.as_str() {
            "--urgent" => urgent = true,
            "--sound" => sound = true,
            "--recurring" => recurring = true,
            s if s.starts_with('-') && !s.starts_with("--") => {
                // Handle short flags (single dash) including combined flags like -us
                for ch in s.chars().skip(1) {
                    match ch {
                        'u' => urgent = true,
                        's' => sound = true,
                        'r' => recurring = true,
                        _ => {
                            // Unknown flag, treat whole arg as input
                            cleaned_input.push(arg.clone());
                            break;
                        }
                    }
                }
            }
            _ => cleaned_input.push(arg.clone()),
        }
    }

    (cleaned_input.join(" "), urgent, sound, recurring)
}

fn add_timer(
    input: &str,
    urgent: bool,
    sound: bool,
    recurring: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let (duration_seconds, message) = parser::parse_input(input)?;

    // Use transaction to ensure atomic load-modify-save
    let timer = Database::with_transaction(|db| {
        db.add_timer(message.clone(), duration_seconds, urgent, sound, recurring)
            .map_err(|e| format!("Failed to add timer: {}", e).into())
    })?;

    print!(
        "Timer #{} set for \"{}\" ({} seconds)",
        timer.id, message, duration_seconds
    );
    if urgent || sound || recurring {
        print!(" [");
        let mut flags = Vec::new();
        if urgent {
            flags.push("urgent");
        }
        if sound {
            flags.push("sound");
        }
        if recurring {
            flags.push("recurring");
        }
        print!("{}", flags.join(", "));
        print!("]");
    }
    println!();

    // Show relative time (e.g., "in 5 minutes")
    let now = time::OffsetDateTime::now_utc();
    let duration_until = timer.due_at - now;
    let seconds = duration_until.whole_seconds();

    if seconds > 0 {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        print!("Break will notify you in ");
        if hours > 0 {
            print!("{}h ", hours);
        }
        if minutes > 0 || hours > 0 {
            print!("{}m ", minutes);
        }
        if hours == 0 && minutes < 5 {
            print!("{}s", secs);
        }
        println!();
    } else {
        println!("Break notification is ready!");
    }

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

    // Ensure daemon is running if there are active timers
    daemon::ensure_daemon_running()?;

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
            print!("{}s remaining", seconds);

            // Show flags
            if timer.urgent || timer.sound || timer.recurring {
                print!(" [");
                let mut flags = Vec::new();
                if timer.urgent {
                    flags.push("urgent");
                }
                if timer.sound {
                    flags.push("sound");
                }
                if timer.recurring {
                    flags.push("recurring");
                }
                print!("{}", flags.join(", "));
                print!("]");
            }
            println!();
        } else {
            print!("  #{}: \"{}\" - EXPIRED", timer.id, timer.message);
            if timer.urgent || timer.sound || timer.recurring {
                print!(" [");
                let mut flags = Vec::new();
                if timer.urgent {
                    flags.push("urgent");
                }
                if timer.sound {
                    flags.push("sound");
                }
                if timer.recurring {
                    flags.push("recurring");
                }
                print!("{}", flags.join(", "));
                print!("]");
            }
            println!();
        }
    }

    Ok(())
}

fn remove_timer(id: u32) -> Result<(), Box<dyn std::error::Error>> {
    let timer_opt = Database::with_transaction(|db| Ok(db.remove_timer(id)))?;

    if let Some(timer) = timer_opt {
        println!("Removed timer #{}: \"{}\"", timer.id, timer.message);
    } else {
        println!("Timer #{} not found", id);
    }

    Ok(())
}

fn show_history() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::load()?;

    if db.history.is_empty() {
        println!("No completed timers in history");
        return Ok(());
    }

    println!("Recently completed timers:");
    for timer in &db.history {
        let now = time::OffsetDateTime::now_utc();
        let elapsed = now - timer.due_at;
        let elapsed_secs = elapsed.whole_seconds().abs();

        let hours = elapsed_secs / 3600;
        let minutes = (elapsed_secs % 3600) / 60;

        print!("  #{}: \"{}\" - completed ", timer.id, timer.message);
        if hours > 0 {
            print!("{}h ", hours);
        }
        if minutes > 0 || hours > 0 {
            print!("{}m ", minutes);
        } else {
            print!("< 1m ");
        }
        print!("ago");

        // Show flags
        if timer.urgent || timer.sound || timer.recurring {
            print!(" [");
            let mut flags = Vec::new();
            if timer.urgent {
                flags.push("urgent");
            }
            if timer.sound {
                flags.push("sound");
            }
            if timer.recurring {
                flags.push("recurring");
            }
            print!("{}", flags.join(", "));
            print!("]");
        }
        println!();
    }

    Ok(())
}

fn clear_timers() -> Result<(), Box<dyn std::error::Error>> {
    let count = Database::with_transaction(|db| {
        let count = db.timers.len();
        db.clear_all();
        Ok(count)
    })?;

    println!("Cleared {} timer(s)", count);

    Ok(())
}

fn clear_history() -> Result<(), Box<dyn std::error::Error>> {
    let count = Database::with_transaction(|db| {
        let count = db.history.len();
        db.clear_history();
        Ok(count)
    })?;

    println!("Cleared {} completed timer(s) from history", count);

    Ok(())
}

fn show_status() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::load()?;
    let timer_count = db.timers.len();

    if daemon::is_daemon_running()? {
        println!("Daemon is running");
        println!("Active timers: {}", timer_count);
    } else {
        println!("Daemon is not running");
        if timer_count > 0 {
            println!("Active timers: {} (restarting daemon...)", timer_count);
            daemon::ensure_daemon_running()?;
            println!("Daemon restarted");
        } else {
            println!("Active timers: 0");
        }
    }

    Ok(())
}

fn start_daemon() -> Result<(), Box<dyn std::error::Error>> {
    daemon::start_daemon_process()?;
    println!("Daemon started");
    Ok(())
}
