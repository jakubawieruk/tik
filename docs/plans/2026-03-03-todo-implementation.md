# Todo Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a task queue (todo list) managed via CLI and displayed as a sidebar during timer sessions.

**Architecture:** New `src/todo.rs` module for data model and persistence (JSON array in `~/.local/share/pomitik/todos.json`). CLI subcommands in `main.rs`. Timer and renderer extended with optional todo state passed via `Arc<Mutex<TodoList>>` and watch channels for focus-mode switching.

**Tech Stack:** Existing — serde/serde_json, clap, crossterm, chrono, dirs. New — `uuid` crate for stable IDs (actually we'll use auto-incrementing u32).

---

### Task 1: Todo data model and persistence

**Files:**
- Create: `src/todo.rs`

**Step 1: Write failing tests for Todo serialization and TodoList load/save**

```rust
// src/todo.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_todo() {
        let todo = Todo {
            id: 1,
            text: "Test task".to_string(),
            done: false,
            created_at: chrono::Local::now(),
            completed_at: None,
        };
        let json = serde_json::to_string(&todo).unwrap();
        assert!(json.contains("Test task"));
        assert!(json.contains("\"id\":1"));
    }

    #[test]
    fn deserialize_todo() {
        let json = r#"{"id":1,"text":"Test task","done":false,"created_at":"2026-03-03T10:00:00+01:00","completed_at":null}"#;
        let todo: Todo = serde_json::from_str(json).unwrap();
        assert_eq!(todo.id, 1);
        assert_eq!(todo.text, "Test task");
        assert!(!todo.done);
    }

    #[test]
    fn roundtrip_todo_list() {
        let list = TodoList {
            next_id: 3,
            items: vec![
                Todo {
                    id: 1,
                    text: "First".to_string(),
                    done: false,
                    created_at: chrono::Local::now(),
                    completed_at: None,
                },
                Todo {
                    id: 2,
                    text: "Second".to_string(),
                    done: true,
                    created_at: chrono::Local::now(),
                    completed_at: Some(chrono::Local::now()),
                },
            ],
        };
        let json = serde_json::to_string_pretty(&list).unwrap();
        let parsed: TodoList = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.next_id, 3);
        assert_eq!(parsed.items.len(), 2);
        assert_eq!(parsed.items[0].text, "First");
        assert!(parsed.items[1].done);
    }

    #[test]
    fn todo_path_ends_with_expected() {
        let path = todo_path();
        assert!(path.ends_with("pomitik/todos.json"));
    }

    #[test]
    fn load_empty_returns_default() {
        // When file doesn't exist, load returns empty list
        let list = TodoList::new();
        assert_eq!(list.next_id, 1);
        assert!(list.items.is_empty());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib todo::tests -- --nocapture`
Expected: Compilation error — `todo` module not found

**Step 3: Write the data model and persistence**

```rust
// src/todo.rs
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Todo {
    pub id: u32,
    pub text: String,
    pub done: bool,
    pub created_at: DateTime<Local>,
    pub completed_at: Option<DateTime<Local>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoList {
    pub next_id: u32,
    pub items: Vec<Todo>,
}

pub fn todo_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("pomitik")
        .join("todos.json")
}

impl TodoList {
    pub fn new() -> Self {
        TodoList {
            next_id: 1,
            items: Vec::new(),
        }
    }

    pub fn load() -> Self {
        let path = todo_path();
        if !path.exists() {
            return Self::new();
        }
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = todo_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data dir: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize todos: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("Failed to write todos: {e}"))?;
        Ok(())
    }
}
```

Also add `mod todo;` to `src/main.rs` (line 1 area, alongside other module declarations).

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib todo::tests -- --nocapture`
Expected: All 5 tests PASS

**Step 5: Commit**

```bash
git add src/todo.rs src/main.rs
git commit -m "feat(todo): add data model and persistence"
```

---

### Task 2: TodoList CRUD operations

**Files:**
- Modify: `src/todo.rs`

**Step 1: Write failing tests for CRUD operations**

Add these tests to the existing `mod tests` block in `src/todo.rs`:

```rust
    #[test]
    fn add_todo() {
        let mut list = TodoList::new();
        list.add("First task".to_string());
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].id, 1);
        assert_eq!(list.items[0].text, "First task");
        assert!(!list.items[0].done);
        assert_eq!(list.next_id, 2);
    }

    #[test]
    fn add_multiple_todos() {
        let mut list = TodoList::new();
        list.add("First".to_string());
        list.add("Second".to_string());
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].id, 1);
        assert_eq!(list.items[1].id, 2);
        assert_eq!(list.next_id, 3);
    }

    #[test]
    fn mark_done() {
        let mut list = TodoList::new();
        list.add("Task".to_string());
        assert!(list.mark_done(1).is_ok());
        assert!(list.items[0].done);
        assert!(list.items[0].completed_at.is_some());
    }

    #[test]
    fn mark_done_invalid_id() {
        let mut list = TodoList::new();
        assert!(list.mark_done(99).is_err());
    }

    #[test]
    fn remove_todo() {
        let mut list = TodoList::new();
        list.add("Task".to_string());
        assert!(list.remove(1).is_ok());
        assert!(list.items.is_empty());
    }

    #[test]
    fn remove_invalid_id() {
        let mut list = TodoList::new();
        assert!(list.remove(99).is_err());
    }

    #[test]
    fn move_todo() {
        let mut list = TodoList::new();
        list.add("First".to_string());
        list.add("Second".to_string());
        list.add("Third".to_string());
        // Move "Third" (id=3) to position 1 (0-indexed internally, 1-based in CLI)
        assert!(list.move_to(3, 1).is_ok());
        assert_eq!(list.items[0].text, "Third");
        assert_eq!(list.items[1].text, "First");
        assert_eq!(list.items[2].text, "Second");
    }

    #[test]
    fn move_todo_invalid_id() {
        let mut list = TodoList::new();
        assert!(list.move_to(99, 1).is_err());
    }

    #[test]
    fn move_todo_out_of_bounds_clamps() {
        let mut list = TodoList::new();
        list.add("First".to_string());
        list.add("Second".to_string());
        // Position 99 should clamp to end
        assert!(list.move_to(1, 99).is_ok());
        assert_eq!(list.items[1].text, "First");
    }

    #[test]
    fn edit_todo() {
        let mut list = TodoList::new();
        list.add("Old text".to_string());
        assert!(list.edit(1, "New text".to_string()).is_ok());
        assert_eq!(list.items[0].text, "New text");
    }

    #[test]
    fn edit_invalid_id() {
        let mut list = TodoList::new();
        assert!(list.edit(99, "text".to_string()).is_err());
    }

    #[test]
    fn clear_completed() {
        let mut list = TodoList::new();
        list.add("Done task".to_string());
        list.add("Pending task".to_string());
        let _ = list.mark_done(1);
        let cleared = list.clear_completed();
        assert_eq!(cleared, 1);
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].text, "Pending task");
    }

    #[test]
    fn current_task_returns_first_pending() {
        let mut list = TodoList::new();
        list.add("Done".to_string());
        list.add("Current".to_string());
        list.add("Next".to_string());
        let _ = list.mark_done(1);
        let current = list.current_task();
        assert!(current.is_some());
        assert_eq!(current.unwrap().text, "Current");
    }

    #[test]
    fn current_task_none_when_all_done() {
        let mut list = TodoList::new();
        list.add("Done".to_string());
        let _ = list.mark_done(1);
        assert!(list.current_task().is_none());
    }

    #[test]
    fn has_pending_tasks() {
        let mut list = TodoList::new();
        assert!(!list.has_pending());
        list.add("Task".to_string());
        assert!(list.has_pending());
        let _ = list.mark_done(1);
        assert!(!list.has_pending());
    }

    #[test]
    fn move_up_swaps_with_previous() {
        let mut list = TodoList::new();
        list.add("First".to_string());
        list.add("Second".to_string());
        list.add("Third".to_string());
        assert!(list.move_up(1).is_ok()); // move index 1 up
        assert_eq!(list.items[0].text, "Second");
        assert_eq!(list.items[1].text, "First");
    }

    #[test]
    fn move_up_at_top_is_noop() {
        let mut list = TodoList::new();
        list.add("First".to_string());
        assert!(list.move_up(0).is_ok());
        assert_eq!(list.items[0].text, "First");
    }

    #[test]
    fn move_down_swaps_with_next() {
        let mut list = TodoList::new();
        list.add("First".to_string());
        list.add("Second".to_string());
        list.add("Third".to_string());
        assert!(list.move_down(0).is_ok()); // move index 0 down
        assert_eq!(list.items[0].text, "Second");
        assert_eq!(list.items[1].text, "First");
    }

    #[test]
    fn move_down_at_bottom_is_noop() {
        let mut list = TodoList::new();
        list.add("First".to_string());
        assert!(list.move_down(0).is_ok());
        assert_eq!(list.items[0].text, "First");
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib todo::tests -- --nocapture`
Expected: Compilation errors — methods not found

**Step 3: Implement CRUD methods**

Add to `impl TodoList` in `src/todo.rs`:

```rust
    pub fn add(&mut self, text: String) {
        let todo = Todo {
            id: self.next_id,
            text,
            done: false,
            created_at: Local::now(),
            completed_at: None,
        };
        self.items.push(todo);
        self.next_id += 1;
    }

    pub fn mark_done(&mut self, id: u32) -> Result<(), String> {
        let todo = self.items.iter_mut().find(|t| t.id == id)
            .ok_or_else(|| format!("No todo with id {id}"))?;
        todo.done = true;
        todo.completed_at = Some(Local::now());
        Ok(())
    }

    pub fn remove(&mut self, id: u32) -> Result<(), String> {
        let idx = self.items.iter().position(|t| t.id == id)
            .ok_or_else(|| format!("No todo with id {id}"))?;
        self.items.remove(idx);
        Ok(())
    }

    /// Move task with `id` to `position` (1-based, clamped to valid range).
    pub fn move_to(&mut self, id: u32, position: u32) -> Result<(), String> {
        let from = self.items.iter().position(|t| t.id == id)
            .ok_or_else(|| format!("No todo with id {id}"))?;
        let to = ((position as usize).saturating_sub(1)).min(self.items.len().saturating_sub(1));
        let item = self.items.remove(from);
        self.items.insert(to, item);
        Ok(())
    }

    pub fn edit(&mut self, id: u32, text: String) -> Result<(), String> {
        let todo = self.items.iter_mut().find(|t| t.id == id)
            .ok_or_else(|| format!("No todo with id {id}"))?;
        todo.text = text;
        Ok(())
    }

    /// Remove all completed tasks. Returns count of removed items.
    pub fn clear_completed(&mut self) -> usize {
        let before = self.items.len();
        self.items.retain(|t| !t.done);
        before - self.items.len()
    }

    /// Returns the first pending (not done) task, which is the "current" task.
    pub fn current_task(&self) -> Option<&Todo> {
        self.items.iter().find(|t| !t.done)
    }

    /// Returns true if there are any pending tasks.
    pub fn has_pending(&self) -> bool {
        self.items.iter().any(|t| !t.done)
    }

    /// Move item at `index` one position up (swap with previous). Noop if at top.
    pub fn move_up(&mut self, index: usize) -> Result<(), String> {
        if index >= self.items.len() {
            return Err(format!("Index {index} out of bounds"));
        }
        if index > 0 {
            self.items.swap(index, index - 1);
        }
        Ok(())
    }

    /// Move item at `index` one position down (swap with next). Noop if at bottom.
    pub fn move_down(&mut self, index: usize) -> Result<(), String> {
        if index >= self.items.len() {
            return Err(format!("Index {index} out of bounds"));
        }
        if index + 1 < self.items.len() {
            self.items.swap(index, index + 1);
        }
        Ok(())
    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib todo::tests -- --nocapture`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/todo.rs
git commit -m "feat(todo): add CRUD operations"
```

---

### Task 3: CLI subcommands and list display

**Files:**
- Modify: `src/main.rs`
- Modify: `src/todo.rs` (add `print_list` and `print_list_json`)

**Step 1: Add display methods to todo.rs**

Add to `impl TodoList` in `src/todo.rs`:

```rust
    pub fn print_list(&self) {
        if self.items.is_empty() {
            println!("No tasks.");
            return;
        }
        println!("Tasks:");
        for (i, todo) in self.items.iter().enumerate() {
            let status = if todo.done { "x" } else { " " };
            println!("  {}. [{}] {:<30} (#{})","i + 1, status, todo.text, todo.id);
        }
    }

    pub fn print_list_json(&self) {
        let json = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string());
        println!("{json}");
    }
```

**Step 2: Add Todo subcommand to main.rs**

Add `TodoAction` enum and `Todo` variant to `Commands`:

```rust
// In the Commands enum, add:
    /// Manage todo tasks
    Todo {
        #[command(subcommand)]
        action: Option<TodoAction>,
    },

// New enum:
#[derive(Subcommand)]
enum TodoAction {
    /// Add a new task
    Add {
        /// Task description
        text: String,
    },
    /// List all tasks
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Mark a task as done
    Done {
        /// Task ID
        id: u32,
    },
    /// Remove a task
    Remove {
        /// Task ID
        id: u32,
    },
    /// Move a task to a new position
    Move {
        /// Task ID
        id: u32,
        /// Target position (1-based)
        position: u32,
    },
    /// Edit a task's text
    Edit {
        /// Task ID
        id: u32,
        /// New text
        text: String,
    },
    /// Remove all completed tasks
    Clear,
}
```

**Step 3: Wire up the subcommand handling in main()**

In the `match command` block (around line 59), add the `Commands::Todo` arm:

```rust
            Commands::Todo { action } => {
                let mut todos = todo::TodoList::load();
                match action {
                    None | Some(TodoAction::List { json: false }) => {
                        todos.print_list();
                    }
                    Some(TodoAction::List { json: true }) => {
                        todos.print_list_json();
                    }
                    Some(TodoAction::Add { text }) => {
                        todos.add(text.clone());
                        if let Err(e) = todos.save() {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        println!("Added: {text} (#{})","todos.next_id - 1);
                    }
                    Some(TodoAction::Done { id }) => {
                        if let Err(e) = todos.mark_done(id) {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        if let Err(e) = todos.save() {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        println!("Marked #{id} as done.");
                    }
                    Some(TodoAction::Remove { id }) => {
                        if let Err(e) = todos.remove(id) {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        if let Err(e) = todos.save() {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        println!("Removed #{id}.");
                    }
                    Some(TodoAction::Move { id, position }) => {
                        if let Err(e) = todos.move_to(id, position) {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        if let Err(e) = todos.save() {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        println!("Moved #{id} to position {position}.");
                    }
                    Some(TodoAction::Edit { id, text }) => {
                        if let Err(e) = todos.edit(id, text.clone()) {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        if let Err(e) = todos.save() {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        println!("Updated #{id}: {text}");
                    }
                    Some(TodoAction::Clear) => {
                        let count = todos.clear_completed();
                        if let Err(e) = todos.save() {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        println!("Cleared {count} completed task{}.", if count == 1 { "" } else { "s" });
                    }
                }
            }
```

**Step 4: Build and manually test CLI**

Run: `cargo build && ./target/debug/tik todo add "Test task" && ./target/debug/tik todo list && ./target/debug/tik todo list --json`
Expected: Task added, listed in both formats

**Step 5: Commit**

```bash
git add src/main.rs src/todo.rs
git commit -m "feat(todo): add CLI subcommands for todo management"
```

---

### Task 4: Extend timer to accept todo state

**Files:**
- Modify: `src/timer.rs`
- Modify: `src/render.rs`

**Step 1: Define TodoState for the render/timer boundary**

Add to `src/todo.rs`:

```rust
/// Lightweight snapshot of todo state for rendering. Avoids passing the full TodoList
/// (with its file I/O methods) into the render loop.
#[derive(Debug, Clone)]
pub struct TodoSnapshot {
    pub items: Vec<(u32, String, bool)>, // (id, text, done)
    pub selected_index: usize,
    pub focus: bool, // true = todo panel has focus
}
```

**Step 2: Extend DrawParams in render.rs**

Add a new field to `DrawParams`:

```rust
pub struct DrawParams<'a> {
    pub remaining_secs: u64,
    pub total_secs: u64,
    pub elapsed_secs: u64,
    pub paused: bool,
    pub title: Option<&'a str>,
    pub round_info: Option<(u32, u32)>,
    pub context: crate::timer::TimerContext,
    pub todo: Option<&'a crate::todo::TodoSnapshot>, // NEW
}
```

**Step 3: Modify the draw method in render.rs for two-column layout**

Replace the `draw` method body. The key change: when `params.todo` is `Some`, use a two-column layout (timer on left, todos on right). When `None`, keep existing centered layout.

```rust
    pub fn draw(&self, params: &DrawParams) -> io::Result<()> {
        let (cols, rows) = terminal::size()?;
        let mut stdout = io::stdout();
        execute!(stdout, terminal::Clear(ClearType::All))?;

        if let Some(todo_snap) = params.todo {
            self.draw_with_sidebar(&mut stdout, params, todo_snap, cols, rows)?;
        } else {
            self.draw_centered(&mut stdout, params, cols, rows)?;
        }

        stdout.flush()?;
        Ok(())
    }
```

Move the existing draw logic into `draw_centered`, and create `draw_with_sidebar`:

```rust
    fn draw_centered(&self, stdout: &mut io::Stdout, params: &DrawParams, cols: u16, rows: u16) -> io::Result<()> {
        // ... existing draw logic (title, round, time, bar, elapsed, hints), using cols for centering
    }

    fn draw_with_sidebar(&self, stdout: &mut io::Stdout, params: &DrawParams, todo: &crate::todo::TodoSnapshot, cols: u16, rows: u16) -> io::Result<()> {
        let sidebar_width: u16 = 32;
        let separator_col = cols.saturating_sub(sidebar_width);
        let left_width = separator_col.saturating_sub(1);
        let mid_row = rows / 2;

        // --- Left side: timer (centered within left_width) ---

        // Current task above title
        if let Some((_, text, _)) = todo.items.iter().find(|(_, _, done)| !done) {
            let label = format!("> {text}");
            let truncated = if label.len() > left_width as usize - 2 {
                format!("{}...", &label[..left_width as usize - 5])
            } else {
                label
            };
            let col = left_width.saturating_sub(truncated.len() as u16) / 2;
            execute!(
                stdout,
                cursor::MoveTo(col, mid_row.saturating_sub(5)),
                SetForegroundColor(Color::White),
                SetAttribute(Attribute::Bold),
                Print(&truncated),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

        // Title
        if let Some(title) = params.title {
            let col = left_width.saturating_sub(title.len() as u16) / 2;
            execute!(
                stdout,
                cursor::MoveTo(col, mid_row.saturating_sub(4)),
                SetForegroundColor(Color::White),
                SetAttribute(Attribute::Bold),
                Print(title),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

        // Round info
        if let Some((current, total)) = params.round_info {
            let round_str = format!("Round {current}/{total}");
            let col = left_width.saturating_sub(round_str.len() as u16) / 2;
            execute!(
                stdout,
                cursor::MoveTo(col, mid_row.saturating_sub(3)),
                SetForegroundColor(Color::Cyan),
                SetAttribute(Attribute::Bold),
                Print(&round_str),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

        // Remaining time
        let remaining_str = format_time(params.remaining_secs);
        let time_col = left_width.saturating_sub(remaining_str.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(time_col, mid_row.saturating_sub(1)),
            SetAttribute(Attribute::Bold),
            Print(&remaining_str),
            SetAttribute(Attribute::Reset),
        )?;

        // Progress bar
        let progress = if params.total_secs > 0 {
            1.0 - (params.remaining_secs as f64 / params.total_secs as f64)
        } else { 1.0 };
        let filled = (progress * self.bar_width as f64) as u16;
        let empty = self.bar_width - filled;
        let bar_color = if params.remaining_secs <= 60 {
            Color::Red
        } else if params.remaining_secs as f64 <= params.total_secs as f64 * 0.2 {
            Color::Yellow
        } else {
            Color::Green
        };
        let bar_filled: String = "\u{2588}".repeat(filled as usize);
        let bar_empty: String = "\u{2591}".repeat(empty as usize);
        let bar_col = left_width.saturating_sub(self.bar_width) / 2;
        execute!(
            stdout,
            cursor::MoveTo(bar_col, mid_row + 1),
            SetForegroundColor(bar_color),
            Print(&bar_filled),
            SetForegroundColor(Color::DarkGrey),
            Print(&bar_empty),
            ResetColor,
        )?;

        // Elapsed / PAUSED
        let elapsed_str = format_time(params.elapsed_secs);
        let label = if params.paused { "PAUSED".to_string() } else { format!("{elapsed_str} elapsed") };
        let label_col = left_width.saturating_sub(label.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(label_col, mid_row + 3),
            SetForegroundColor(Color::DarkGrey),
            Print(&label),
            ResetColor,
        )?;

        // Hint bar
        let hints = if todo.focus {
            "[tab] timer  [\u{2191}\u{2193}] select  [enter] done  [S-\u{2191}\u{2193}] move"
        } else {
            let is_last_round = params.round_info.is_some_and(|(cur, total)| cur >= total);
            match params.context {
                crate::timer::TimerContext::Standalone => {
                    "[space] pause  [s] skip  [tab] tasks  [x] stop"
                }
                _ if is_last_round => {
                    "[space] pause  [a/d] +/-round  [tab] tasks  [x] stop"
                }
                _ => {
                    "[space] pause  [s] skip  [a/d] +/-round  [tab] tasks  [x] stop"
                }
            }
        };
        let hints_col = left_width.saturating_sub(hints.len() as u16) / 2;
        execute!(
            stdout,
            cursor::MoveTo(hints_col, mid_row + 5),
            SetForegroundColor(Color::DarkGrey),
            Print(hints),
            ResetColor,
        )?;

        // --- Vertical separator ---
        for row in 0..rows {
            execute!(
                stdout,
                cursor::MoveTo(separator_col, row),
                SetForegroundColor(Color::DarkGrey),
                Print("\u{2502}"),
                ResetColor,
            )?;
        }

        // --- Right side: todo list ---
        let right_start = separator_col + 2;
        let max_text_width = (sidebar_width - 4) as usize;

        execute!(
            stdout,
            cursor::MoveTo(right_start, 1),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print("Tasks:"),
            SetAttribute(Attribute::Reset),
            ResetColor,
        )?;

        for (i, (_, text, done)) in todo.items.iter().enumerate() {
            let row = 3 + i as u16;
            if row >= rows - 1 { break; }

            let is_selected = todo.focus && i == todo.selected_index;
            let truncated = if text.len() > max_text_width {
                format!("{}...", &text[..max_text_width - 3])
            } else {
                text.clone()
            };

            let (prefix, color) = if *done {
                ("\u{2713} ", Color::DarkGrey)
            } else if i == 0 || (!done && todo.items.iter().take(i).all(|(_, _, d)| *d)) {
                ("> ", Color::White)  // current task marker
            } else {
                ("  ", Color::Grey)
            };

            let highlight_color = if is_selected { Color::Cyan } else { color };

            execute!(
                stdout,
                cursor::MoveTo(right_start, row),
                SetForegroundColor(highlight_color),
            )?;

            if is_selected {
                execute!(stdout, SetAttribute(Attribute::Bold))?;
            }
            if *done {
                execute!(stdout, SetAttribute(Attribute::CrossedOut))?;
            }

            execute!(
                stdout,
                Print(prefix),
                Print(&truncated),
                SetAttribute(Attribute::Reset),
                ResetColor,
            )?;
        }

        Ok(())
    }
```

**Step 4: Build to verify compilation**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/render.rs src/todo.rs
git commit -m "feat(todo): add sidebar rendering with two-column layout"
```

---

### Task 5: Wire todo state through timer input loop

**Files:**
- Modify: `src/timer.rs`

**Step 1: Add todo-related parameters and channels to timer::run**

Update the `run` function signature to accept todo state:

```rust
use std::sync::Mutex;

pub async fn run(
    total_secs: u64,
    _name: &str,
    context: TimerContext,
    title: Option<&str>,
    round_info: Option<(u32, Arc<AtomicU32>)>,
    todos: Option<Arc<Mutex<crate::todo::TodoList>>>,  // NEW
) -> TimerOutcome {
```

Add watch channels for todo focus and selected index inside the function:

```rust
    let (todo_focus_tx, todo_focus_rx) = watch::channel(false);
    let (todo_selected_tx, todo_selected_rx) = watch::channel(0usize);
```

**Step 2: Extend keyboard input thread with todo controls**

In the `std::thread::spawn` closure, add handling for Tab, arrows, Enter, and Shift+arrows:

```rust
    let todos_clone = todos.clone();
    // ... existing clones ...
    std::thread::spawn(move || {
        loop {
            if event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    let in_todo_focus = *todo_focus_tx_clone.borrow();

                    if in_todo_focus {
                        // Todo-focus key handling
                        match key {
                            KeyEvent { code: KeyCode::Tab, .. } => {
                                let _ = todo_focus_tx_clone.send(false);
                            }
                            KeyEvent { code: KeyCode::Up, modifiers, .. }
                                if modifiers.contains(KeyModifiers::SHIFT) =>
                            {
                                // Move task up
                                if let Some(ref todos) = todos_clone {
                                    let sel = *todo_selected_tx_clone.borrow();
                                    if let Ok(mut list) = todos.lock() {
                                        if list.move_up(sel).is_ok() && sel > 0 {
                                            let _ = todo_selected_tx_clone.send(sel - 1);
                                        }
                                    }
                                }
                            }
                            KeyEvent { code: KeyCode::Down, modifiers, .. }
                                if modifiers.contains(KeyModifiers::SHIFT) =>
                            {
                                // Move task down
                                if let Some(ref todos) = todos_clone {
                                    let sel = *todo_selected_tx_clone.borrow();
                                    if let Ok(mut list) = todos.lock() {
                                        let len = list.items.len();
                                        if list.move_down(sel).is_ok() && sel + 1 < len {
                                            let _ = todo_selected_tx_clone.send(sel + 1);
                                        }
                                    }
                                }
                            }
                            KeyEvent { code: KeyCode::Up, .. } => {
                                let sel = *todo_selected_tx_clone.borrow();
                                if sel > 0 {
                                    let _ = todo_selected_tx_clone.send(sel - 1);
                                }
                            }
                            KeyEvent { code: KeyCode::Down, .. } => {
                                let sel = *todo_selected_tx_clone.borrow();
                                if let Some(ref todos) = todos_clone {
                                    if let Ok(list) = todos.lock() {
                                        if sel + 1 < list.items.len() {
                                            let _ = todo_selected_tx_clone.send(sel + 1);
                                        }
                                    }
                                }
                            }
                            KeyEvent { code: KeyCode::Enter, .. } => {
                                // Mark selected as done
                                if let Some(ref todos) = todos_clone {
                                    let sel = *todo_selected_tx_clone.borrow();
                                    if let Ok(mut list) = todos.lock() {
                                        if let Some(todo) = list.items.get(sel) {
                                            let id = todo.id;
                                            let _ = list.mark_done(id);
                                        }
                                    }
                                }
                            }
                            // Ctrl+C still works in todo focus
                            KeyEvent { code: KeyCode::Char('c'), modifiers, .. }
                                if modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                let _ = quit_tx_clone.send(true);
                                break;
                            }
                            _ => {}
                        }
                    } else {
                        // Timer-focus key handling (existing + Tab)
                        match key {
                            KeyEvent { code: KeyCode::Tab, .. } => {
                                if todos_clone.is_some() {
                                    let _ = todo_focus_tx_clone.send(true);
                                }
                            }
                            // ... all existing key handlers unchanged ...
                        }
                    }
                }
            }
            if *quit_tx_clone.borrow() {
                break;
            }
        }
    });
