use git2::{Repository, Signature};
use std::fs;
use tempfile::tempdir;

fn setup_repo_with_commits(
    branch_name: &str,
    commits: Vec<&str>,
) -> (tempfile::TempDir, Repository) {
    let dir = tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    // Configure test user
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    let signature = Signature::now("Test User", "test@example.com").unwrap();

    // Create initial commit
    let initial_commit = {
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap()
    };

    // Create and checkout the specified branch
    let initial_commit_obj = repo.find_commit(initial_commit).unwrap();
    repo.branch(branch_name, &initial_commit_obj, false)
        .unwrap();
    repo.set_head(&format!("refs/heads/{}", branch_name))
        .unwrap();

    // Add commits
    let mut parent = initial_commit_obj;
    for (i, commit_msg) in commits.iter().enumerate() {
        let file_path = dir.path().join(format!("file{}.txt", i));
        fs::write(&file_path, format!("content {}", i)).unwrap();

        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new(&format!("file{}.txt", i)))
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let commit_id = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                commit_msg,
                &tree,
                &[&parent],
            )
            .unwrap();
        parent = repo.find_commit(commit_id).unwrap();
    }

    drop(parent); // Drop to release borrow

    (dir, repo)
}

#[test]
fn test_beta_increments_counter_not_version() {
    // Scenario: v1.0.0 on main, v1.2.0-beta.1 on beta
    // New feat commit on beta should create v1.2.0-beta.2, NOT v1.3.0-beta.0
    let (dir, repo) = setup_repo_with_commits("develop", vec!["feat: initial feature"]);
    let signature = Signature::now("Test User", "test@example.com").unwrap();

    // Create v1.0.0 tag on initial commit
    let initial_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag(
        "v1.0.0",
        initial_commit.as_object(),
        &signature,
        "Release v1.0.0",
        false,
    )
    .unwrap();

    // Create v1.2.0-beta.1 tag on the same commit
    repo.tag(
        "v1.2.0-beta.1",
        initial_commit.as_object(),
        &signature,
        "Beta release v1.2.0-beta.1",
        false,
    )
    .unwrap();

    // Add another feature commit
    let file_path = dir.path().join("another.txt");
    fs::write(&file_path, "another feature").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("another.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "feat: another feature",
        &tree,
        &[&parent],
    )
    .unwrap();

    // Run tag command in prerelease mode
    let mut cmd = assert_cmd::Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--prerelease")
        .arg("--dry-run")
        .arg("--no-fetch")
        .arg("--output")
        .arg("json");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    println!("Output: {}", stdout);

    // Should produce v1.2.0-beta.2, not v1.3.0-beta.0
    assert!(
        stdout.contains("v1.2.0-beta.2"),
        "Expected v1.2.0-beta.2 but got: {}",
        stdout
    );
    assert!(
        !stdout.contains("v1.3.0"),
        "Should NOT bump to v1.3.0, output: {}",
        stdout
    );
}

#[test]
fn test_beta_with_breaking_change_increments_counter() {
    // Scenario: v1.0.0 on main, v2.0.0-beta.1 on beta
    // Another breaking change should create v2.0.0-beta.2, NOT v3.0.0-beta.0
    let (dir, repo) = setup_repo_with_commits("develop", vec!["feat!: breaking change"]);
    let signature = Signature::now("Test User", "test@example.com").unwrap();

    let initial_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag(
        "v1.0.0",
        initial_commit.as_object(),
        &signature,
        "Release v1.0.0",
        false,
    )
    .unwrap();

    repo.tag(
        "v2.0.0-beta.1",
        initial_commit.as_object(),
        &signature,
        "Beta release v2.0.0-beta.1",
        false,
    )
    .unwrap();

    // Add another breaking change
    let file_path = dir.path().join("breaking.txt");
    fs::write(&file_path, "breaking change").unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_path(std::path::Path::new("breaking.txt"))
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "feat!: another breaking change",
        &tree,
        &[&parent],
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--prerelease")
        .arg("--dry-run")
        .arg("--no-fetch")
        .arg("--output")
        .arg("json");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("v2.0.0-beta.2"),
        "Expected v2.0.0-beta.2 but got: {}",
        stdout
    );
    assert!(
        !stdout.contains("v3.0.0"),
        "Should NOT bump to v3.0.0, output: {}",
        stdout
    );
}

