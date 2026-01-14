use crate::config::COMMIT_TYPES;
use git2::{Delta, DiffOptions, Repository};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Typechange,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub status: FileStatus,
    pub staged: bool,
    pub selected: bool,
    pub suggested_group: Option<String>, // For auto-grouping
}

#[derive(Debug, Clone)]
pub struct CommitGroup {
    pub name: String,
    pub commit_type: String,
    pub files: Vec<PathBuf>,
    pub suggested_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    FileSelection,  // Select files to stage/unstage
    CommitMessage,  // Write commit message
    GroupView,      // View/edit auto-grouped commits
    DiffView,       // View diff for selected file
    Help,           // Show help
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommitFormField {
    Type,
    Scope,
    ShortMessage,
    LongMessage,
    BreakingChange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileFilter {
    All,
    StagedOnly,
    UnstagedOnly,
}

pub struct AppState {
    pub mode: AppMode,
    pub files: Vec<FileEntry>,
    pub selected_index: usize,
    pub scroll_offset: usize,

    // Commit form state
    pub commit_type: String,
    pub commit_type_index: usize,
    pub commit_scope: String,
    pub commit_message: String,
    pub commit_body: String,
    pub breaking_change: bool,
    pub current_field: CommitFormField,

    // Groups for multi-commit feature
    pub groups: Vec<CommitGroup>,
    pub selected_group: usize,

    // AI integration
    pub ai_enabled: bool,
    pub ai_suggestions: HashMap<PathBuf, String>,

    // UI state
    pub file_filter: FileFilter,
    pub collapsed_folders: std::collections::HashSet<PathBuf>,
    pub error_message: Option<String>,
    pub success_message: Option<String>,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let files = Self::load_files()?;

        Ok(Self {
            mode: AppMode::FileSelection,
            files,
            selected_index: 0,
            scroll_offset: 0,
            commit_type: COMMIT_TYPES[0].to_string(),
            commit_type_index: 0,
            commit_scope: String::new(),
            commit_message: String::new(),
            commit_body: String::new(),
            breaking_change: false,
            current_field: CommitFormField::Type,
            groups: Vec::new(),
            selected_group: 0,
            ai_enabled: false,
            ai_suggestions: HashMap::new(),
            file_filter: FileFilter::All,
            collapsed_folders: std::collections::HashSet::new(),
            error_message: None,
            success_message: None,
        })
    }

    fn load_files() -> Result<Vec<FileEntry>, String> {
        let repo = Repository::open(".").map_err(|e| format!("Failed to open repository: {}", e))?;

        let mut files = Vec::new();

        // Get HEAD tree
        let head = repo.head().map_err(|e| format!("Failed to get HEAD: {}", e))?;
        let head_tree = head.peel_to_tree().map_err(|e| format!("Failed to get HEAD tree: {}", e))?;

        // Get staged files (index vs HEAD)
        let mut staged_opts = DiffOptions::new();
        let staged_diff = repo
            .diff_tree_to_index(Some(&head_tree), None, Some(&mut staged_opts))
            .map_err(|e| format!("Failed to get staged diff: {}", e))?;

        let mut staged_files = std::collections::HashSet::new();
        staged_diff.foreach(
            &mut |delta, _| {
                let path = delta.new_file().path().unwrap_or_else(|| delta.old_file().path().unwrap());

                // Skip directories
                if let Ok(cwd) = std::env::current_dir() {
                    let full_path = cwd.join(path);
                    if full_path.is_dir() {
                        return true;
                    }
                }

                let status = match delta.status() {
                    Delta::Added | Delta::Untracked => FileStatus::Added,
                    Delta::Deleted => FileStatus::Deleted,
                    Delta::Modified => FileStatus::Modified,
                    Delta::Renamed => FileStatus::Renamed,
                    Delta::Typechange => FileStatus::Typechange,
                    _ => FileStatus::Modified,
                };

                let suggested_group = Self::suggest_file_group(path);

                // Add staged files to the list
                files.push(FileEntry {
                    path: path.to_path_buf(),
                    status,
                    staged: true,
                    selected: false,
                    suggested_group,
                });

                staged_files.insert(path.to_path_buf());
                true
            },
            None,
            None,
            None,
        ).ok();

        // Get unstaged files (working dir vs index)
        let mut unstaged_opts = DiffOptions::new();
        unstaged_opts.include_untracked(true);
        unstaged_opts.recurse_untracked_dirs(true); // Recurse into untracked directories
        let unstaged_diff = repo
            .diff_index_to_workdir(None, Some(&mut unstaged_opts))
            .map_err(|e| format!("Failed to get unstaged diff: {}", e))?;

        unstaged_diff.foreach(
            &mut |delta, _| {
                let path = delta.new_file().path().unwrap_or_else(|| delta.old_file().path().unwrap());

                // Skip if this is a directory (should not happen with recurse enabled, but be safe)
                if let Ok(cwd) = std::env::current_dir() {
                    let full_path = cwd.join(path);
                    if full_path.is_dir() {
                        return true; // Skip directories
                    }
                }

                let status = match delta.status() {
                    Delta::Added | Delta::Untracked => FileStatus::Added,
                    Delta::Deleted => FileStatus::Deleted,
                    Delta::Modified => FileStatus::Modified,
                    Delta::Renamed => FileStatus::Renamed,
                    Delta::Typechange => FileStatus::Typechange,
                    _ => FileStatus::Modified,
                };

                let is_staged = staged_files.contains(path);
                let suggested_group = Self::suggest_file_group(path);

                files.push(FileEntry {
                    path: path.to_path_buf(),
                    status,
                    staged: is_staged,
                    selected: false,
                    suggested_group,
                });
                true
            },
            None,
            None,
            None,
        ).ok();

        Ok(files)
    }

    fn suggest_file_group(path: &std::path::Path) -> Option<String> {
        let path_str = path.to_str()?;

        // Docs
        if path_str.contains("README") || path_str.ends_with(".md") || path_str.contains("/docs/") {
            return Some("docs".to_string());
        }

        // Tests
        if path_str.contains("test") || path_str.contains("spec") || path_str.ends_with("_test.rs") {
            return Some("tests".to_string());
        }

        // CI/CD
        if path_str.contains(".github") || path_str.contains(".gitlab") || path_str.contains("ci") {
            return Some("ci".to_string());
        }

        // Dependencies
        if path_str.contains("Cargo.toml") || path_str.contains("package.json") || path_str.contains("requirements.txt") {
            return Some("deps".to_string());
        }

        // Build
        if path_str.contains("Makefile") || path_str.contains("build.rs") || path_str.contains("webpack") {
            return Some("build".to_string());
        }

        None
    }

    pub fn toggle_selected(&mut self) {
        if let Some(file) = self.files.get_mut(self.selected_index) {
            file.selected = !file.selected;
        }
    }

    pub fn stage_selected(&mut self) -> Result<(), String> {
        let repo = Repository::open(".").map_err(|e| format!("Failed to open repository: {}", e))?;
        let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;

        for file in &mut self.files {
            if file.selected {
                // Validate the path
                if !file.path.is_file() && file.status != FileStatus::Deleted {
                    return Err(format!("Invalid file path (might be a directory): {}", file.path.display()));
                }

                match file.status {
                    FileStatus::Deleted => {
                        index.remove_path(&file.path).map_err(|e| {
                            format!("Failed to remove file '{}': {}", file.path.display(), e)
                        })?;
                    }
                    _ => {
                        // For untracked files, we need to add them to the working directory first
                        if file.status == FileStatus::Added && !file.path.exists() {
                            return Err(format!("File does not exist: {}", file.path.display()));
                        }

                        index.add_path(&file.path).map_err(|e| {
                            format!("Failed to add file '{}': {} (path: {:?})",
                                file.path.display(), e, file.path)
                        })?;
                    }
                }
                file.staged = true;
                file.selected = false;
            }
        }

        index.write().map_err(|e| format!("Failed to write index: {}", e))?;
        Ok(())
    }

    pub fn unstage_selected(&mut self) -> Result<(), String> {
        let repo = Repository::open(".").map_err(|e| format!("Failed to open repository: {}", e))?;
        let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;

        // Get HEAD commit once if needed for non-added files
        let head_commit = if self.files.iter().any(|f| f.selected && f.staged && f.status != FileStatus::Added) {
            let head = repo.head().map_err(|e| format!("Failed to get HEAD: {}", e))?;
            Some(head.peel_to_commit().map_err(|e| format!("Failed to peel HEAD to commit: {}", e))?)
        } else {
            None
        };

        for file in &mut self.files {
            if file.selected && file.staged {
                // For newly added files (not in HEAD), we need to remove them from index
                if file.status == FileStatus::Added {
                    index.remove_path(&file.path)
                        .map_err(|e| format!("Failed to unstage new file '{}': {}", file.path.display(), e))?;
                } else if let Some(ref commit) = head_commit {
                    // For modified/deleted files, use reset_default with HEAD commit
                    repo.reset_default(Some(commit.as_object()), &[&file.path])
                        .map_err(|e| format!("Failed to unstage file '{}': {}", file.path.display(), e))?;
                }

                file.staged = false;
                file.selected = false;
            }
        }

        index.write().map_err(|e| format!("Failed to write index: {}", e))?;
        Ok(())
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;

            // Update scroll offset if selection moves above visible area
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    pub fn move_selection_down(&mut self) {
        let visible = self.visible_files();
        let max_index = visible.len().saturating_sub(1);

        if self.selected_index < max_index {
            self.selected_index += 1;
        }
    }

    pub fn update_scroll(&mut self, viewport_height: usize) {
        // Ensure selected item is visible
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.selected_index.saturating_sub(viewport_height - 1);
        }
    }

    pub fn create_auto_groups(&mut self) {
        let mut groups_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for file in &self.files {
            if file.staged {
                let group_name = file.suggested_group.clone().unwrap_or_else(|| "code".to_string());
                groups_map.entry(group_name).or_insert_with(Vec::new).push(file.path.clone());
            }
        }

        self.groups = groups_map.into_iter()
            .map(|(name, files)| {
                let commit_type = match name.as_str() {
                    "docs" => "docs",
                    "tests" => "test",
                    "ci" => "ci",
                    "deps" => "build",
                    "build" => "build",
                    _ => "feat",
                }.to_string();

                CommitGroup {
                    name,
                    commit_type,
                    files,
                    suggested_message: None,
                }
            })
            .collect();
    }

    pub fn has_staged_files(&self) -> bool {
        self.files.iter().any(|f| f.staged)
    }

    pub fn visible_files(&self) -> Vec<&FileEntry> {
        self.files.iter()
            .filter(|f| {
                match self.file_filter {
                    FileFilter::StagedOnly => f.staged,
                    FileFilter::UnstagedOnly => !f.staged,
                    FileFilter::All => true,
                }
            })
            .collect()
    }

    pub fn cycle_filter(&mut self) {
        self.file_filter = match self.file_filter {
            FileFilter::All => FileFilter::StagedOnly,
            FileFilter::StagedOnly => FileFilter::UnstagedOnly,
            FileFilter::UnstagedOnly => FileFilter::All,
        };

        // Reset selection if it's out of bounds for the new filter
        let visible_count = self.visible_files().len();
        if visible_count > 0 && self.selected_index >= visible_count {
            self.selected_index = visible_count.saturating_sub(1);
        }
        if visible_count == 0 {
            self.selected_index = 0;
        }
    }

    pub fn cycle_commit_type(&mut self) {
        self.commit_type_index = (self.commit_type_index + 1) % COMMIT_TYPES.len();
        self.commit_type = COMMIT_TYPES[self.commit_type_index].to_string();
    }

    pub fn next_field(&mut self) {
        self.current_field = match self.current_field {
            CommitFormField::Type => CommitFormField::Scope,
            CommitFormField::Scope => CommitFormField::ShortMessage,
            CommitFormField::ShortMessage => CommitFormField::LongMessage,
            CommitFormField::LongMessage => CommitFormField::BreakingChange,
            CommitFormField::BreakingChange => CommitFormField::Type,
        };
    }

    pub fn prev_field(&mut self) {
        self.current_field = match self.current_field {
            CommitFormField::Type => CommitFormField::BreakingChange,
            CommitFormField::Scope => CommitFormField::Type,
            CommitFormField::ShortMessage => CommitFormField::Scope,
            CommitFormField::LongMessage => CommitFormField::ShortMessage,
            CommitFormField::BreakingChange => CommitFormField::LongMessage,
        };
    }

    pub fn toggle_folder(&mut self, folder: PathBuf) {
        if self.collapsed_folders.contains(&folder) {
            self.collapsed_folders.remove(&folder);
        } else {
            self.collapsed_folders.insert(folder);
        }
    }
}