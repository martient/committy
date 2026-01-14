use crate::tui::state::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.groups.is_empty() {
        let empty = Paragraph::new("No groups available.\n\nPress 'g' in file selection mode to auto-group staged files.")
            .block(Block::default().borders(Borders::ALL).title("Auto-grouped Commits"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = state.groups.iter().enumerate().map(|(idx, group)| {
        let count = group.files.len();
        let type_colored = Span::styled(
            &group.commit_type,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        );

        let file_list = group.files
            .iter()
            .map(|p| format!("    • {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");

        let suggested = group.suggested_message
            .as_ref()
            .map(|m| format!("\n  Message: {}", m))
            .unwrap_or_default();

        let lines = vec![
            Line::from(vec![
                Span::raw("Group: "),
                Span::styled(&group.name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" ("),
                type_colored,
                Span::raw(")"),
                Span::styled(format!(" - {} files", count), Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(file_list),
            Line::from(suggested),
            Line::from(""),
        ];

        let mut style = Style::default();
        if idx == state.selected_group {
            style = style.bg(Color::DarkGray);
        }

        ListItem::new(lines).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title("Auto-grouped Commits - Review and Commit"))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    // Create list state for scrolling
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_group));

    frame.render_stateful_widget(list, area, &mut list_state);
}