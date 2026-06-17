mod app;
mod audio;
mod config;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, Dialog, Panel};
use audio::detect;
use audio::pulse::PulseBackend;

fn main() -> Result<()> {
    let audio_system = detect::detect();
    let backend: Box<dyn audio::AudioBackend> = Box::new(PulseBackend::new());

    let mut app = App::new(backend, audio_system);
    app.refresh();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let refresh_interval = Duration::from_secs(2);
    let mut last_refresh = Instant::now();

    loop {
        terminal.draw(|f| ui::render(app, f))?;

        // Auto-refresh audio state every 2 seconds
        let timeout = refresh_interval
            .checked_sub(last_refresh.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Clear status message on any keypress
                app.status_msg = None;

                if handle_key(app, key.code, key.modifiers)? {
                    return Ok(());
                }
            }
        } else {
            // Timeout — refresh audio state
            app.refresh();
            last_refresh = Instant::now();
        }

        if last_refresh.elapsed() >= refresh_interval {
            app.refresh();
            last_refresh = Instant::now();
        }
    }
}

/// Returns true if the app should quit.
fn handle_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<bool> {
    match &app.dialog.clone() {
        Dialog::None => handle_key_normal(app, code, modifiers),
        Dialog::NewLoopbackPickSource | Dialog::NewLoopbackPickSink { .. } => {
            handle_key_loopback_wizard(app, code)
        }
        Dialog::MoveSinkInput { .. } => handle_key_move_sink_input(app, code),
        Dialog::ListenToSink { .. } => handle_key_listen_to_sink(app, code),
        Dialog::CreateVirtualSource { .. } => {
            handle_key_input(app, code, InputTarget::CreateVirtualSource)
        }
        Dialog::NewVirtualSink { .. } => handle_key_input(app, code, InputTarget::VirtualSink),
        Dialog::Presets { selected, .. } => {
            let sel = *selected;
            handle_key_presets(app, code, sel)
        }
        Dialog::SavePreset { .. } => handle_key_input(app, code, InputTarget::SavePreset),
        Dialog::Error(_) => {
            app.dialog = Dialog::None;
            Ok(false)
        }
    }
}

fn handle_key_normal(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<bool> {
    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(true),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),

        KeyCode::Tab => app.focused = app.focused.next(),
        KeyCode::BackTab => app.focused = app.focused.prev(),

        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),

        KeyCode::Char('n') => {
            app.dialog = Dialog::NewLoopbackPickSource;
            app.focused = Panel::Sources;
        }
        KeyCode::Char('v') => {
            app.dialog = Dialog::NewVirtualSink {
                input: String::new(),
            };
        }
        KeyCode::Char('u') => {
            app.begin_create_virtual_source();
        }
        KeyCode::Char('l') => {
            app.begin_listen_to_sink();
        }
        KeyCode::Char('m') => {
            app.begin_move_sink_input();
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            app.delete_selected();
        }
        KeyCode::Char('p') => {
            app.dialog = Dialog::Presets { selected: 0 };
        }
        KeyCode::Char('r') => {
            app.refresh();
            app.status_msg = Some("Refreshed".to_string());
        }
        _ => {}
    }
    Ok(false)
}

fn handle_key_loopback_wizard(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.dialog = Dialog::None;
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Enter => {
            match &app.dialog {
                Dialog::NewLoopbackPickSource => {
                    // Advance to sink selection
                    app.confirm_new_loopback();
                    if matches!(&app.dialog, Dialog::NewLoopbackPickSink { .. }) {
                        app.focused = Panel::Sinks;
                    }
                }
                Dialog::NewLoopbackPickSink { .. } => {
                    app.confirm_new_loopback();
                    app.focused = Panel::Loopbacks;
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_key_move_sink_input(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.dialog = Dialog::None;
            app.focused = Panel::Applications;
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Enter => {
            app.confirm_move_sink_input();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_key_listen_to_sink(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.dialog = Dialog::None;
            app.focused = Panel::Sinks;
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Enter => {
            app.confirm_listen_to_sink();
        }
        _ => {}
    }
    Ok(false)
}

#[derive(Clone, Copy)]
enum InputTarget {
    VirtualSink,
    SavePreset,
    CreateVirtualSource,
}

fn handle_key_input(app: &mut App, code: KeyCode, target: InputTarget) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.dialog = Dialog::None;
        }
        KeyCode::Enter => match target {
            InputTarget::VirtualSink => app.confirm_new_virtual_sink(),
            InputTarget::SavePreset => {
                if let Dialog::SavePreset { ref input } = app.dialog.clone() {
                    let name = input.clone();
                    app.save_current_as_preset(&name);
                    app.dialog = Dialog::None;
                }
            }
            InputTarget::CreateVirtualSource => app.confirm_create_virtual_source(),
        },
        KeyCode::Backspace => {
            match &mut app.dialog {
                Dialog::NewVirtualSink { input }
                | Dialog::SavePreset { input }
                | Dialog::CreateVirtualSource { name: input, .. } => {
                    input.pop();
                }
                _ => {}
            }
        }
        KeyCode::Char(c) => {
            match &mut app.dialog {
                Dialog::NewVirtualSink { input }
                | Dialog::SavePreset { input }
                | Dialog::CreateVirtualSource { name: input, .. } => {
                    input.push(c);
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_key_presets(app: &mut App, code: KeyCode, selected: usize) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.dialog = Dialog::None;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if selected > 0 {
                app.dialog = Dialog::Presets { selected: selected - 1 };
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if selected + 1 < app.config.presets.len() {
                app.dialog = Dialog::Presets { selected: selected + 1 };
            }
        }
        KeyCode::Enter => {
            if !app.config.presets.is_empty() {
                if let Err(e) = app.load_preset(selected) {
                    app.dialog = Dialog::Error(format!("{}", e));
                } else {
                    app.dialog = Dialog::None;
                }
            }
        }
        KeyCode::Char('s') => {
            app.dialog = Dialog::SavePreset {
                input: String::new(),
            };
        }
        _ => {}
    }
    Ok(false)
}
