use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use std::io;
use std::process;

mod daemon;
mod database;
mod parser;

use database::Database;

// Time constants to avoid magic numbers
const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE; // 3600

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
    #[command(aliases = ["l", "li", "lis", "sh", "sho", "show", "dis", "display"])]
    List,
    /// Show recently completed timers
    #[command(aliases = ["h", "hi", "his", "hist", "histo", "histor"])]
    History,
    /// Remove a timer by ID
    #[command(aliases = ["r", "rm", "rem", "remo", "remov", "del", "dele", "delet", "delete"])]
    Remove { id: u32 },
    /// Clear all timers
    #[command(aliases = ["c", "cl", "cle", "clea"])]
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
    /// Generate shell completions (bash, zsh, fish, powershell)
    #[command(hide = true)]
    Completions { shell: Shell },
}

/// Formats seconds into a human-readable duration string.
///
/// Shows hours and minutes for all durations, and includes seconds only if the
/// total duration is less than the specified threshold.
///
/// # Arguments
///
/// * `seconds` - Total number of seconds to format
/// * `show_seconds_threshold_mins` - Only show seconds if duration < this many minutes
///
/// # Returns
///
/// A formatted string like "5h 30m 15s" or "2m 45s"
///
/// # Examples
///
/// ```ignore
/// assert_eq!(format_duration(3665, 5), "1h 1m 5s");  // < 5 mins from hours, shows seconds
/// assert_eq!(format_duration(360, 5), "6m");          // >= 5 mins, no seconds
/// assert_eq!(format_duration(45, 5), "0m 45s");       // < 5 mins, shows seconds
/// ```
fn format_duration(seconds: i64, show_seconds_threshold_mins: i64) -> String {
    let hours = seconds / SECONDS_PER_HOUR;
    let minutes = (seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let secs = seconds % SECONDS_PER_MINUTE;

    let mut parts = Vec::new();

    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 || hours > 0 {
        parts.push(format!("{}m", minutes));
    }
    if hours == 0 && minutes < show_seconds_threshold_mins {
        parts.push(format!("{}s", secs));
    }

    parts.join(" ")
}

/// Formats timer flags for display.
///
/// Returns a string containing the flags in brackets if any are set,
/// or an empty string if no flags are active.
///
/// # Arguments
///
/// * `timer` - The timer whose flags should be formatted
///
/// # Returns
///
/// A formatted string like " [urgent, sound]" or "" if no flags are set
///
/// # Examples
///
/// ```ignore
/// let timer = Timer { urgent: true, sound: false, recurring: false, ... };
/// assert_eq!(format_flags(&timer), " [urgent]");
/// ```
fn format_flags(timer: &database::Timer) -> String {
    if !timer.urgent && !timer.sound && !timer.recurring {
        return String::new();
    }

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

    format!(" [{}]", flags.join(", "))
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
        Some(Commands::Completions { shell }) => {
            generate_completions(shell);
            return;
        }
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

/// Extracts flag arguments from mixed input and returns cleaned input plus flag states.
///
/// This function allows users to place flags anywhere in their input, including at the end.
/// It recognizes both long form (`--urgent`) and short form (`-u`) flags, and supports
/// combined short flags like `-usr` for `-u -s -r`.
///
/// # Arguments
///
/// * `input` - Slice of input strings that may contain flags mixed with duration/message
///
/// # Returns
///
/// Returns a tuple of:
/// - `String` - The cleaned input with all flags removed, joined with spaces
/// - `bool` - Whether `--urgent` or `-u` was found
/// - `bool` - Whether `--sound` or `-s` was found
/// - `bool` - Whether `--recurring` or `-r` was found
///
/// # Examples
///
/// ```ignore
/// let (clean, u, s, r) = extract_flags_from_input(&["5m", "coffee", "--urgent"]);
/// assert_eq!(clean, "5m coffee");
/// assert!(u); // urgent flag found
/// ```
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

/// Creates a new timer from user input with specified flags.
///
/// Parses the input string to extract duration and message, creates a timer in the
/// database using a transaction for atomicity, displays confirmation to the user,
/// and ensures the daemon is running to monitor the timer.
///
/// # Arguments
///
/// * `input` - The input string containing duration and message (e.g., "5m get coffee")
/// * `urgent` - Whether to mark the notification as urgent/critical
/// * `sound` - Whether to play a sound when the notification fires
/// * `recurring` - Whether the timer should automatically repeat after completion
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if parsing fails, timer creation fails,
/// or the daemon cannot be started.
///
/// # Examples
///
/// ```ignore
/// add_timer("5m coffee break", true, false, false)?; // Urgent 5-minute timer
/// add_timer("1h meeting", false, true, true)?;       // Recurring hourly timer with sound
/// ```
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

    println!(
        "Timer #{} set for \"{}\" ({} seconds){}",
        timer.id,
        message,
        duration_seconds,
        format_flags(&timer)
    );

    // Show relative time (e.g., "in 5 minutes")
    let now = time::OffsetDateTime::now_utc();
    let duration_until = timer.due_at - now;
    let seconds = duration_until.whole_seconds();

    if seconds > 0 {
        println!("Break will notify you in {}", format_duration(seconds, 5));
    } else {
        println!("Break notification is ready!");
    }

    // Ensure daemon is running
    daemon::ensure_daemon_running()?;

    Ok(())
}

/// Lists all active timers with their remaining time and flags.
///
/// Loads the timer database, displays each active timer with formatted time remaining,
/// marks expired timers as "EXPIRED", shows any flags (urgent/sound/recurring), and
/// ensures the daemon is running if there are active timers.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the database cannot be loaded or
/// the daemon cannot be started.
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
            println!(
                "  #{}: \"{}\" - {} remaining{}",
                timer.id,
                timer.message,
                format_duration(remaining_secs, i64::MAX), // Always show seconds for active timers
                format_flags(timer)
            );
        } else {
            println!(
                "  #{}: \"{}\" - EXPIRED{}",
                timer.id,
                timer.message,
                format_flags(timer)
            );
        }
    }

    Ok(())
}