```

**Step 3: Build the TodoSnapshot in the render loop**

In the main render loop, build the snapshot before calling `renderer.draw()`:

```rust
        let todo_snapshot = todos.as_ref().and_then(|t| {
            let list = t.lock().ok()?;
            if !list.has_pending() && list.items.iter().all(|t| t.done) && list.items.is_empty() {
                return None; // No sidebar when empty
            }
            Some(crate::todo::TodoSnapshot {
                items: list.items.iter().map(|t| (t.id, t.text.clone(), t.done)).collect(),
                selected_index: *todo_selected_rx.borrow(),
                focus: *todo_focus_rx.borrow(),
            })
        });

        let params = crate::render::DrawParams {
            remaining_secs,
            total_secs,
            elapsed_secs,
            paused: is_paused,
            title,
            round_info: current_round_info,
            context,
            todo: todo_snapshot.as_ref(),
        };
```

**Step 4: Build to verify compilation**

Run: `cargo build`
Expected: Compiles. Fix any borrow/lifetime issues.

**Step 5: Commit**

```bash
git add src/timer.rs
git commit -m "feat(todo): wire todo state and keyboard controls into timer"
```

---

### Task 6: Wire todo through session and standalone timer

**Files:**
- Modify: `src/session.rs`
- Modify: `src/main.rs`

**Step 1: Update session::run_session to load and pass TodoList**

In `src/session.rs`, modify `run_session` to load todos and pass them through:

```rust
use std::sync::{Arc, Mutex};

