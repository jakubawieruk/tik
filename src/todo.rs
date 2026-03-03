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
        Self {
            next_id: 1,
            items: Vec::new(),
        }
    }

    pub fn load() -> Self {
        let path = todo_path();
        if !path.exists() {
            return Self::new();
        }
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::new(),
        };
        serde_json::from_str(&contents).unwrap_or_else(|_| Self::new())
    }

    pub fn save(&self) -> Result<(), String> {
        let path = todo_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize todos: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("Failed to write todos: {e}"))?;
        Ok(())
    }

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

    /// Returns the first pending (not done) task.
    pub fn current_task(&self) -> Option<&Todo> {
        self.items.iter().find(|t| !t.done)
    }

    pub fn print_list(&self) {
        if self.items.is_empty() {
            println!("No tasks.");
            return;
        }
        println!("Tasks:");
        for (i, todo) in self.items.iter().enumerate() {
            let status = if todo.done { "x" } else { " " };
            println!("  {}. [{}] {:<30} (#{})", i + 1, status, todo.text, todo.id);
        }
    }

    pub fn print_list_json(&self) {
        let json = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string());
        println!("{json}");
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_todo() {
        let todo = Todo {
            id: 1,
            text: "Write tests".to_string(),
            done: false,
            created_at: Local::now(),
            completed_at: None,
        };
        let json = serde_json::to_string(&todo).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"text\":\"Write tests\""));
        assert!(json.contains("\"done\":false"));
        assert!(json.contains("\"created_at\""));
        assert!(json.contains("\"completed_at\":null"));
    }

    #[test]
    fn deserialize_todo() {
        let json = r#"{"id":1,"text":"Write tests","done":false,"created_at":"2026-03-03T10:00:00+01:00","completed_at":null}"#;
        let todo: Todo = serde_json::from_str(json).unwrap();
        assert_eq!(todo.id, 1);
        assert_eq!(todo.text, "Write tests");
        assert!(!todo.done);
        assert!(todo.completed_at.is_none());
    }

    #[test]
    fn roundtrip_todo_list() {
        let list = TodoList {
            next_id: 3,
            items: vec![
                Todo {
                    id: 1,
                    text: "First task".to_string(),
                    done: true,
                    created_at: Local::now(),
                    completed_at: Some(Local::now()),
                },
                Todo {
                    id: 2,
                    text: "Second task".to_string(),
                    done: false,
                    created_at: Local::now(),
                    completed_at: None,
                },
            ],
        };
        let json = serde_json::to_string_pretty(&list).unwrap();
        let parsed: TodoList = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.next_id, 3);
        assert_eq!(parsed.items.len(), 2);
        assert_eq!(parsed.items[0].id, 1);
        assert_eq!(parsed.items[0].text, "First task");
        assert!(parsed.items[0].done);
        assert!(parsed.items[0].completed_at.is_some());
        assert_eq!(parsed.items[1].id, 2);
        assert_eq!(parsed.items[1].text, "Second task");
        assert!(!parsed.items[1].done);
        assert!(parsed.items[1].completed_at.is_none());
    }

    #[test]
    fn todo_path_ends_with_expected() {
        let path = todo_path();
        assert!(path.ends_with("pomitik/todos.json"));
    }

    #[test]
    fn load_empty_returns_default() {
        let list = TodoList::new();
        assert_eq!(list.next_id, 1);
        assert!(list.items.is_empty());
    }

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
        assert!(list.move_up(1).is_ok());
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
        assert!(list.move_down(0).is_ok());
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
}