/// Removes a timer by its ID.
///
/// Uses a database transaction to atomically remove the specified timer.
/// The timer is removed without adding it to history (unlike timer completion).
///
/// # Arguments
///
/// * `id` - The numeric ID of the timer to remove
///
/// # Returns
///
/// Returns `Ok(())` on success (whether or not the timer was found), or an error
/// if the database transaction fails.
fn remove_timer(id: u32) -> Result<(), Box<dyn std::error::Error>> {
    let timer_opt = Database::with_transaction(|db| Ok(db.remove_timer(id)))?;

    if let Some(timer) = timer_opt {
        println!("Removed timer #{}: \"{}\"", timer.id, timer.message);
    } else {
        println!("Timer #{} not found", id);
    }

    Ok(())
}

/// Displays the history of recently completed timers.
///
/// Shows the last 20 completed timers (most recent first) with information about
/// when they were completed and their flags. This allows users to see timers they
/// may have missed if notifications were disabled.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the database cannot be loaded.
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

        let time_ago = if elapsed_secs < SECONDS_PER_MINUTE {
            "< 1m".to_string()
        } else {
            format_duration(elapsed_secs, i64::MAX)
        };

        println!(
            "  #{}: \"{}\" - completed {} ago{}",
            timer.id,
            timer.message,
            time_ago,
            format_flags(timer)
        );
    }

    Ok(())
}

/// Clears all active timers from the database.
///
/// Uses a database transaction to atomically remove all timers. Timers are not
/// added to history. Displays the count of cleared timers.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the database transaction fails.
fn clear_timers() -> Result<(), Box<dyn std::error::Error>> {
    let count = Database::with_transaction(|db| {
        let count = db.timers.len();
        db.clear_all();
        Ok(count)
    })?;

    println!("Cleared {} timer(s)", count);

    Ok(())
}

/// Clears the history of completed timers.
///
/// Uses a database transaction to atomically remove all entries from the history.
/// Displays the count of cleared history entries. Does not affect active timers.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the database transaction fails.
fn clear_history() -> Result<(), Box<dyn std::error::Error>> {
    let count = Database::with_transaction(|db| {
        let count = db.history.len();
        db.clear_history();
        Ok(count)
    })?;

    println!("Cleared {} completed timer(s) from history", count);

    Ok(())
}

/// Shows the status of the daemon and active timers.
///
/// Checks if the daemon is running and displays the count of active timers.
/// If the daemon is not running but there are active timers, automatically
/// restarts the daemon to ensure timers are monitored.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the database cannot be loaded
/// or the daemon cannot be started.
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

/// Manually starts the daemon process.
///
/// Spawns a new daemon process to monitor timers. This is typically called
/// automatically when timers are created, but can be manually invoked if needed.
/// If the daemon is already running, this has no effect.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the daemon process cannot be spawned.
fn start_daemon() -> Result<(), Box<dyn std::error::Error>> {
    daemon::start_daemon_process()?;
    println!("Daemon started");
    Ok(())
}

/// Generates shell completion scripts for the specified shell.
///
/// This function outputs the completion script to stdout, which can be saved
/// or sourced directly. Supports bash, zsh, fish, and PowerShell.
///
/// # Arguments
///
/// * `shell` - The shell type to generate completions for
///
/// # Examples
///
/// ```bash
/// # Generate and install bash completions
/// breakrs completions bash > ~/.local/share/bash-completion/completions/breakrs
///
/// # Generate and install zsh completions
/// breakrs completions zsh > ~/.zsh/completion/_breakrs
///
/// # Generate and install fish completions
/// breakrs completions fish > ~/.config/fish/completions/breakrs.fish
/// ```
fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    generate(shell, &mut cmd, bin_name, &mut io::stdout());
}
