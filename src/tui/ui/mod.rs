mod file_list;
mod commit_form;
mod group_view;
mod help;

use crate::tui::state::{AppMode, AppState};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, state: &mut AppState) {
    let size = frame.area();

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(0),      // Content
            Constraint::Length(2),  // Status bar
        ])
        .split(size);

    // Render title
    render_title(frame, chunks[0], state);

    // Render content based on mode
    match state.mode {
        AppMode::FileSelection => file_list::render(frame, chunks[1], state),
        AppMode::CommitMessage => commit_form::render(frame, chunks[1], state),
        AppMode::GroupView => group_view::render(frame, chunks[1], state),
        AppMode::DiffView => render_diff_view(frame, chunks[1], state),
        AppMode::Help => help::render(frame, chunks[1], state),
    }

    // Render status bar
    render_status_bar(frame, chunks[2], state);

    // Render messages overlay if any
    if state.error_message.is_some() || state.success_message.is_some() {
        render_message_overlay(frame, size, state);
    }
}

fn render_title(frame: &mut Frame, area: Rect, state: &AppState) {
    let mode_text = match state.mode {
        AppMode::FileSelection => "File Selection",
        AppMode::CommitMessage => "Commit Message",
        AppMode::GroupView => "Group View",
        AppMode::DiffView => "Diff View",
        AppMode::Help => "Help",
    };

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Committy TUI", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" - "),
            Span::styled(mode_text, Style::default().fg(Color::Yellow)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Center);

    frame.render_widget(title, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    use crate::tui::state::FileFilter;

    let status_text = match state.mode {
        AppMode::FileSelection => {
            let staged = state.files.iter().filter(|f| f.staged).count();
            let total = state.files.len();
            let filter_text = match state.file_filter {
                FileFilter::All => "All",
                FileFilter::StagedOnly => "Staged",
                FileFilter::UnstagedOnly => "Unstaged",
            };
            format!("Staged: {}/{} | Filter: {} | [f] Filter | [Space] Select | [s] Stage | [u] Unstage | [c] Commit | [g] Group | [?] Help",
                staged, total, filter_text)
        }
        AppMode::CommitMessage => {
            "[Tab/Shift+Tab] Navigate | [Space] Cycle/Toggle | [Ctrl+Enter] Commit | [Esc] Back".to_string()
        }
        AppMode::GroupView => {
            format!("Groups: {} | [c] Commit All | [Esc] Back | [?] Help", state.groups.len())
        }
        AppMode::DiffView => {
            "[q/Esc] Back | [?] Help".to_string()
        }
        AppMode::Help => {
            "[q/Esc/?] Close Help".to_string()
        }
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));

    frame.render_widget(status, area);
}

fn render_message_overlay(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let (message, style) = if let Some(err) = &state.error_message {
        (err.clone(), Style::default().fg(Color::White).bg(Color::Red))
    } else if let Some(success) = &state.success_message {
        (success.clone(), Style::default().fg(Color::White).bg(Color::Green))
    } else {
        return;
    };

    // Center the message
    let message_width = (message.len() + 4).min(area.width as usize) as u16;
    let message_area = Rect {
        x: (area.width.saturating_sub(message_width)) / 2,
        y: area.height / 2,
        width: message_width,
        height: 3,
    };

    let message_widget = Paragraph::new(message)
        .style(style)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(message_widget, message_area);

    // Clear messages after display (will be shown for one frame)
    // In a real app, you'd want a timer
}

fn render_diff_view(frame: &mut Frame, area: Rect, state: &AppState) {
    let selected_file = state.files.get(state.selected_index);

    let text = if let Some(file) = selected_file {
        format!("Diff view for: {}\n\n(Diff rendering to be implemented)", file.path.display())
    } else {
        "No file selected".to_string()
    };

    let diff = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Diff View"))
        .style(Style::default().fg(Color::White));

    frame.render_widget(diff, area);
}