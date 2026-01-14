use serial_test::serial;

use committy::tui::state::{AppState, AppMode, CommitFormField, FileFilter};
use git2::{Repository, Signature};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test repository with files
fn setup_test_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    // Create initial commit
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    {
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();
    }

    (dir, repo)
}

/// Helper to create a file in the repo
fn create_file(dir: &TempDir, path: &str, content: &str) {
    let file_path = dir.path().join(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(file_path, content).unwrap();
}

#[test]
#[serial]
fn test_app_state_initialization() {
    let (_dir, _repo) = setup_test_repo();

    // Initialize in the test repo directory
    std::env::set_current_dir(_dir.path()).unwrap();

    let state = AppState::new().unwrap();

    assert_eq!(state.mode, AppMode::FileSelection);
    assert_eq!(state.selected_index, 0);
    assert_eq!(state.commit_type, "feat");
    assert_eq!(state.commit_scope, "");
    assert_eq!(state.commit_message, "");
    assert_eq!(state.breaking_change, false);
    assert_eq!(state.current_field, CommitFormField::Type);
    assert_eq!(state.file_filter, FileFilter::All);
}

#[test]
#[serial]
fn test_file_selection_navigation() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create some test files
    create_file(&dir, "file1.txt", "content1");
    create_file(&dir, "file2.txt", "content2");
    create_file(&dir, "file3.txt", "content3");

    let mut state = AppState::new().unwrap();

    // Initially at index 0
    assert_eq!(state.selected_index, 0);

    // Move down
    state.move_selection_down();
    assert_eq!(state.selected_index, 1);

    state.move_selection_down();
    assert_eq!(state.selected_index, 2);

    // Move up
    state.move_selection_up();
    assert_eq!(state.selected_index, 1);

    state.move_selection_up();
    assert_eq!(state.selected_index, 0);

    // Can't go below 0
    state.move_selection_up();
    assert_eq!(state.selected_index, 0);
}

#[test]
#[serial]
fn test_file_selection_toggle() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    create_file(&dir, "test.txt", "content");

    let mut state = AppState::new().unwrap();

    if let Some(file) = state.files.get(0) {
        assert!(!file.selected);
    }

    // Toggle selection
    state.toggle_selected();

    if let Some(file) = state.files.get(0) {
        assert!(file.selected);
    }

    // Toggle again
    state.toggle_selected();

    if let Some(file) = state.files.get(0) {
        assert!(!file.selected);
    }
}

#[test]
#[serial]
fn test_commit_type_cycling() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    assert_eq!(state.commit_type, "feat");
    assert_eq!(state.commit_type_index, 0);

    state.cycle_commit_type();
    assert_eq!(state.commit_type, "fix");
    assert_eq!(state.commit_type_index, 1);

    state.cycle_commit_type();
    assert_eq!(state.commit_type, "build");
    assert_eq!(state.commit_type_index, 2);

    // Cycle once more to verify it continues working
    state.cycle_commit_type();
    assert_eq!(state.commit_type, "chore");
    assert_eq!(state.commit_type_index, 3);
}

#[test]
#[serial]
fn test_commit_form_field_navigation() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    assert_eq!(state.current_field, CommitFormField::Type);

    // Navigate forward with Tab
    state.next_field();
    assert_eq!(state.current_field, CommitFormField::Scope);

    state.next_field();
    assert_eq!(state.current_field, CommitFormField::ShortMessage);

    state.next_field();
    assert_eq!(state.current_field, CommitFormField::LongMessage);

    state.next_field();
    assert_eq!(state.current_field, CommitFormField::BreakingChange);

    // Should wrap to beginning
    state.next_field();
    assert_eq!(state.current_field, CommitFormField::Type);

    // Navigate backward with Shift+Tab
    state.prev_field();
    assert_eq!(state.current_field, CommitFormField::BreakingChange);

    state.prev_field();
    assert_eq!(state.current_field, CommitFormField::LongMessage);
}

#[test]
#[serial]
fn test_file_filter_cycling() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    assert_eq!(state.file_filter, FileFilter::All);

    state.cycle_filter();
    assert_eq!(state.file_filter, FileFilter::StagedOnly);

    state.cycle_filter();
    assert_eq!(state.file_filter, FileFilter::UnstagedOnly);

    state.cycle_filter();
    assert_eq!(state.file_filter, FileFilter::All);
}

