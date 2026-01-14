use super::{event::Event, state::AppState, ui, EventHandler, Tui};
use crate::error::CliError;
use crossterm::event::{KeyCode, KeyModifiers};

pub struct App {
    pub state: AppState,
    pub running: bool,
}

impl App {
    pub fn new() -> Result<Self, CliError> {
        let state = AppState::new().map_err(|e| CliError::Generic(e))?;
        Ok(Self {
            state,
            running: true,
        })
    }

    pub fn run(&mut self, terminal: &mut Tui) -> Result<(), CliError> {
        let events = EventHandler::new(250); // 250ms tick rate

        while self.running {
            terminal
                .draw(|f| ui::render(f, &mut self.state))
                .map_err(|e| CliError::Generic(format!("Failed to draw: {}", e)))?;

            match events
                .next()
                .map_err(|e| CliError::Generic(format!("Failed to get event: {}", e)))?
            {
                Event::Key(key) => self.handle_key_event(key)?,
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
                Event::Tick => {}
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<(), CliError> {
        use super::state::AppMode;

        // Global keybindings
        match (key.code, key.modifiers) {
            // Quit with Ctrl+C or ESC (depending on mode)
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.running = false;
                return Ok(());
            }
            (KeyCode::Esc, _) if self.state.mode != AppMode::CommitMessage => {
                self.running = false;
                return Ok(());
            }
            // Help
            (KeyCode::Char('?'), _) | (KeyCode::F(1), _) => {
                self.state.mode = AppMode::Help;
                return Ok(());
            }
            _ => {}
        }

        // Mode-specific keybindings
        match self.state.mode {
            AppMode::FileSelection => self.handle_file_selection_keys(key)?,
            AppMode::CommitMessage => self.handle_commit_message_keys(key)?,
            AppMode::GroupView => self.handle_group_view_keys(key)?,
            AppMode::DiffView => self.handle_diff_view_keys(key)?,
            AppMode::Help => self.handle_help_keys(key)?,
        }

        Ok(())
    }

    fn handle_file_selection_keys(&mut self, key: crossterm::event::KeyEvent) -> Result<(), CliError> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.move_selection_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.move_selection_down();
            }
            KeyCode::Char(' ') => {
                self.state.toggle_selected();
            }
            KeyCode::Char('s') => {
                match self.state.stage_selected() {
                    Ok(_) => {
                        self.state.success_message = Some("Files staged successfully".to_string());
                    }
                    Err(e) => {
                        self.state.error_message = Some(format!("Failed to stage files: {}", e));
                    }
                }
            }
            KeyCode::Char('u') => {
                match self.state.unstage_selected() {
                    Ok(_) => {
                        self.state.success_message = Some("Files unstaged successfully".to_string());
                    }
                    Err(e) => {
                        self.state.error_message = Some(format!("Failed to unstage files: {}", e));
                    }
                }
            }
            KeyCode::Char('a') => {
                // Select all
                for file in &mut self.state.files {
                    file.selected = true;
                }
            }
            KeyCode::Char('d') => {
                // Deselect all
                for file in &mut self.state.files {
                    file.selected = false;
                }
            }
            KeyCode::Char('c') => {
                // Go to commit message mode
                if self.state.has_staged_files() {
                    self.state.mode = super::state::AppMode::CommitMessage;
                    self.state.current_field = super::state::CommitFormField::Type;
                } else {
                    self.state.error_message = Some("No staged files to commit".to_string());
                }
            }
            KeyCode::Char('g') => {
                // Auto-group and go to group view
                if self.state.has_staged_files() {
                    self.state.create_auto_groups();
                    if !self.state.groups.is_empty() {
                        self.state.mode = super::state::AppMode::GroupView;
                    } else {
                        self.state.error_message = Some("No groups created".to_string());
                    }
                } else {
                    self.state.error_message = Some("No staged files to group".to_string());
                }
            }
            KeyCode::Char('v') => {
                // View diff
                self.state.mode = super::state::AppMode::DiffView;
            }
            KeyCode::Char('f') => {
                // Cycle file filter
                self.state.cycle_filter();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_commit_message_keys(&mut self, key: crossterm::event::KeyEvent) -> Result<(), CliError> {
        use super::state::CommitFormField;

        // Debug logging
        eprintln!("Key pressed: {:?}, Current field: {:?}", key.code, self.state.current_field);

        match key.code {
            KeyCode::Esc => {
                self.state.mode = super::state::AppMode::FileSelection;
            }
            KeyCode::Tab => {
                self.state.next_field();
            }
            KeyCode::BackTab => {
                self.state.prev_field();
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Commit with Ctrl+Enter
                if !self.state.commit_message.is_empty() {
                    self.perform_commit()?;
                } else {
                    self.state.error_message = Some("Commit message cannot be empty".to_string());
                }
            }
            // Handle Space key for Type and BreakingChange fields BEFORE general Char handling
            KeyCode::Char(' ') if self.state.current_field == CommitFormField::Type => {
                eprintln!("Cycling commit type from: {}", self.state.commit_type);
                self.state.cycle_commit_type();
                eprintln!("Cycled to: {}", self.state.commit_type);
            }
            KeyCode::Char(' ') if self.state.current_field == CommitFormField::BreakingChange => {
                self.state.breaking_change = !self.state.breaking_change;
            }
            KeyCode::Char(' ') => {
                // Space in text fields - only for Scope, ShortMessage, LongMessage
                match self.state.current_field {
                    CommitFormField::Scope => {
                        self.state.commit_scope.push(' ');
                    }
                    CommitFormField::ShortMessage => {
                        self.state.commit_message.push(' ');
                    }
                    CommitFormField::LongMessage => {
                        self.state.commit_body.push(' ');
                    }
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                // Text input for other characters
                match self.state.current_field {
                    CommitFormField::Scope => {
                        self.state.commit_scope.push(c);
                    }
                    CommitFormField::ShortMessage => {
                        self.state.commit_message.push(c);
                    }
                    CommitFormField::LongMessage => {
                        self.state.commit_body.push(c);
                    }
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                // Delete last character
                match self.state.current_field {
                    CommitFormField::Scope => {
                        self.state.commit_scope.pop();
                    }
                    CommitFormField::ShortMessage => {
                        self.state.commit_message.pop();
                    }
                    CommitFormField::LongMessage => {
                        self.state.commit_body.pop();
                    }
                    _ => {}
                }
            }
            KeyCode::Enter => {
                // Newline in long message only
                if self.state.current_field == CommitFormField::LongMessage {
                    self.state.commit_body.push('\n');
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_group_view_keys(&mut self, key: crossterm::event::KeyEvent) -> Result<(), CliError> {
        match key.code {
            KeyCode::Esc => {
                self.state.mode = super::state::AppMode::FileSelection;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.state.selected_group > 0 {
                    self.state.selected_group -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.state.selected_group < self.state.groups.len().saturating_sub(1) {
                    self.state.selected_group += 1;
                }
            }
            KeyCode::Char('c') => {
                // Commit all groups
                self.commit_all_groups()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_diff_view_keys(&mut self, key: crossterm::event::KeyEvent) -> Result<(), CliError> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state.mode = super::state::AppMode::FileSelection;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_help_keys(&mut self, key: crossterm::event::KeyEvent) -> Result<(), CliError> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.state.mode = super::state::AppMode::FileSelection;
            }
            _ => {}
        }
        Ok(())
    }

    fn perform_commit(&mut self) -> Result<(), CliError> {
        use crate::git;

        let full_message = git::format_commit_message(
            &self.state.commit_type,
            self.state.breaking_change,
            &self.state.commit_scope,
            &self.state.commit_message,
            &self.state.commit_body,
        );

        git::commit_changes(&full_message, false)?;

        self.state.success_message = Some("Commit created successfully!".to_string());
        self.running = false; // Exit after commit
        Ok(())
    }

    fn commit_all_groups(&mut self) -> Result<(), CliError> {
        use crate::git;

        for group in &self.state.groups {
            // Message should be just the description, not include the type prefix
            let message = group.suggested_message.as_ref()
                .map(|m| m.clone())
                .unwrap_or_else(|| format!("update {} files", group.name));

            let full_message = git::format_commit_message(
                &group.commit_type,
                false,
                &group.name,
                &message,
                "",
            );

            // Stage only the files in this group
            let repo = git2::Repository::open(".").map_err(CliError::from)?;
            let mut index = repo.index().map_err(CliError::from)?;

            for file_path in &group.files {
                index.add_path(file_path).map_err(CliError::from)?;
            }
            index.write().map_err(CliError::from)?;

            // Commit
            git::commit_changes(&full_message, false)?;
        }

        self.state.success_message = Some(format!("{} commits created successfully!", self.state.groups.len()));
        self.running = false; // Exit after commits
        Ok(())
    }
}