use crate::tui::state::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, _state: &AppState) {
    let help_text = vec![
        Line::from(vec![
            Span::styled("Committy TUI Help", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("File Selection Mode:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  ↑/k          - Move up"),
        Line::from("  ↓/j          - Move down"),
        Line::from("  Space        - Toggle file selection"),
        Line::from("  s            - Stage selected files"),
        Line::from("  u            - Unstage selected files"),
        Line::from("  a            - Select all files"),
        Line::from("  d            - Deselect all files"),
        Line::from("  c            - Go to commit message (if staged files exist)"),
        Line::from("  g            - Auto-group staged files and go to group view"),
        Line::from("  v            - View diff for selected file"),
        Line::from("  f            - Cycle file filter (All → Staged → Unstaged)"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Commit Message Mode:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  Tab          - Next field"),
        Line::from("  Shift+Tab    - Previous field"),
        Line::from("  Space        - Cycle commit type / Toggle breaking change"),
        Line::from("  Type text    - Input for scope, short message, long message"),
        Line::from("  Backspace    - Delete character"),
        Line::from("  Enter        - Newline in long message field"),
        Line::from("  Ctrl+Enter   - Create commit"),
        Line::from("  Esc          - Back to file selection"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Group View Mode:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  ↑/k          - Move up"),
        Line::from("  ↓/j          - Move down"),
        Line::from("  c            - Commit all groups as separate commits"),
        Line::from("  Esc          - Back to file selection"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Global Keys:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  ?/F1         - Show this help"),
        Line::from("  Ctrl+C/Esc   - Quit"),
        Line::from(""),
        Line::from(vec![
            Span::styled("File Status Icons:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  M", Style::default().fg(Color::Yellow)),
            Span::raw(" - Modified"),
        ]),
        Line::from(vec![
            Span::styled("  A", Style::default().fg(Color::Green)),
            Span::raw(" - Added"),
        ]),
        Line::from(vec![
            Span::styled("  D", Style::default().fg(Color::Red)),
            Span::raw(" - Deleted"),
        ]),
        Line::from(vec![
            Span::styled("  R", Style::default().fg(Color::Cyan)),
            Span::raw(" - Renamed"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Staged Marker:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  ●", Style::default().fg(Color::Green)),
            Span::raw(" - Staged"),
        ]),
        Line::from(vec![
            Span::styled("  ○", Style::default().fg(Color::Gray)),
            Span::raw(" - Unstaged"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Auto-grouping:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from("  Files are automatically grouped by type:"),
        Line::from("  - docs: README, .md files, /docs/ directories"),
        Line::from("  - tests: test files, spec files"),
        Line::from("  - ci: .github, .gitlab, CI configs"),
        Line::from("  - deps: Cargo.toml, package.json, requirements.txt"),
        Line::from("  - build: Makefile, build.rs, webpack configs"),
        Line::from(""),
        Line::from(Span::styled("Press ? or Esc to close this help", Style::default().fg(Color::Gray))),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false });

    frame.render_widget(help, area);
}