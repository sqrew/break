use clap::{Parser, Subcommand};
use std::process;

mod daemon;
mod database;
mod parser;

use database::Database;

#[derive(Parser)]
#[command(name = "break")]
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
            let (input_cleaned, urgent_flag, sound_flag, recurring_flag) = extract_flags_from_input(&cli.input);

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

    let mut db = Database::load()?;
    let timer = db.add_timer(message.clone(), duration_seconds, urgent, sound, recurring);
    db.save()?;

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
    println!(
        "Break will notify you at {}",
        timer
            .due_at
            .format(&time::format_description::well_known::Rfc3339)?
    );

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
