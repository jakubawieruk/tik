# Changelog

All notable changes to pomitik (`tik`) will be documented in this file.

## [0.3.1] - 2026-03-03

### Fixed

- Updated usage message to list all available subcommands (log, config, todo)
- Updated README with todo feature documentation and controls

## [0.3.0] - 2026-03-03

### Added

- **Todo list feature** — manage a task queue alongside your timer
  - CLI commands: `tik todo add`, `list`, `done`, `undone`, `remove`, `move`, `edit`, `clear`
  - `--json` flag on `tik todo list` for machine-readable output
  - Sidebar display in timer UI when pending tasks exist
  - Tab to switch focus between timer and todo panel
  - Arrow keys to navigate, Enter to toggle done/undone, Shift+arrows to reorder
  - Auto-save on every change during timer
  - Falls back to centered layout on narrow terminals (<60 cols)
- Todo data stored as JSON at `~/.local/share/pomitik/todos.json` (macOS: `~/Library/Application Support/pomitik/todos.json`)

## [0.2.0] - 2026-02-26

### Added

- `tik config show` and `tik config set` subcommands for managing presets and rounds
- `--title` flag to display a custom label in the timer UI
- Dynamic round adjustment during sessions with `a` (add) and `d` (remove) keys
- Skip (`s`) and stop-early (`x`) keyboard controls
- Title, round info, and hint bar in timer UI
- Smooth skip transitions between session phases (stays in alternate screen)
- Disable skip on last round to prevent accidental session exit
- Scoop bucket for Windows installation
- CI auto-updates Homebrew tap and Scoop bucket on release

### Fixed

- Title rendering now uses white foreground color

## [0.1.0] - 2026-02-25

### Added

- Countdown timer with flexible duration parsing (`25m`, `1h30m`, `90s`)
- Built-in presets: `pomodoro` (25m), `break` (5m), `long-break` (15m)
- Pomodoro session mode with work/break cycles (`tik pomodoro`)
- Custom sessions and presets via TOML config (`~/.config/pomitik/config.toml`)
- Session log with daily/weekly summaries (`tik log`)
- Terminal UI with colored progress bar (green/yellow/red transitions)
- Pause/resume with spacebar
- Desktop notifications on timer completion (macOS and Windows)
- Homebrew tap for macOS installation
- CI release pipeline for macOS (arm64, x86_64) and Windows (x86_64)
