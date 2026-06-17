pub mod dialogs;
pub mod panels;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use dialogs::render_dialog;
use panels::{render_applications, render_loopbacks, render_sinks, render_sources};

pub fn render(app: &App, frame: &mut Frame) {
    let area = frame.area();

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(5),    // top row: sources | sinks
            Constraint::Min(3),    // bottom row: applications | loopbacks
            Constraint::Length(1), // help bar
        ])
        .split(area);

    render_title(app, frame, root[0]);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[1]);

    render_sources(app, frame, top[0]);
    render_sinks(app, frame, top[1]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[2]);

    render_applications(app, frame, bottom[0]);
    render_loopbacks(app, frame, bottom[1]);

    render_helpbar(app, frame, root[3]);
    render_dialog(app, frame);

    if let Some(ref msg) = app.status_msg {
        render_status(frame, msg, root[3]);
    }
}

fn render_title(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let title = Line::from(vec![
        Span::styled(
            " sinkercli ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("[{}]", app.audio_system.label()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(title), area);
}

fn render_helpbar(_app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let help = Line::from(vec![
        help_key("Tab"), help_sep(" focus  "),
        help_key("↑↓"), help_sep(" navigate  "),
        help_key("n"), help_sep(" loopback  "),
        help_key("v"), help_sep(" virtual sink  "),
        help_key("u"), help_sep(" virtual input  "),
        help_key("l"), help_sep(" listen  "),
        help_key("m"), help_sep(" move app  "),
        help_key("d"), help_sep(" delete  "),
        help_key("p"), help_sep(" presets  "),
        help_key("r"), help_sep(" refresh  "),
        help_key("q"), help_sep(" quit"),
    ]);
    frame.render_widget(
        Paragraph::new(help).block(Block::default().borders(Borders::NONE)),
        area,
    );
}

fn render_status(frame: &mut Frame, msg: &str, area: ratatui::layout::Rect) {
    let line = Line::from(vec![Span::styled(
        format!(" {} ", msg),
        Style::default().fg(Color::Yellow),
    )]);
    frame.render_widget(Paragraph::new(line), area);
}

fn help_key(k: &'static str) -> Span<'static> {
    Span::styled(
        format!("[{}]", k),
        Style::default()
            .fg(Color::Black)
            .bg(Color::DarkGray)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )
}

fn help_sep(s: &'static str) -> Span<'static> {
    Span::styled(s, Style::default().fg(Color::DarkGray))
}