#[test]
fn test_first_beta_after_main_applies_bump() {
    // Scenario: v1.0.0 on main, no beta tags
    // feat commit on beta branch should create v1.1.0-beta.0
    let (dir, repo) = setup_repo_with_commits("develop", vec![]);
    let signature = Signature::now("Test User", "test@example.com").unwrap();

    let initial_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag(
        "v1.0.0",
        initial_commit.as_object(),
        &signature,
        "Release v1.0.0",
        false,
    )
    .unwrap();

    // Add a feature commit
    let file_path = dir.path().join("feature.txt");
    fs::write(&file_path, "new feature").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("feature.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "feat: new feature",
        &tree,
        &[&parent],
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--prerelease")
        .arg("--dry-run")
        .arg("--no-fetch")
        .arg("--output")
        .arg("json");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("v1.1.0-beta.0"),
        "Expected v1.1.0-beta.0 but got: {}",
        stdout
    );
}

#[test]
fn test_multiple_patches_on_beta() {
    // Scenario: v1.0.0 on main, v1.0.1-beta.1 on beta
    // Another fix should create v1.0.1-beta.2, NOT v1.0.2-beta.0
    let (dir, repo) = setup_repo_with_commits("develop", vec!["fix: bug fix"]);
    let signature = Signature::now("Test User", "test@example.com").unwrap();

    let initial_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag(
        "v1.0.0",
        initial_commit.as_object(),
        &signature,
        "Release v1.0.0",
        false,
    )
    .unwrap();

    repo.tag(
        "v1.0.1-beta.1",
        initial_commit.as_object(),
        &signature,
        "Beta release v1.0.1-beta.1",
        false,
    )
    .unwrap();

    // Add another fix
    let file_path = dir.path().join("fix.txt");
    fs::write(&file_path, "another fix").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("fix.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "fix: another fix",
        &tree,
        &[&parent],
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--prerelease")
        .arg("--dry-run")
        .arg("--no-fetch")
        .arg("--output")
        .arg("json");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("v1.0.1-beta.2"),
        "Expected v1.0.1-beta.2 but got: {}",
        stdout
    );
    assert!(
        !stdout.contains("v1.0.2"),
        "Should NOT bump to v1.0.2, output: {}",
        stdout
    );
}

#[test]
fn test_beta_catches_up_to_main() {
    // Scenario: v2.0.0 on main, v1.5.0-beta.3 on beta
    // feat commit on beta should create v2.1.0-beta.0 (catches up and bumps)
    let (dir, repo) = setup_repo_with_commits("develop", vec![]);
    let signature = Signature::now("Test User", "test@example.com").unwrap();

    let initial_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag(
        "v2.0.0",
        initial_commit.as_object(),
        &signature,
        "Release v2.0.0",
        false,
    )
    .unwrap();

    repo.tag(
        "v1.5.0-beta.3",
        initial_commit.as_object(),
        &signature,
        "Beta release v1.5.0-beta.3",
        false,
    )
    .unwrap();

    // Add a feature commit
    let file_path = dir.path().join("feature.txt");
    fs::write(&file_path, "new feature").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("feature.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "feat: new feature",
        &tree,
        &[&parent],
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--prerelease")
        .arg("--dry-run")
        .arg("--no-fetch")
        .arg("--output")
        .arg("json");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("v2.1.0-beta.0"),
        "Expected v2.1.0-beta.0 but got: {}",
        stdout
    );
}

#[test]
fn test_chore_on_beta_increments_counter() {
    // Scenario: v1.0.0 on main, v1.1.0-beta.1 on beta
    // Chore commit (patch) should create v1.1.0-beta.2, NOT v1.1.1-beta.0
    let (dir, repo) = setup_repo_with_commits("develop", vec!["feat: feature"]);
    let signature = Signature::now("Test User", "test@example.com").unwrap();

    let initial_commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag(
        "v1.0.0",
        initial_commit.as_object(),
        &signature,
        "Release v1.0.0",
        false,
    )
    .unwrap();

    repo.tag(
        "v1.1.0-beta.1",
        initial_commit.as_object(),
        &signature,
        "Beta release v1.1.0-beta.1",
        false,
    )
    .unwrap();

    // Add a chore commit
    let file_path = dir.path().join("chore.txt");
    fs::write(&file_path, "chore work").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("chore.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "chore: update dependencies",
        &tree,
        &[&parent],
    )
    .unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("committy").unwrap();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--prerelease")
        .arg("--dry-run")
        .arg("--no-fetch")
        .arg("--output")
        .arg("json");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("v1.1.0-beta.2"),
        "Expected v1.1.0-beta.2 but got: {}",
        stdout
    );
    assert!(
        !stdout.contains("v1.1.1"),
        "Should NOT bump to v1.1.1, output: {}",
        stdout
    );
}
