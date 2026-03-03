use crate::config::{Config, SessionConfig};
use crate::duration::Duration;
use crate::log::LogEntry;
use crate::timer;
use chrono::Local;
use crossterm::{
    cursor, execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{self, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub async fn run_session(session: &SessionConfig, config: &Config, silent: bool, title: Option<&str>) {
    let total_rounds = Arc::new(AtomicU32::new(session.rounds));
    let todos = {
        let list = crate::todo::TodoList::load();
        if list.items.is_empty() {
            None
        } else {
            Some(Arc::new(Mutex::new(list)))
        }
    };
    let mut round: u32 = 1;
    let mut in_alt_screen = false;

    loop {
        let current_total = total_rounds.load(Ordering::Relaxed);
        if round > current_total {
            break;
        }

        // --- Work phase ---
        let work_duration_str = config
            .resolve_preset(&session.work)
            .unwrap_or(&session.work);
        let work_dur = match Duration::parse(work_duration_str) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Invalid work duration '{}': {e}", session.work);
                return;
            }
        };

        // Show header: if previous phase was skipped, we're already in alternate screen
        if in_alt_screen {
            draw_round_header_content(round, current_total, &session.work, &work_dur.format_hms(), title);
        } else {
            show_round_header(round, current_total, &session.work, &work_dur.format_hms(), title);
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let outcome = timer::run(
            work_dur.total_secs,
            &session.work,
            timer::TimerContext::Work,
            title,
            Some((round, Arc::clone(&total_rounds))),
            todos.clone(),
        ).await;

        in_alt_screen = outcome == timer::TimerOutcome::Skipped;

        match outcome {
            timer::TimerOutcome::Quit => {
                println!("Session cancelled.");
                return;
            }
            timer::TimerOutcome::StoppedEarly => {
                cleanup_alt_screen();
                println!("Session stopped early after {} round{}.", round.saturating_sub(1), if round.saturating_sub(1) == 1 { "" } else { "s" });
                return;
            }
            _ => {} // Completed or Skipped — continue to break
        }

        if !in_alt_screen {
            crate::notify::send_completion(&session.work, &work_dur.format_hms(), silent);
        }
        log_entry(&session.work, work_dur.total_secs);

        // --- Break phase ---
        let current_total = total_rounds.load(Ordering::Relaxed);
        let (break_name, break_duration_str) = if round == current_total {
            let dur_str = config
                .resolve_preset(&session.long_break)
                .unwrap_or(&session.long_break);
            (&session.long_break, dur_str.to_string())
        } else {
            let dur_str = config
                .resolve_preset(&session.break_preset)
                .unwrap_or(&session.break_preset);
            (&session.break_preset, dur_str.to_string())
        };

        let break_dur = match Duration::parse(&break_duration_str) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Invalid break duration '{break_name}': {e}");
                return;
            }
        };

        if in_alt_screen {
            draw_round_header_content(round, current_total, break_name, &break_dur.format_hms(), title);
        } else {
            show_round_header(round, current_total, break_name, &break_dur.format_hms(), title);
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let outcome = timer::run(
            break_dur.total_secs,
            break_name,
            timer::TimerContext::Break,
            title,
            Some((round, Arc::clone(&total_rounds))),
            todos.clone(),
        ).await;

        in_alt_screen = outcome == timer::TimerOutcome::Skipped;

        match outcome {
            timer::TimerOutcome::Quit => {
                println!("Session cancelled.");
                return;
            }
            timer::TimerOutcome::StoppedEarly => {
                cleanup_alt_screen();
                println!("Session stopped early after {} round{}.", round, if round == 1 { "" } else { "s" });
                return;
            }
            _ => {} // Completed or Skipped — continue
        }

        if !in_alt_screen {
            crate::notify::send_completion(break_name, &break_dur.format_hms(), silent);
        }
        log_entry(break_name, break_dur.total_secs);

        round += 1;
    }

    if in_alt_screen {
        cleanup_alt_screen();
    }

    // Save todos if they were modified during session
    if let Some(ref todos) = todos {
        if let Ok(list) = todos.lock() {
            if let Err(e) = list.save() {
                eprintln!("Failed to save todos: {e}");
            }
        }
    }

    let final_total = total_rounds.load(Ordering::Relaxed);
    println!("Session complete! {} rounds finished.", final_total);
}

fn cleanup_alt_screen() {
    let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
}

fn show_round_header(round: u32, total: u32, name: &str, duration: &str, title: Option<&str>) {
    let _ = terminal::enable_raw_mode();
    let _ = execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide);

    draw_round_header_content(round, total, name, duration, title);

    let _ = io::stdout().flush();
    let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
}

/// Draw round header content without managing alternate screen.
/// Used both by show_round_header (first phase entry) and for smooth
/// transitions when skipping (alternate screen already active).
fn draw_round_header_content(round: u32, total: u32, name: &str, duration: &str, title: Option<&str>) {
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let mid_row = rows / 2;

    let line1 = format!("Round {round}/{total}");
    let line2 = format!("{name} ({duration})");

    let col1 = cols.saturating_sub(line1.len() as u16) / 2;
    let col2 = cols.saturating_sub(line2.len() as u16) / 2;

    let _ = execute!(
        io::stdout(),
        terminal::Clear(ClearType::All),
    );

    if let Some(title) = title {
        let title_col = cols.saturating_sub(title.len() as u16) / 2;
        let _ = execute!(
            io::stdout(),
            cursor::MoveTo(title_col, mid_row.saturating_sub(3)),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print(title),
            SetAttribute(Attribute::Reset),
            ResetColor,
        );
    }

    let _ = execute!(
        io::stdout(),
        cursor::MoveTo(col1, mid_row.saturating_sub(1)),
        SetForegroundColor(Color::Cyan),
        SetAttribute(Attribute::Bold),
        Print(&line1),
        SetAttribute(Attribute::Reset),
        ResetColor,
        cursor::MoveTo(col2, mid_row + 1),
        SetForegroundColor(Color::DarkGrey),
        Print(&line2),
        ResetColor,
    );
    let _ = io::stdout().flush();
}

fn log_entry(name: &str, duration_secs: u64) {
    let entry = LogEntry {
        name: name.to_string(),
        duration_secs,
        completed_at: Local::now(),
    };
    if let Err(e) = crate::log::append_entry(&entry) {
        eprintln!("Failed to write log: {e}");
    }
}
