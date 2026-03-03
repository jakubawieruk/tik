# pomitik

A command-line countdown timer with sessions, presets, and a minimal terminal UI. Works on macOS and Windows.

<p align="center">
  <img src="pomotik.png" alt="pomitik terminal UI" width="600">
</p>

## Install

### Homebrew (macOS)

```bash
brew tap jakubawieruk/pomitik
brew install pomitik
```

### Scoop (Windows)

```powershell
scoop bucket add pomitik https://github.com/jakubawieruk/scoop-pomitik
scoop install pomitik
```

### From source

```bash
cargo install --path .
```

## Usage

```bash
tik 25m              # 25 minute timer
tik 1h30m            # 1 hour 30 minutes
tik 90s              # 90 seconds

tik pomodoro         # Full session: 4x (25m work + 5m break), ends with 15m long break
tik break            # Single 5m break timer
tik long-break       # Single 15m timer

tik --silent 25m     # Suppress notification sound
tik --title "Deep Work" pomodoro  # Display a custom title in the timer

tik log              # Show today's and this week's session summary
tik config show      # Show current configuration
tik config set work 30m   # Set work duration to 30 minutes
tik config set rounds 6   # Set number of rounds to 6

tik todo add "Write docs"     # Add a task
tik todo list                 # List all tasks
tik todo list --json          # List as JSON (for scripting/bots)
tik todo done 1               # Mark task #1 as done
tik todo undone 1             # Mark task #1 as not done
tik todo move 2 1             # Move task #2 to position 1
tik todo edit 1 "New text"    # Edit task text
tik todo remove 1             # Delete a task
tik todo clear                # Remove all completed tasks
```

## Controls

- **Space** — pause / resume
- **s** — skip to next phase (disabled on last round)
- **a** / **d** — add / remove a round (during sessions)
- **x** — stop session early
- **Tab** — switch focus between timer and todo sidebar
- **Ctrl+C** — quit

When the todo sidebar has focus:

- **↑ / ↓** — navigate tasks
- **Enter** — toggle done / undone
- **Shift+↑ / Shift+↓** — reorder tasks
- **Tab** — switch back to timer

## Config

View or change settings from the command line:

```bash
tik config show               # Show current values
tik config set work 30m       # Set work duration
tik config set break 10m      # Set break duration
tik config set long-break 20m # Set long break duration
tik config set rounds 6       # Set number of rounds
```

Settings are stored in `~/.config/pomitik/config.toml`. You can also edit this file directly:

```toml
[presets]
pomodoro = "25m"
break = "5m"
long-break = "15m"

[sessions.pomodoro]
work = "pomodoro"
break = "break"
long_break = "long-break"
rounds = 4
```

Built-in defaults (pomodoro: 25m, break: 5m, long-break: 15m, 4 rounds) work without a config file.

## Todo List

Manage a task queue that appears as a sidebar during timer sessions. The top pending task is shown as the "current task" above the timer.

Tasks are stored in `~/.local/share/pomitik/todos.json` and persist across sessions. The `--json` flag on `tik todo list` makes it easy for scripts and bots to manage your task queue.

The sidebar appears automatically when you have pending tasks. On narrow terminals (<60 columns) it falls back to the standard centered layout.

## Session Log

Completed timers are logged to `~/.local/share/pomitik/log.json`. View a summary with:

```bash
tik log
```

## Known Limitations

- **macOS notifications appear under Finder** in System Settings > Notifications. This is because CLI tools don't have their own app bundle, so macOS attributes notifications to the parent process. To receive notifications, enable notifications for Finder. Packaging as a `.app` bundle would resolve this but is not currently implemented.
