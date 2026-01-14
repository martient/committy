use crate::tui::state::{AppState, CommitFormField};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Type
            Constraint::Length(3),  // Scope
            Constraint::Length(5),  // Short message
            Constraint::Min(5),     // Long message
            Constraint::Length(3),  // Breaking change toggle
        ])
        .split(area);

    // Commit Type
    let type_active = state.current_field == CommitFormField::Type;
    let type_text = format!("{} (Space to cycle)", state.commit_type);

    let type_widget = Paragraph::new(type_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(if type_active { "► Commit Type [ACTIVE]" } else { "Commit Type" })
            .border_style(if type_active {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            }))
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(type_widget, chunks[0]);

    // Scope
    let scope_active = state.current_field == CommitFormField::Scope;
    let scope_text = if state.commit_scope.is_empty() {
        "(type here...)".to_string()
    } else {
        state.commit_scope.clone()
    };

    let scope_widget = Paragraph::new(scope_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(if scope_active { "► Scope (optional) [ACTIVE]" } else { "Scope (optional)" })
            .border_style(if scope_active {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            }))
        .style(Style::default().fg(if state.commit_scope.is_empty() { Color::DarkGray } else { Color::Yellow }));

    frame.render_widget(scope_widget, chunks[1]);

    // Short Message
    let message_active = state.current_field == CommitFormField::ShortMessage;
    let message_text = if state.commit_message.is_empty() {
        "(type here...)".to_string()
    } else {
        state.commit_message.clone()
    };

    let message_widget = Paragraph::new(message_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(if message_active { "► Short Message [ACTIVE]" } else { "Short Message *REQUIRED*" })
            .border_style(if message_active {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            }))
        .style(Style::default().fg(if state.commit_message.is_empty() { Color::DarkGray } else { Color::White }))
        .wrap(Wrap { trim: false });

    frame.render_widget(message_widget, chunks[2]);

    // Long Message
    let body_active = state.current_field == CommitFormField::LongMessage;
    let body_text = if state.commit_body.is_empty() {
        "(type here... Enter for newline)".to_string()
    } else {
        state.commit_body.clone()
    };

    let body_widget = Paragraph::new(body_text)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(if body_active { "► Long Message (optional) [ACTIVE]" } else { "Long Message (optional)" })
            .border_style(if body_active {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            }))
        .style(Style::default().fg(if state.commit_body.is_empty() { Color::DarkGray } else { Color::Gray }))
        .wrap(Wrap { trim: false });

    frame.render_widget(body_widget, chunks[3]);

    // Breaking Change Toggle
    let breaking_active = state.current_field == CommitFormField::BreakingChange;
    let breaking_text = if state.breaking_change {
        "[x] Breaking Change (Space to toggle)"
    } else {
        "[ ] Breaking Change (Space to toggle)"
    };

    let breaking_widget = Paragraph::new(Line::from(vec![
        Span::styled(breaking_text, Style::default().fg(if state.breaking_change { Color::Red } else { Color::Gray })),
    ]))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(if breaking_active {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        }))
    .style(Style::default());

    frame.render_widget(breaking_widget, chunks[4]);
}