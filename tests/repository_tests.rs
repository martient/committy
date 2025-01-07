use committy::error::CliError;
use committy::git::has_staged_changes;
use git2::Repository;
use std::env;
use std::fs;
use tempfile::TempDir;

fn setup_test_repo() -> (TempDir, Repository) {
    let temp_dir = TempDir::new().unwrap();
    let repo = Repository::init(temp_dir.path()).unwrap();

    // Create a basic git config
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    (temp_dir, repo)
}

#[test]
fn test_repository_discovery_from_subdirectory() -> Result<(), CliError> {
    let (temp_dir, repo) = setup_test_repo();

    // Create a subdirectory structure
    let subdir_path = temp_dir.path().join("src").join("deep").join("path");
    fs::create_dir_all(&subdir_path).unwrap();

    // Create and stage a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Stage the file
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("test.txt")).unwrap();
    index.write().unwrap();

    // Change to the deep subdirectory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&subdir_path).unwrap();

    // Verify we can still detect the repository and check staged changes
    let result = has_staged_changes();

    // Change back to the original directory
    env::set_current_dir(original_dir).unwrap();

    assert!(result.is_ok());
    assert!(result.unwrap(), "Expected staged changes to be detected");
    Ok(())
}

#[test]
fn test_repository_not_found() {
    // Create a temporary directory that is not a git repository
    let temp_dir = TempDir::new().unwrap();

    // Change to the temporary directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    // Verify we get an appropriate error when there's no git repository
    let result = has_staged_changes();

    // Change back to the original directory
    env::set_current_dir(original_dir).unwrap();

    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            CliError::GitError(_) => (),
            _ => panic!("Expected GitError, got different error type"),
        }
    }
}

#[test]
fn test_staged_deleted_file() -> Result<(), CliError> {
    let (temp_dir, repo) = setup_test_repo();

    // Create and commit a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Stage and commit the file
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("test.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let _commit = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

    // Verify the file exists in HEAD
    if let Ok(obj) = repo.revparse_single("HEAD") {
        if let Ok(commit) = obj.peel_to_commit() {
            let tree = commit.tree().unwrap();
            for entry in tree.iter() {
                println!("   - {}", entry.name().unwrap_or(""));
            }
        }
    }

    // Delete and stage the file
    fs::remove_file(&test_file).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(std::path::Path::new("test.txt")).unwrap();
    index.write().unwrap();

    // Check status flags
    let _statuses = repo.statuses(None).unwrap();

    // Verify that has_staged_changes detects the deleted file
    assert!(has_staged_changes()?);
    Ok(())
}

#[test]
fn test_no_staged_changes() -> Result<(), CliError> {
    let (temp_dir, repo) = setup_test_repo();

    // Create and commit a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Stage and commit the file
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("test.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )
    .unwrap();

    // Change to the repository directory to ensure we're in the right context
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    // Verify no staged changes are detected
    let result = has_staged_changes()?;
    assert!(
        !result,
        "Expected no staged changes, but changes were detected"
    );

    // Change back to the original directory
    env::set_current_dir(original_dir).unwrap();

    Ok(())
}

#[test]
fn test_unstaged_changes_only() -> Result<(), CliError> {
    let (temp_dir, repo) = setup_test_repo();

    // Create a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Change to the repository directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    // Verify no staged changes are detected
    let result = has_staged_changes()?;
    assert!(!result, "Expected no staged changes with only unstaged files");

    // Change back to the original directory
    env::set_current_dir(original_dir).unwrap();

    Ok(())
}
