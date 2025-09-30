use crate::tui::state::{AppState, FileStatus};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState) {
    // Update scroll based on viewport height (subtract borders and title)
    let viewport_height = area.height.saturating_sub(2) as usize;
    state.update_scroll(viewport_height);

    let visible_files = state.visible_files();

    let items: Vec<ListItem> = visible_files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let status_icon = match file.status {
                FileStatus::Modified => "M",
                FileStatus::Added => "A",
                FileStatus::Deleted => "D",
                FileStatus::Renamed => "R",
                FileStatus::Typechange => "T",
            };

            let status_color = match file.status {
                FileStatus::Modified => Color::Yellow,
                FileStatus::Added => Color::Green,
                FileStatus::Deleted => Color::Red,
                FileStatus::Renamed => Color::Cyan,
                FileStatus::Typechange => Color::Magenta,
            };

            let checkbox = if file.selected {
                "[x]"
            } else {
                "[ ]"
            };

            let staged_marker = if file.staged { "●" } else { "○" };

            let group_hint = if let Some(group) = &file.suggested_group {
                format!(" ({})", group)
            } else {
                String::new()
            };

            let line = Line::from(vec![
                Span::styled(checkbox, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(staged_marker, Style::default().fg(if file.staged { Color::Green } else { Color::Gray })),
                Span::raw(" "),
                Span::styled(status_icon, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::raw(file.path.display().to_string()),
                Span::styled(group_hint, Style::default().fg(Color::DarkGray)),
            ]);

            // Highlight selected item
            let mut style = Style::default();
            if idx == state.selected_index {
                style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
            }

            ListItem::new(line).style(style)
        })
        .collect();

    use crate::tui::state::FileFilter;

    let title = match state.file_filter {
        FileFilter::StagedOnly => "Files (Staged Only)",
        FileFilter::UnstagedOnly => "Files (Unstaged Only)",
        FileFilter::All => "Files (All)",
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    // Create list state with proper scrolling
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}