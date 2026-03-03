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
        }
        return;
    }

    // Must have a duration/preset argument
    let input = match cli.duration {
        Some(d) => d,
        None => {
            eprintln!("Usage: tik <duration|preset> or tik log");
            eprintln!("Examples: tik 25m, tik pomodoro, tik 1h30m");
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
    let outcome = timer::run(dur.total_secs, &name, timer::TimerContext::Standalone, cli.title.as_deref(), None).await;

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
