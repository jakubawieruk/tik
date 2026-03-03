mod config;
mod duration;
mod log;
mod notify;
mod render;
mod session;
mod timer;
mod todo;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tik", about = "A command-line countdown timer", version)]
struct Cli {
    /// Duration (e.g., 25m, 1h30m, 90s) or preset name (e.g., pomodoro, break)
    duration: Option<String>,

    /// Suppress notification sound
    #[arg(long)]
    silent: bool,

    /// Optional title displayed in the timer
    #[arg(long)]
    title: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show session log summary
    Log,
    /// View or change configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage todo tasks
    Todo {
        #[command(subcommand)]
        action: Option<TodoAction>,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set a configuration value (keys: work, break, long-break, rounds)
    Set {
        /// Config key to set
        key: String,
        /// New value (duration like "25m" or number for rounds)
        value: String,
    },
}

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
    /// Mark a task as not done
    Undone {
        /// Task ID
        id: u32,
    },
    /// Remove a task
    Remove {
        /// Task ID
        id: u32,
    },
    /// Move a task to a new position (1-based)
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

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Handle subcommands
    if let Some(command) = cli.command {
        match command {
            Commands::Log => {
                log::print_summary();
            }
            Commands::Config { action } => {
                let cfg = config::Config::load();
                match action {
                    ConfigAction::Show => cfg.show_config(),
                    ConfigAction::Set { key, value } => {
                        if let Err(e) = config::Config::set_value(&key, &value) {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                    }
                }
            }
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
                        println!("Added: {} (#{})", text, todos.next_id - 1);
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
                    Some(TodoAction::Undone { id }) => {
                        if let Err(e) = todos.mark_undone(id) {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        if let Err(e) = todos.save() {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                        println!("Marked #{id} as not done.");
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
        }
        return;
    }

    // Must have a duration/preset argument
    let input = match cli.duration {
        Some(d) => d,
        None => {
            eprintln!("Usage: tik <duration|preset>");
            eprintln!("       tik <log|config|todo>");
            eprintln!("Examples: tik 25m, tik pomodoro, tik todo add \"Task\"");
            std::process::exit(1);
        }
    };

    // Resolution order: session → preset → duration
    let config = config::Config::load();

    // 1. Check if it's a session
    if let Some(session_config) = config.resolve_session(&input) {
        let session_config = session_config.clone();
        session::run_session(&session_config, &config, cli.silent, cli.title.as_deref()).await;
        return;
    }

    // 2. Try parsing as duration, then as preset
    let (name, dur) = match duration::Duration::parse(&input) {
        Ok(d) => (input.clone(), d),
        Err(_) => {
            // Try as preset
            match config.resolve_preset(&input) {
                Some(preset_duration) => match duration::Duration::parse(preset_duration) {
                    Ok(d) => (input.clone(), d),
                    Err(e) => {
                        eprintln!("Invalid preset duration for '{input}': {e}");
                        std::process::exit(1);
                    }
                },
                None => {
                    eprintln!("Unknown duration or preset: '{input}'");
                    eprintln!("Valid formats: 25m, 1h30m, 90s");
                    eprintln!("Built-in presets: pomodoro, break, long-break");
                    std::process::exit(1);
                }
            }
        }
    };

    let display = dur.format_hms();
    let todos = {
        let list = todo::TodoList::load();
        if list.items.is_empty() {
            None
        } else {
            Some(std::sync::Arc::new(std::sync::Mutex::new(list)))
        }
    };
    let outcome = timer::run(dur.total_secs, &name, timer::TimerContext::Standalone, cli.title.as_deref(), None, todos.clone()).await;

    // Save todos if they were modified during timer
    if let Some(ref todos) = todos {
        if let Ok(list) = todos.lock() {
            if let Err(e) = list.save() {
                eprintln!("Failed to save todos: {e}");
            }
        }
    }

    if outcome == timer::TimerOutcome::Completed {
        notify::send_completion(&name, &display, cli.silent);

        let entry = log::LogEntry {
            name,
            duration_secs: dur.total_secs,
            completed_at: chrono::Local::now(),
        };
        if let Err(e) = log::append_entry(&entry) {
            eprintln!("Failed to write log: {e}");
        }

        println!("Timer complete: {display}");
    } else {
        println!("Timer cancelled.");
    }
}
