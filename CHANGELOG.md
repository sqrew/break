# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Number word parsing support (0-60)
  - Words like `one`, `five`, `twenty`, `fortyfive` now work in place of digits
  - Fully case-insensitive: `One Minute`, `FIVE SECONDS`, `twenty minutes` all work
  - Can be mixed with other formats: `one hour 30m five seconds reminder`
  - 7 new tests covering basic, teens, tens, compounds, mixed usage

### Changed
- **Cross-platform daemon process checking** using sysinfo crate
  - Windows: Now properly detects if daemon is running (previously just checked for PID file)
  - macOS/Linux: Continues to work correctly
  - Fixes stale PID file issues on all platforms
- Removed platform-specific `ps` command usage in favor of cross-platform solution

## [0.1.0] - 2025-01-24

### Added

#### Core Features
- Natural language duration parsing with flexible formats
  - Standard units: `5m`, `1h`, `30s`, `5minutes`, `1hour`, `30seconds`
  - Colon format: `5:30` (m:s), `1:30:45` (h:m:s)
  - Mixed formats: `1h 30m 2:15 take a break`
  - Case-insensitive parsing
  - Support for unicode and special characters in messages

#### Timer Flags
- `--urgent` / `-u`: Mark notifications as critical/urgent priority
- `--sound` / `-s`: Play notification sounds
- `--recurring` / `-r`: Automatically repeat timer after completion
- Combined short flags support (e.g., `-usr` instead of `-u -s -r`)
- Flags can appear anywhere in command input

#### Commands
- `break <duration> <message>`: Create a timer
- `break list` / `l`: List active timers with time remaining
- `break history` / `h`: Show last 20 completed timers
- `break remove <id>` / `rm`: Remove a specific timer
- `break clear` / `c`: Clear all active timers
- `break clear-history` / `ch`: Clear timer history
- `break status` / `s`: Check daemon status
- `break daemon` / `d`: Manually start daemon
- Progressive alias matching for all commands (e.g., `l`, `li`, `lis` for `list`)

#### Storage & Persistence
- JSON-based database at `~/.local/share/break/timers.json`
- File locking (fs2) to prevent corruption from concurrent access
- Transaction-based updates for atomic database modifications
- History tracking (last 20 completed timers)
- Maximum duration validation (1 year limit)

#### Daemon
- Background process for monitoring timers
- Dynamic sleep intervals based on next timer due time
- Auto-recovery: daemon restarts when running any command if timers exist
- Automatic exit when no active timers remain
- PID file management at `~/.local/share/break/daemon.pid`

#### Notifications
- Desktop notifications via notify-rust
- Timer message shown as notification title for visibility
- Urgency levels (normal/critical)
- Optional sound alerts
- Recurring timers add to history on each completion

#### Testing
- 35 comprehensive tests covering:
  - Parser: 20 tests for all duration formats and edge cases
  - Database: 12 tests for CRUD operations, history, and validation
  - Daemon: 3 tests for basic functionality
- All tests passing

#### Documentation
- Comprehensive README.md with:
  - Feature overview
  - Installation instructions
  - Usage examples
  - Troubleshooting guide
- Module-level documentation for all source files
- Function-level documentation for all public APIs with examples
- MIT License

### Technical Details
- Written in Rust with minimal dependencies
- Uses clap for CLI parsing (derive API)
- Time handling with time crate and OffsetDateTime
- UUID generation for unique timer identification
- Sequential numeric IDs for user-friendly reference

[0.1.0]: https://github.com/sqrew/break/releases/tag/v0.1.0
