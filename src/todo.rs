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
}
