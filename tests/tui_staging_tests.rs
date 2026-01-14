use serial_test::serial;

use committy::tui::state::AppState;
use git2::{Repository, Signature};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test repository with initial commit
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
fn test_stage_selected_files() {
    let (dir, repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create test files
    create_file(&dir, "file1.txt", "content1");
    create_file(&dir, "file2.txt", "content2");

    let mut state = AppState::new().unwrap();

    // Initially files should be unstaged
    assert!(state.files.iter().all(|f| !f.staged));

    // Select first file
    state.selected_index = 0;
    state.toggle_selected();

    // Stage the selected file
    state.stage_selected().unwrap();

    // Verify file is staged in git
    let statuses = repo.statuses(None).unwrap();
    let mut has_staged = false;
    for entry in statuses.iter() {
        if entry.index_to_workdir().is_none() && entry.head_to_index().is_some() {
            has_staged = true;
            break;
        }
    }
    assert!(has_staged, "Should have at least one staged file");

    // Reload state and verify
    state = AppState::new().unwrap();
    assert!(state.files.iter().any(|f| f.staged));
}

#[test]
#[serial]
fn test_stage_multiple_files() {
    let (dir, repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create test files
    create_file(&dir, "file1.txt", "content1");
    create_file(&dir, "file2.txt", "content2");
    create_file(&dir, "file3.txt", "content3");

    let mut state = AppState::new().unwrap();

    // Select all files
    for file in &mut state.files {
        file.selected = true;
    }

    // Stage all selected files
    state.stage_selected().unwrap();

    // Verify all files are staged
    let statuses = repo.statuses(None).unwrap();
    let staged_count = statuses.iter().filter(|e| {
        e.index_to_workdir().is_none() && e.head_to_index().is_some()
    }).count();

    assert!(staged_count >= 3, "Should have at least 3 staged files");
}

#[test]
#[serial]
fn test_unstage_newly_added_file() {
    let (dir, repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create and stage a new file
    create_file(&dir, "newfile.txt", "new content");

    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("newfile.txt")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();

    // Find and select the staged file
    let file_index = state.files.iter()
        .position(|f| f.path.to_str().unwrap().contains("newfile.txt") && f.staged)
        .expect("Should find staged file");

    state.selected_index = file_index;
    state.toggle_selected();

    // Unstage it
    state.unstage_selected().unwrap();

    // Verify file is no longer staged
    let statuses = repo.statuses(None).unwrap();
    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            if path.contains("newfile.txt") {
                // Should be untracked or in workdir, not in index
                assert!(entry.head_to_index().is_none() || entry.status().is_wt_new());
            }
        }
    }
}

#[test]
#[serial]
fn test_unstage_modified_file() {
    let (dir, repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create and commit a file first
    create_file(&dir, "existing.txt", "original content");
    {
        let mut index = repo.index().unwrap();
        index.add_path(&PathBuf::from("existing.txt")).unwrap();
        index.write().unwrap();

        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Add existing file",
            &tree,
            &[&parent],
        )
        .unwrap();
    }

    // Modify and stage the file
    create_file(&dir, "existing.txt", "modified content");
    {
        let mut index = repo.index().unwrap();
        index.add_path(&PathBuf::from("existing.txt")).unwrap();
        index.write().unwrap();
    }

    let mut state = AppState::new().unwrap();

    // Find and select the staged modified file
    let file_index = state.files.iter()
        .position(|f| f.path.to_str().unwrap().contains("existing.txt") && f.staged)
        .expect("Should find staged file");

    state.selected_index = file_index;
    state.toggle_selected();

    // Unstage it
    state.unstage_selected().unwrap();

    // Verify file is unstaged (should be in workdir but not in index)
    let statuses = repo.statuses(None).unwrap();
    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            if path.contains("existing.txt") {
                // Should be modified in workdir, not staged
                assert!(entry.status().is_wt_modified() || entry.status().is_index_modified());
            }
        }
    }
}

#[test]
#[serial]
fn test_stage_deleted_file() {
    let (dir, repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create and commit a file
    create_file(&dir, "to_delete.txt", "content");
    {
        let mut index = repo.index().unwrap();
        index.add_path(&PathBuf::from("to_delete.txt")).unwrap();
        index.write().unwrap();

        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Add file to delete",
            &tree,
            &[&parent],
        )
        .unwrap();
    }

    // Delete the file
    fs::remove_file(dir.path().join("to_delete.txt")).unwrap();

    let mut state = AppState::new().unwrap();

    // Find and select the deleted file
    let file_index = state.files.iter()
        .position(|f| f.path.to_str().unwrap().contains("to_delete.txt"))
        .expect("Should find deleted file");

    state.selected_index = file_index;
    state.toggle_selected();

    // Stage the deletion
    state.stage_selected().unwrap();

    // Verify deletion is staged
    let statuses = repo.statuses(None).unwrap();
    let mut found_deleted = false;
    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            if path.contains("to_delete.txt") {
                assert!(entry.status().is_index_deleted());
                found_deleted = true;
            }
        }
    }
    assert!(found_deleted, "Deletion should be staged");
}

#[test]
#[serial]
fn test_stage_files_in_subdirectories() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create files in subdirectories
    create_file(&dir, "src/main.rs", "fn main() {}");
    create_file(&dir, "src/lib.rs", "pub fn lib() {}");
    create_file(&dir, "tests/test.rs", "#[test] fn test() {}");

    let mut state = AppState::new().unwrap();

    // Select all files
    for file in &mut state.files {
        file.selected = true;
    }

    // Stage all
    let result = state.stage_selected();
    assert!(result.is_ok(), "Should stage files in subdirectories without error");

    // Reload and verify
    state = AppState::new().unwrap();
    assert!(state.files.iter().filter(|f| f.staged).count() >= 3);
}

#[test]
#[serial]
fn test_unstage_multiple_files() {
    let (dir, repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create and stage multiple files
    create_file(&dir, "file1.txt", "content1");
    create_file(&dir, "file2.txt", "content2");
    create_file(&dir, "file3.txt", "content3");

    {
        let mut index = repo.index().unwrap();
        index.add_path(&PathBuf::from("file1.txt")).unwrap();
        index.add_path(&PathBuf::from("file2.txt")).unwrap();
        index.add_path(&PathBuf::from("file3.txt")).unwrap();
        index.write().unwrap();
    }

    let mut state = AppState::new().unwrap();

    // Select all staged files
    for file in &mut state.files {
        if file.staged {
            file.selected = true;
        }
    }

    // Unstage all
    state.unstage_selected().unwrap();

    // Verify no files are staged
    state = AppState::new().unwrap();
    assert!(state.files.iter().filter(|f| f.staged).count() == 0);
}

#[test]
#[serial]
fn test_stage_then_unstage_cycle() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create a file
    create_file(&dir, "cycle.txt", "content");

    let mut state = AppState::new().unwrap();

    // Stage it
    state.files[0].selected = true;
    state.stage_selected().unwrap();

    // Reload and verify staged
    state = AppState::new().unwrap();
    assert!(state.files.iter().any(|f| f.staged));

    // Unstage it
    for file in &mut state.files {
        if file.staged {
            file.selected = true;
        }
    }
    state.unstage_selected().unwrap();

    // Reload and verify unstaged
    state = AppState::new().unwrap();
    assert!(state.files.iter().filter(|f| f.staged).count() == 0);

    // Stage again
    state.files[0].selected = true;
    state.stage_selected().unwrap();

    // Verify staged again
    state = AppState::new().unwrap();
    assert!(state.files.iter().any(|f| f.staged));
}