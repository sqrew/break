# break

A simple, ergonomic CLI timer for taking breaks. Set natural language reminders with flexible syntax, get desktop notifications, and never miss a break again.

Why break? Because typing "break 15m stretch" or "break go outside one hour 30mins" and having everything handled automatically is the quick and easy standard we should have.
No need for watches, phones, calendars, sticky notes, guis, bloated apps, etc. No syntax, no manual conversions and no handling. No configs or setup. No learning curve.
Type how you think and a notification pops up when you need it to. That's why.

## Features

- **Natural time parsing**: `5m`, `1h30m`, `1:30:45`, `one minute`, or mix them all: `1h 2:30 five seconds reminder`
- **Number word support**: Type `five minutes`, `twenty seconds`, `one hour` - fully case-insensitive
- **Flexible flag placement**: Put flags anywhere - `break 5m coffee --urgent` or `break --urgent 5m coffee`
- **Combined short flags**: Use `-usr` instead of `-u -s -r`
- **Recurring timers**: Automatically repeat with `--recurring`
- **Priority notifications**: Mark important breaks as `--urgent`
- **Sound alerts**: Add `--sound` to play notification sounds
- **History tracking**: See your last 20 completed timers
- **Command aliases**: Type `break l` instead of `break list`
- **Auto-recovery**: Daemon automatically restarts after reboot
- **File locking**: Safe concurrent access, no database corruption

## Installation

### From Source

```bash
git clone <https://github.com/sqrew/break>
cd break
cargo build --release
sudo cp target/release/break /usr/local/bin/
```

### From crates.io

```
cargo install break
```

### Platform Support

**Fully supported on:**
- Linux (all distros)
- macOS (10.8+)
- Windows (10+)

### Dependencies

- Rust 1.70+ (for building)
- Linux/macOS: notification daemon (most systems have this by default)
- Windows: native notification system (built into Windows 10+)

## Usage

### Basic Timer

```bash
# Simple format
break 5m Get coffee
break 1h Meeting reminder
break 30s Quick stretch

# Colon format (h:m:s or m:s)
break 1:30 Tea is ready
break 1:30:45 Long break over
break 0:30 Quick reminder

# Mixed formats
break 1h 30m 2:15 Combined duration message

# Number words (case-insensitive)
break one minute thirty seconds reminder
break Five Minutes Get Coffee
break two hours five minutes lunch break
```

### Flags

```bash
# Urgent/critical notification
break --urgent 5m Important meeting
break 5m Important meeting --urgent  # Flags work anywhere!

# Play sound
break --sound 10m Timer with sound

# Recurring timer (repeats after completion)
break --recurring 1h Stretch every hour
break -r 1h Stretch every hour  # Short form

# Combine flags
break --urgent --sound --recurring 30m Drink water
break -usr 30m Drink water  # Combined short flags
```

### Commands

```bash
# List active timers
break list
break l        # Short alias
break li       # Partial alias

# Show recently completed timers (last 20)
break history
break h        # Short alias

# Remove a specific timer by ID
break remove 5
break rm 5     # Short alias

# Clear all active timers
break clear
break c        # Short alias

# Clear history
break clear-history
break ch       # Short alias

# Check daemon status
break status
break s        # Short alias

# Manually start daemon
break daemon
break d        # Short alias
```

### Examples

```bash
# Set a 5-minute coffee break reminder
break 5m Get coffee

# Set an urgent 10-minute meeting reminder with sound
break 10m Meeting in conference room -us

# Set a recurring hourly stretch reminder
break -r 1h Stand up and stretch

# Create multiple timers
break 5m First reminder
break 10m Second reminder
break 15m Third reminder

# List active timers
break l

# Check history of completed timers
break h

# Remove a specific timer
break r 2

# Clear all timers
break c
```

## How It Works

1. **Parser**: Extracts duration and message from natural language input
   - Supports units: `s`, `sec`, `m`, `min`, `h`, `hr`, `hours`, etc.
   - Supports number words: `one`, `five`, `twenty`, `fortyfive` (0-60)
   - Supports colon format: `5:30` (5 min 30 sec), `1:30:45` (1 hr 30 min 45 sec)
   - Flags can appear anywhere in the input

2. **Database**: Stores active and completed timers in JSON
   - Location: `~/.local/share/break/timers.json`
   - File locking prevents corruption from concurrent access
   - Keeps last 20 completed timers in history

3. **Daemon**: Background process that monitors timers
   - Automatically starts when you create a timer
   - Sleeps until next timer expires (efficient)
   - Auto-restarts when you run any command (survives reboots)
   - Exits when no active timers remain

4. **Notifications**: Desktop notifications via `notify-rust`
   - Title shows your message for quick visibility
   - Supports urgency levels (normal/critical)
   - Optional sound alerts
   - Recurring timers add to history on each completion

## Duration Formats

All of these work and can be mixed:

```bash
# Standard units
5m, 1h, 30s
5 minutes, 1 hour, 30 seconds
1h30m, 2h15m30s

# Number words (case-insensitive, 0-60)
one minute, five seconds, twenty minutes
two hours, fifteen minutes, fortyfive seconds

# Colon format
5:30        # 5 minutes 30 seconds
1:30:45     # 1 hour 30 minutes 45 seconds

# Mixed (combine any formats!)
1h 2:30 five seconds break    # 1 hour + 2m 30s + 5s = 3755 seconds
one hour 30m reminder          # Mix number words with standard units
```

## Command Aliases

Every command supports progressive prefix matching:

- `list`: `l`, `li`, `lis`
- `history`: `h`, `hi`, `his`, `hist`
- `remove`: `r`, `rm`, `rem`
- `clear`: `c`, `cl`, `cle`
- `clear-history`: `ch`, `clh`, `clear-h`
- `status`: `s`, `st`, `sta`, `stat`, `stats`
- `daemon`: `d`, `da`, `dae`

## Troubleshooting

### Notifications not appearing

Check if your notification daemon is running:
```bash
ps aux | grep notification
```

### Database corrupted

If you see a corruption error, the message tells you how to fix it:
```bash
rm ~/.local/share/break/timers.json
```

### Daemon not running after reboot

Any command will auto-restart the daemon if there are active timers:
```bash
break list   # Will restart daemon if needed
break status # Explicitly checks and restarts
```

## License

[MIT - see LICENSE file]

## Contributing

Contributions welcome! This tool is trying to follow the Unix philosophy: do one thing and do it well.
