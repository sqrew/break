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
  - Common spelling variations supported: `fourty`, hyphenated forms like `twenty-one`
  - 7 new tests covering basic, teens, tens, compounds, mixed usage
- **Comprehensive documentation improvements**
  - Added detailed doc comments to all parser helper functions (`tokenize`, `parse_unit`)
  - Added doc comments to all 9 functions in main.rs with Arguments, Returns, and Examples sections
  - Improved code maintainability and developer onboarding experience
- **Database validation and limits**
  - Automatic validation and cleanup of invalid timers on database load
  - Filters out timers with empty messages, corrupted timestamps, or invalid durations
  - Maximum limit of 100 active timers to prevent resource exhaustion
  - Clear error messages when limits are exceeded
  - 8 new comprehensive tests for validation logic and timer limits
- **CI/CD Pipeline with GitHub Actions**
  - Automated testing on Linux, macOS, and Windows for every push/PR
  - Clippy linting and rustfmt formatting checks enforced in CI
  - Security audit via cargo-audit for dependency vulnerabilities
  - Automated release workflow that builds binaries for 5 platforms
  - Creates GitHub releases with pre-built binaries on version tags
- **CONTRIBUTING.md** - Comprehensive contribution guidelines
  - Development workflow and testing procedures
  - Code style and architectural guidelines
  - Clear explanation of project philosophy (simplicity first)
- **Shell completions** - Auto-complete support for commands and flags
  - Generate completions for bash, zsh, fish, and PowerShell
  - Tab completion for commands: `breakrs l<TAB>` → `list`
  - Tab completion for flags: `breakrs --u<TAB>` → `--urgent`
  - Easy installation with `breakrs completions <shell>` command
  - Improves discoverability and reduces typing
  - README updated with installation instructions for all shells

### Changed
- **Cross-platform daemon process checking** using sysinfo crate
  - Windows: Now properly detects if daemon is running (previously just checked for PID file)
  - macOS/Linux: Continues to work correctly
  - Fixes stale PID file issues on all platforms
- Removed platform-specific `ps` command usage in favor of cross-platform solution
- **Code deduplication and refactoring**
  - Added `format_duration()` helper function to centralize time formatting logic
  - Added `format_flags()` helper function to centralize flag display logic
  - Refactored `add_timer()`, `list_timers()`, and `show_history()` to use helpers
  - Eliminated ~60 lines of duplicated code, improving maintainability
- **Replaced magic numbers with named constants**
  - Added time constants (`SECONDS_PER_MINUTE`, `SECONDS_PER_HOUR`, `SECONDS_PER_DAY`, `SECONDS_PER_YEAR`)
  - Replaced hardcoded values (60, 3600, 86400) throughout codebase
  - Improved code readability and maintainability across all modules
  - Makes time calculations self-documenting and easier to understand
- **Binary size optimization** - Reduced from 4.7MB to 1.6MB (66% reduction)
  - Added optimized release profile with LTO and size optimizations
  - Stripped debug symbols and enabled panic=abort
  - Significantly faster downloads and less disk space usage
- **Enhanced Cargo.toml metadata**
  - Added README, homepage, and documentation links for better crates.io presentation
  - Excluded unnecessary files from published package (.github/, .claude/, target/)
  - Improved package metadata for better discoverability

### Fixed
- **Notification error handling** - Daemon now properly handles notification failures
  - Added automatic retry logic (500ms delay, one retry attempt)
  - Clear error messages printed to stderr when notifications fail
  - Helpful troubleshooting suggestions for users
- **Cross-platform notification API compatibility**
  - Fixed compilation errors on Windows and macOS
  - Platform-specific notification code: full features on Linux, basic on macOS/Windows
  - Urgency levels supported on Linux only (gracefully ignored on other platforms)
  - Sound support properly handled per platform
  - Documentation updated to explain platform differences
- **Parser code cleanup** - Fixed Clippy warnings for cleaner, more idiomatic code

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
- `breakrs <duration> <message>`: Create a timer
- `breakrs list` / `l`: List active timers with time remaining
- `breakrs history` / `h`: Show last 20 completed timers
- `breakrs remove <id>` / `rm`: Remove a specific timer
- `breakrs clear` / `c`: Clear all active timers
- `breakrs clear-history` / `ch`: Clear timer history
- `breakrs status` / `s`: Check daemon status
- `breakrs daemon` / `d`: Manually start daemon
- Progressive alias matching for all commands (e.g., `l`, `li`, `lis` for `list`)

#### Storage & Persistence
- JSON-based database at `~/.local/share/breakrs/timers.json`
- File locking (fs2) to prevent corruption from concurrent access
- Transaction-based updates for atomic database modifications
- History tracking (last 20 completed timers)
- Maximum duration validation (1 year limit)

#### Daemon
- Background process for monitoring timers
- Dynamic sleep intervals based on next timer due time
- Auto-recovery: daemon restarts when running any command if timers exist
- Automatic exit when no active timers remain
- PID file management at `~/.local/share/breakrs/daemon.pid`

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

[0.1.0]: https://github.com/sqrew/breakrs/releases/tag/v0.1.0