pub async fn run_session(session: &SessionConfig, config: &Config, silent: bool, title: Option<&str>) {
    let todos = {
        let list = crate::todo::TodoList::load();
        if list.items.is_empty() {
            None
        } else {
            Some(Arc::new(Mutex::new(list)))
        }
    };

    // ... existing code ...

    // Pass todos to each timer::run call:
    let outcome = timer::run(
        work_dur.total_secs,
        &session.work,
        timer::TimerContext::Work,
        title,
        Some((round, Arc::clone(&total_rounds))),
        todos.clone(),  // NEW
    ).await;

    // ... same for break timer::run call ...
```

At the end of `run_session`, save the todo list if it was modified:

```rust
    // Save todos if they were loaded
    if let Some(ref todos) = todos {
        if let Ok(list) = todos.lock() {
            if let Err(e) = list.save() {
                eprintln!("Failed to save todos: {e}");
            }
        }
    }
```

**Step 2: Update standalone timer call in main.rs**

In `main.rs`, update the standalone `timer::run` call (around line 123):

```rust
    let todos = {
        let list = todo::TodoList::load();
        if list.items.is_empty() {
            None
        } else {
            Some(Arc::new(std::sync::Mutex::new(list)))
        }
    };

    let outcome = timer::run(
        dur.total_secs,
        &name,
        timer::TimerContext::Standalone,
        cli.title.as_deref(),
        None,
        todos.clone(),  // NEW
    ).await;

    // After timer, save todos
    if let Some(ref todos) = todos {
        if let Ok(list) = todos.lock() {
            if let Err(e) = list.save() {
                eprintln!("Failed to save todos: {e}");
            }
        }
    }
```

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Manual integration test**

Run:
```bash
./target/debug/tik todo add "Test task 1"
./target/debug/tik todo add "Test task 2"
./target/debug/tik 5s
```
Expected: Timer runs with sidebar showing two tasks. Tab switches focus. Arrow keys navigate. Enter marks done.

**Step 5: Commit**

```bash
git add src/session.rs src/main.rs
git commit -m "feat(todo): integrate todo display into timer and session"
```

---

### Task 7: Final polish and edge cases

**Files:**
- Modify: `src/timer.rs` (save on mark-done)
- Modify: `src/render.rs` (handle narrow terminals)

**Step 1: Auto-save when marking tasks done during timer**

In the `KeyCode::Enter` handler in timer.rs, save after marking done:

```rust
KeyEvent { code: KeyCode::Enter, .. } => {
    if let Some(ref todos) = todos_clone {
        let sel = *todo_selected_tx_clone.borrow();
        if let Ok(mut list) = todos.lock() {
            if let Some(todo) = list.items.get(sel) {
                let id = todo.id;
                let _ = list.mark_done(id);
                let _ = list.save(); // Auto-save
            }
        }
    }
}
```

Also save after move_up and move_down operations.

**Step 2: Handle narrow terminals in sidebar render**

In `draw_with_sidebar`, add a fallback: if `cols < 60`, fall back to `draw_centered`:

```rust
    fn draw_with_sidebar(&self, stdout: &mut io::Stdout, params: &DrawParams, todo: &crate::todo::TodoSnapshot, cols: u16, rows: u16) -> io::Result<()> {
        if cols < 60 {
            // Terminal too narrow for sidebar — fall back to centered
            return self.draw_centered(stdout, params, cols, rows);
        }
        // ... rest of sidebar logic
    }
```

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Build release and smoke test**

Run: `cargo build --release`
Then test the full workflow:
```bash
./target/release/tik todo add "Write docs"
./target/release/tik todo add "Run tests"
./target/release/tik todo list
./target/release/tik todo edit 1 "Write documentation"
./target/release/tik todo move 2 1
./target/release/tik todo list
./target/release/tik 10s   # verify sidebar appears
./target/release/tik todo done 2
./target/release/tik todo clear
./target/release/tik todo list
```

**Step 5: Commit**

```bash
git add src/timer.rs src/render.rs
git commit -m "feat(todo): add auto-save and narrow terminal fallback"
```
