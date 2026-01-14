use committy::tui::state::{AppState, FileFilter};
use git2::{Repository, Signature};
use serial_test::serial;
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
fn create_file(_dir: &TempDir, path: &str, content: &str) {
    let file_path = _dir.path().join(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(file_path, content).unwrap();
}

#[test]
#[serial]
#[serial]
fn test_file_filter_initial_state() {
    let (_dir, _repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    let state = AppState::new().unwrap();

    assert_eq!(state.file_filter, FileFilter::All);
}

#[test]
#[serial]
fn test_file_filter_cycling() {
    let (_dir,_repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    assert_eq!(state.file_filter, FileFilter::All);

    state.cycle_filter();
    assert_eq!(state.file_filter, FileFilter::StagedOnly);

    state.cycle_filter();
    assert_eq!(state.file_filter, FileFilter::UnstagedOnly);

    state.cycle_filter();
    assert_eq!(state.file_filter, FileFilter::All);

    // Continue cycling
    state.cycle_filter();
    assert_eq!(state.file_filter, FileFilter::StagedOnly);
}

#[test]
#[serial]
fn test_visible_files_with_all_filter() {
    let (_dir,repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    // Create staged and unstaged files
    create_file(&_dir,"staged1.txt", "staged");
    create_file(&_dir,"staged2.txt", "staged");
    create_file(&_dir,"unstaged1.txt", "unstaged");
    create_file(&_dir,"unstaged2.txt", "unstaged");

    // Stage some files
    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("staged1.txt")).unwrap();
    index.add_path(&PathBuf::from("staged2.txt")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();
    state.file_filter = FileFilter::All;

    let visible = state.visible_files();

    // Should see all files
    assert!(visible.len() >= 4);
}

#[test]
#[serial]
fn test_visible_files_with_staged_filter() {
    let (_dir,repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    // Create staged and unstaged files
    create_file(&_dir,"staged.txt", "staged");
    create_file(&_dir,"unstaged.txt", "unstaged");

    // Stage one file
    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("staged.txt")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();
    state.file_filter = FileFilter::StagedOnly;

    let visible = state.visible_files();

    // Should only see staged files
    assert!(visible.iter().all(|f| f.staged));
    assert!(visible.len() >= 1);
}

#[test]
#[serial]
fn test_visible_files_with_unstaged_filter() {
    let (_dir,repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    // Create staged and unstaged files
    create_file(&_dir,"staged.txt", "staged");
    create_file(&_dir,"unstaged.txt", "unstaged");

    // Stage one file
    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("staged.txt")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();
    state.file_filter = FileFilter::UnstagedOnly;

    let visible = state.visible_files();

    // Should only see unstaged files
    assert!(visible.iter().all(|f| !f.staged));
    assert!(visible.len() >= 1);
}

#[test]
#[serial]
fn test_file_grouping_docs() {
    let (_dir,_repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    create_file(&_dir,"README.md", "# Documentation");
    create_file(&_dir,"docs/guide.md", "# Guide");
    create_file(&_dir,"CHANGELOG.md", "# Changelog");

    let state = AppState::new().unwrap();

    for file in &state.files {
        let path_str = file.path.to_str().unwrap();
        if path_str.contains("README") || path_str.contains("docs/") || path_str.ends_with(".md") {
            assert_eq!(file.suggested_group, Some("docs".to_string()));
        }
    }
}

#[test]
#[serial]
fn test_file_grouping_tests() {
    let (_dir,_repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    create_file(&_dir,"tests/test_main.rs", "#[test] fn test() {}");
    create_file(&_dir,"src/lib_test.rs", "#[test] fn test() {}");
    create_file(&_dir,"spec/test_spec.rb", "describe 'test'");

    let state = AppState::new().unwrap();

    for file in &state.files {
        let path_str = file.path.to_str().unwrap();
        if path_str.contains("test") || path_str.contains("spec") {
            assert_eq!(file.suggested_group, Some("tests".to_string()));
        }
    }
}

#[test]
#[serial]
fn test_file_grouping_ci() {
    let (_dir,_repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    create_file(&_dir,".github/workflows/ci.yml", "name: CI");
    create_file(&_dir,".gitlab-ci.yml", "stages:");
    create_file(&_dir,"ci/build.sh", "#!/bin/bash");

    let state = AppState::new().unwrap();

    for file in &state.files {
        let path_str = file.path.to_str().unwrap();
        if path_str.contains(".github") || path_str.contains(".gitlab") || path_str.contains("ci") {
            assert_eq!(file.suggested_group, Some("ci".to_string()));
        }
    }
}

#[test]
#[serial]
fn test_file_grouping_deps() {
    let (_dir,_repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    create_file(&_dir,"Cargo.toml", "[package]");
    create_file(&_dir,"package.json", "{}");
    create_file(&_dir,"requirements.txt", "requests==2.28.0");

    let state = AppState::new().unwrap();

    for file in &state.files {
        let path_str = file.path.to_str().unwrap();
        if path_str.contains("Cargo.toml") || path_str.contains("package.json") || path_str.contains("requirements.txt") {
            assert_eq!(file.suggested_group, Some("deps".to_string()));
        }
    }
}

#[test]
#[serial]
fn test_file_grouping_build() {
    let (_dir,_repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    create_file(&_dir,"Makefile", "all:");
    create_file(&_dir,"build.rs", "fn main() {}");
    create_file(&_dir,"webpack.config.js", "module.exports = {}");

    let state = AppState::new().unwrap();

    for file in &state.files {
        let path_str = file.path.to_str().unwrap();
        if path_str.contains("Makefile") || path_str.contains("build.rs") || path_str.contains("webpack") {
            assert_eq!(file.suggested_group, Some("build".to_string()));
        }
    }
}

#[test]
#[serial]
fn test_auto_grouping_creates_groups() {
    let (_dir, repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    // Create and stage various types of files
    create_file(&_dir, "README.md", "docs");
    create_file(&_dir, "src/main.rs", "code");
    create_file(&_dir, "tests/test.rs", "tests");
    create_file(&_dir, "Cargo.toml", "deps");

    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("README.md")).unwrap();
    index.add_path(&PathBuf::from("src/main.rs")).unwrap();
    index.add_path(&PathBuf::from("tests/test.rs")).unwrap();
    index.add_path(&PathBuf::from("Cargo.toml")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();

    assert_eq!(state.groups.len(), 0);

    state.create_auto_groups();

    // Should have created multiple groups
    assert!(state.groups.len() > 0);

    // Check that each group has files
    for group in &state.groups {
        assert!(!group.files.is_empty());
    }
}

#[test]
#[serial]
fn test_auto_grouping_assigns_correct_commit_types() {
    let (_dir,repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    // Create and stage files
    create_file(&_dir,"README.md", "docs");
    create_file(&_dir,"tests/test.rs", "tests");
    create_file(&_dir,".github/workflows/ci.yml", "ci");
    create_file(&_dir,"Cargo.toml", "deps");

    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("README.md")).unwrap();
    index.add_path(&PathBuf::from("tests/test.rs")).unwrap();
    index.add_path(&PathBuf::from(".github/workflows/ci.yml")).unwrap();
    index.add_path(&PathBuf::from("Cargo.toml")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();
    state.create_auto_groups();

    // Verify commit types match groups
    for group in &state.groups {
        match group.name.as_str() {
            "docs" => assert_eq!(group.commit_type, "docs"),
            "tests" => assert_eq!(group.commit_type, "test"),
            "ci" => assert_eq!(group.commit_type, "ci"),
            "deps" => assert_eq!(group.commit_type, "build"),
            "build" => assert_eq!(group.commit_type, "build"),
            _ => assert_eq!(group.commit_type, "feat"), // default
        }
    }
}

#[test]
#[serial]
fn test_auto_grouping_only_staged_files() {
    let (_dir,repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    // Create staged and unstaged files
    create_file(&_dir,"staged.md", "staged docs");
    create_file(&_dir,"unstaged.md", "unstaged docs");

    // Only stage one file
    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("staged.md")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();
    state.create_auto_groups();

    // Groups should only contain staged files
    for group in &state.groups {
        for file_path in &group.files {
            let file = state.files.iter().find(|f| &f.path == file_path);
            if let Some(f) = file {
                assert!(f.staged, "Group should only contain staged files");
            }
        }
    }
}

#[test]
#[serial]
fn test_auto_grouping_empty_when_no_staged_files() {
    let (_dir,_repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    // Create files but don't stage them
    create_file(&_dir,"unstaged1.txt", "content");
    create_file(&_dir,"unstaged2.txt", "content");

    let mut state = AppState::new().unwrap();
    state.create_auto_groups();

    // Should have no groups since no files are staged
    assert_eq!(state.groups.len(), 0);
}

#[test]
#[serial]
fn test_filter_changes_visible_files() {
    let (_dir,repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    // Create and stage some files
    create_file(&_dir,"staged1.txt", "staged");
    create_file(&_dir,"staged2.txt", "staged");
    create_file(&_dir,"unstaged1.txt", "unstaged");
    create_file(&_dir,"unstaged2.txt", "unstaged");

    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("staged1.txt")).unwrap();
    index.add_path(&PathBuf::from("staged2.txt")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();

    // Test All filter
    state.file_filter = FileFilter::All;
    let all_count = state.visible_files().len();

    // Test StagedOnly filter
    state.file_filter = FileFilter::StagedOnly;
    let staged_count = state.visible_files().len();

    // Test UnstagedOnly filter
    state.file_filter = FileFilter::UnstagedOnly;
    let unstaged_count = state.visible_files().len();

    // Verify counts make sense
    assert!(all_count >= staged_count + unstaged_count);
    assert!(staged_count > 0);
    assert!(unstaged_count > 0);
}

#[test]
#[serial]
fn test_grouping_with_mixed_file_types() {
    let (_dir,repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    // Create a mix of file types
    create_file(&_dir,"README.md", "docs");
    create_file(&_dir,"src/main.rs", "code");
    create_file(&_dir,"src/lib.rs", "code");
    create_file(&_dir,"tests/test1.rs", "test");
    create_file(&_dir,"tests/test2.rs", "test");
    create_file(&_dir,"Cargo.toml", "deps");
    create_file(&_dir,".github/workflows/ci.yml", "ci");

    // Stage all
    let mut index = repo.index().unwrap();
    index.add_path(&PathBuf::from("README.md")).unwrap();
    index.add_path(&PathBuf::from("src/main.rs")).unwrap();
    index.add_path(&PathBuf::from("src/lib.rs")).unwrap();
    index.add_path(&PathBuf::from("tests/test1.rs")).unwrap();
    index.add_path(&PathBuf::from("tests/test2.rs")).unwrap();
    index.add_path(&PathBuf::from("Cargo.toml")).unwrap();
    index.add_path(&PathBuf::from(".github/workflows/ci.yml")).unwrap();
    index.write().unwrap();

    let mut state = AppState::new().unwrap();
    state.create_auto_groups();

    // Should have multiple groups
    assert!(state.groups.len() >= 4); // docs, tests, deps, ci at minimum

    // Verify each group has the right number of files
    for group in &state.groups {
        match group.name.as_str() {
            "tests" => assert!(group.files.len() >= 2),
            _ => {}
        }
    }
}

#[test]
#[serial]
fn test_folder_collapse_toggle() {
    let (_dir,_repo) = setup_test_repo();
    std::env::set_current_dir(_dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    let folder1 = PathBuf::from("src");
    let folder2 = PathBuf::from("tests");

    // Initially no folders collapsed
    assert!(!state.collapsed_folders.contains(&folder1));
    assert!(!state.collapsed_folders.contains(&folder2));

    // Collapse folder1
    state.toggle_folder(folder1.clone());
    assert!(state.collapsed_folders.contains(&folder1));
    assert!(!state.collapsed_folders.contains(&folder2));

    // Collapse folder2
    state.toggle_folder(folder2.clone());
    assert!(state.collapsed_folders.contains(&folder1));
    assert!(state.collapsed_folders.contains(&folder2));

    // Expand folder1
    state.toggle_folder(folder1.clone());
    assert!(!state.collapsed_folders.contains(&folder1));
    assert!(state.collapsed_folders.contains(&folder2));

    // Expand folder2
    state.toggle_folder(folder2.clone());
    assert!(!state.collapsed_folders.contains(&folder1));
    assert!(!state.collapsed_folders.contains(&folder2));
}