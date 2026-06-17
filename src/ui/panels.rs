use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::{App, Dialog, Panel};

pub fn render_applications(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focused == Panel::Applications
        || matches!(&app.dialog, Dialog::MoveSinkInput { .. });
    let block = panel_block("Applications", focused);

    let items: Vec<ListItem> = if app.sink_inputs.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  (no audio streams)",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        app.sink_inputs
            .iter()
            .map(|si| {
                ListItem::new(Line::from(vec![
                    Span::styled(&si.app_name, Style::default().fg(Color::White)),
                    Span::styled(
                        format!("  → {}", si.current_sink_name),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect()
    };

    let mut state = ListState::default();
    if !app.sink_inputs.is_empty() {
        state.select(Some(app.sink_input_sel));
    }

    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(highlight_style(focused))
            .highlight_symbol("▶ "),
        area,
        &mut state,
    );
}

pub fn render_sources(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focused == Panel::Sources
        || matches!(&app.dialog, Dialog::NewLoopbackPickSource);
    let block = panel_block("Sources", focused);

    let items: Vec<ListItem> = app
        .sources
        .iter()
        .map(|s| {
            let style = if s.is_monitor {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![Span::styled(&s.description, style)]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.source_sel));

    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(highlight_style(focused))
            .highlight_symbol("▶ "),
        area,
        &mut state,
    );
}

pub fn render_sinks(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focused == Panel::Sinks
        || matches!(&app.dialog, Dialog::NewLoopbackPickSink { .. })
        || matches!(&app.dialog, Dialog::MoveSinkInput { .. });
    let block = panel_block("Sinks", focused);

    let items: Vec<ListItem> = app
        .sinks
        .iter()
        .map(|s| {
            let label = if s.is_virtual {
                format!("[null] {}", s.description)
            } else {
                s.description.clone()
            };
            ListItem::new(Line::from(vec![Span::raw(label)]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.sink_sel));

    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(highlight_style(focused))
            .highlight_symbol("▶ "),
        area,
        &mut state,
    );
}

pub fn render_loopbacks(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.focused == Panel::Loopbacks;
    let block = panel_block("Active Loopbacks", focused);

    let items: Vec<ListItem> = app
        .loopbacks
        .iter()
        .enumerate()
        .map(|(i, lb)| {
            let label = format!(
                "  {}  {} → {}",
                i + 1,
                lb.source_name,
                lb.sink_name
            );
            ListItem::new(Line::from(vec![Span::raw(label)]))
        })
        .collect();

    let mut state = ListState::default();
    if !app.loopbacks.is_empty() {
        state.select(Some(app.loopback_sel));
    }

    frame.render_stateful_widget(
        List::new(items)
            .block(block)
            .highlight_style(highlight_style(focused))
            .highlight_symbol(""),
        area,
        &mut state,
    );
}

fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
}

fn highlight_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Black)
            .bg(Color::DarkGray)
    }
}