#[test]
#[serial]
fn test_has_staged_files() {
    let (dir, repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    create_file(&dir, "test.txt", "content");

    let mut state = AppState::new().unwrap();

    // Initially no staged files
    assert!(!state.has_staged_files());

    // Stage a file manually
    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("test.txt")).unwrap();
    index.write().unwrap();

    // Reload state
    state = AppState::new().unwrap();

    // Now should have staged files
    assert!(state.has_staged_files());
}

#[test]
#[serial]
fn test_visible_files_with_filter() {
    let (dir, repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create and stage one file
    create_file(&dir, "staged.txt", "staged content");
    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("staged.txt")).unwrap();
    index.write().unwrap();

    // Create unstaged file
    create_file(&dir, "unstaged.txt", "unstaged content");

    let mut state = AppState::new().unwrap();

    // All files visible with All filter
    state.file_filter = FileFilter::All;
    let visible = state.visible_files();
    assert!(visible.len() >= 2);

    // Only staged files visible
    state.file_filter = FileFilter::StagedOnly;
    let visible = state.visible_files();
    assert!(visible.iter().all(|f| f.staged));

    // Only unstaged files visible
    state.file_filter = FileFilter::UnstagedOnly;
    let visible = state.visible_files();
    assert!(visible.iter().all(|f| !f.staged));
}

#[test]
#[serial]
fn test_folder_toggle() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();
    let folder = PathBuf::from("src");

    assert!(!state.collapsed_folders.contains(&folder));

    state.toggle_folder(folder.clone());
    assert!(state.collapsed_folders.contains(&folder));

    state.toggle_folder(folder.clone());
    assert!(!state.collapsed_folders.contains(&folder));
}

#[test]
#[serial]
fn test_file_group_suggestions() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create files of different types
    create_file(&dir, "README.md", "# Documentation");
    create_file(&dir, "src/lib.rs", "// Code");
    create_file(&dir, "tests/test.rs", "// Test");
    create_file(&dir, "Cargo.toml", "[package]");
    create_file(&dir, ".github/workflows/ci.yml", "# CI");

    let state = AppState::new().unwrap();

    // Check that files are grouped correctly
    for file in &state.files {
        match file.path.to_str().unwrap() {
            path if path.contains("README") => {
                assert_eq!(file.suggested_group, Some("docs".to_string()));
            }
            path if path.contains("test") => {
                assert_eq!(file.suggested_group, Some("tests".to_string()));
            }
            path if path.contains("Cargo.toml") => {
                assert_eq!(file.suggested_group, Some("deps".to_string()));
            }
            path if path.contains(".github") => {
                assert_eq!(file.suggested_group, Some("ci".to_string()));
            }
            _ => {}
        }
    }
}

#[test]
#[serial]
fn test_auto_grouping() {
    let (dir, repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create and stage various files
    create_file(&dir, "README.md", "docs");
    create_file(&dir, "src/main.rs", "code");
    create_file(&dir, "tests/test.rs", "test");

    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("README.md")).unwrap();
    index.add_path(&PathBuf::from("src/main.rs")).unwrap();
    index.add_path(&PathBuf::from("tests/test.rs")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();

    // Create auto groups
    state.create_auto_groups();

    // Should have created groups for docs, code, and tests
    assert!(!state.groups.is_empty());

    // Check that groups exist
    let group_names: Vec<&str> = state.groups.iter().map(|g| g.name.as_str()).collect();
    assert!(group_names.contains(&"docs"));
    assert!(group_names.contains(&"tests"));

    // Check commit types are appropriate
    for group in &state.groups {
        match group.name.as_str() {
            "docs" => assert_eq!(group.commit_type, "docs"),
            "tests" => assert_eq!(group.commit_type, "test"),
            _ => {}
        }
    }
}

#[test]
#[serial]
fn test_mode_transitions() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    assert_eq!(state.mode, AppMode::FileSelection);

    state.mode = AppMode::CommitMessage;
    assert_eq!(state.mode, AppMode::CommitMessage);

    state.mode = AppMode::GroupView;
    assert_eq!(state.mode, AppMode::GroupView);

    state.mode = AppMode::DiffView;
    assert_eq!(state.mode, AppMode::DiffView);

    state.mode = AppMode::Help;
    assert_eq!(state.mode, AppMode::Help);

    state.mode = AppMode::FileSelection;
    assert_eq!(state.mode, AppMode::FileSelection);
}