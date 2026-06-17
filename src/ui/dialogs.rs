use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph,
    },
    Frame,
};

use crate::app::{App, Dialog};

pub fn render_dialog(app: &App, frame: &mut Frame) {
    match &app.dialog {
        Dialog::None => {}
        Dialog::NewLoopbackPickSource => {
            render_hint(frame, "Select source, then press Enter");
        }
        Dialog::NewLoopbackPickSink { source } => {
            render_hint(frame, &format!("Source: {}  —  now select a sink", source.description));
        }
        Dialog::MoveSinkInput { input } => {
            render_hint(frame, &format!("Moving '{}'  —  select target sink, then press Enter", input.app_name));
        }
        Dialog::ListenToSink { sink } => {
            render_hint(frame, &format!("Listening to '{}'  —  select output sink, then press Enter", sink.description));
        }
        Dialog::CreateVirtualSource { sink, name } => {
            render_input_popup(
                frame,
                " Create Virtual Input ",
                &format!("Virtual mic name (wraps monitor of '{}'):", sink.description),
                name,
            );
        }
        Dialog::NewVirtualSink { input } => {
            render_input_popup(frame, " New Virtual Sink ", "Sink name:", input);
        }
        Dialog::Presets { selected, .. } => {
            render_presets_popup(frame, app, *selected);
        }
        Dialog::SavePreset { input } => {
            render_input_popup(frame, " Save Preset ", "Preset name:", input);
        }
        Dialog::Error(msg) => {
            render_error_popup(frame, msg);
        }
    }
}

fn render_hint(frame: &mut Frame, msg: &str) {
    let area = frame.area();
    let hint_area = Rect {
        x: 1,
        y: area.height.saturating_sub(3),
        width: area.width.saturating_sub(2),
        height: 1,
    };
    let para = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(msg, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("  [Esc to cancel]", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(para, hint_area);
}

fn render_input_popup(frame: &mut Frame, title: &str, label: &str, input: &str) {
    let popup = centered_rect(50, 6, frame.area());
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(block.clone(), popup);

    let inner = block.inner(popup);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .margin(1)
        .split(inner);

    frame.render_widget(
        Paragraph::new(label).style(Style::default().fg(Color::Gray)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(format!("{}_", input))
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new("[Enter] confirm  [Esc] cancel")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[2],
    );
}

fn render_presets_popup(frame: &mut Frame, app: &App, selected: usize) {
    let popup = centered_rect(60, 40, frame.area());
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Presets ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(block.clone(), popup);

    let inner = block.inner(popup);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = if app.config.presets.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  (no presets saved yet — press 's' to save current)",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        app.config
            .presets
            .iter()
            .map(|p| {
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(&p.name, Style::default().fg(Color::White)),
                    Span::styled(
                        format!(
                            "  ({} loopback{}, {} virtual sink{})",
                            p.loopbacks.len(),
                            if p.loopbacks.len() == 1 { "" } else { "s" },
                            p.virtual_sinks.len(),
                            if p.virtual_sinks.len() == 1 { "" } else { "s" },
                        ),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect()
    };

    let mut state = ListState::default();
    if !app.config.presets.is_empty() {
        state.select(Some(selected));
    }

    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ "),
        chunks[0],
        &mut state,
    );

    frame.render_widget(
        Paragraph::new("[Enter] load  [s] save current  [Esc] cancel")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        chunks[1],
    );
}

fn render_error_popup(frame: &mut Frame, msg: &str) {
    let popup = centered_rect(60, 5, frame.area());
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Error ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(msg)
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center),
        inner,
    );
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_w = r.width * percent_x / 100;
    let popup_x = (r.width.saturating_sub(popup_w)) / 2 + r.x;
    let popup_y = (r.height.saturating_sub(height)) / 2 + r.y;
    Rect {
        x: popup_x,
        y: popup_y,
        width: popup_w.min(r.width),
        height: height.min(r.height),
    }
}
