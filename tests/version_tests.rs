mod common;

use committy::git::{TagGenerator, TagGeneratorOptions};
use committy::version::VersionManager;
use std::fs;
use structopt::StructOpt;
use tempfile::tempdir;

fn setup_version_files() -> tempfile::TempDir {
    let dir = tempdir().expect("Failed to create temp directory");

    // Create test version files
    let cargo_toml = dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "test-package"
version = "1.0.0"
"#,
    )
    .expect("Failed to write Cargo.toml");

    let package_json = dir.path().join("package.json");
    fs::write(
        &package_json,
        r#"{
  "name": "test-package",
  "version": "1.0.0"
}"#,
    )
    .expect("Failed to write package.json");

    dir
}

#[test]
fn test_version_update() {
    let temp_dir = setup_version_files();
    let mut version_manager = VersionManager::new();

    // Add version files with their patterns
    version_manager
        .add_version_file(
            temp_dir.path().join("Cargo.toml"),
            r#"version\s*=\s*"[^"]*""#,
            r#"version = "{}""#,
        )
        .expect("Failed to add Cargo.toml");

    version_manager
        .add_version_file(
            temp_dir.path().join("package.json"),
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )
        .expect("Failed to add package.json");

    // Update versions
    let updated_files = version_manager
        .update_all_versions("2.0.0")
        .expect("Failed to update versions");

    assert_eq!(updated_files.len(), 2);

    // Verify Cargo.toml
    let cargo_contents =
        fs::read_to_string(temp_dir.path().join("Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(cargo_contents.contains("version = \"2.0.0\""));

    // Verify package.json
    let package_contents = fs::read_to_string(temp_dir.path().join("package.json"))
        .expect("Failed to read package.json");
    assert!(package_contents.contains("\"version\": \"2.0.0\""));
}

#[test]
fn test_nonexistent_file() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let mut version_manager = VersionManager::new();

    // Add a non-existent file
    version_manager
        .add_version_file(
            temp_dir.path().join("nonexistent.json"),
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )
        .expect("Failed to add nonexistent file");

    // Update should succeed but not update any files
    let updated_files = version_manager
        .update_all_versions("1.0.0")
        .expect("Failed to handle nonexistent file");
    assert_eq!(updated_files.len(), 0);
}

#[test]
fn test_invalid_version_pattern() {
    let temp_dir = tempdir().expect("Failed to create temp directory");

    // Create file with different version format
    let invalid_file = temp_dir.path().join("invalid.toml");
    fs::write(
        &invalid_file,
        r#"[package]
name = "test-package"
version = v1.0.0
"#,
    )
    .expect("Failed to write invalid.toml");

    let mut version_manager = VersionManager::new();
    version_manager
        .add_version_file(
            &invalid_file,
            r#"version\s*=\s*"[^"]*""#, // This pattern won't match because version isn't in quotes
            r#"version = "{}""#,
        )
        .expect("Failed to add invalid file");

    // Update should succeed but file should remain unchanged
    version_manager
        .update_all_versions("2.0.0")
        .expect("Failed to handle invalid pattern");

    // Verify file wasn't changed
    let contents = fs::read_to_string(&invalid_file).expect("Failed to read invalid.toml");
    assert!(contents.contains("version = v1.0.0"));
}

#[test]
fn test_multiple_version_patterns() {
    let temp_dir = tempdir().expect("Failed to create temp directory");

    // Create file with multiple version occurrences
    let multi_version_file = temp_dir.path().join("multi.toml");
    fs::write(
        &multi_version_file,
        r#"[package]
name = "test-package"
version = "1.0.0"
other_version = "2.0.0"
"#,
    )
    .expect("Failed to write multi.toml");

    let mut version_manager = VersionManager::new();
    version_manager
        .add_version_file(
            &multi_version_file,
            r#"(?m)^\s*version\s*=\s*"[^"]*""#, // Add (?m)^ to match only at line start
            r#"version = "{}""#,
        )
        .expect("Failed to add multi-version file");

    version_manager
        .update_all_versions("1.1.0")
        .expect("Failed to update version");

    // Verify only the correct version was updated
    let contents = fs::read_to_string(&multi_version_file).expect("Failed to read multi.toml");
    assert!(contents.contains("version = \"1.1.0\""));
    assert!(contents.contains("other_version = \"2.0.0\""));
}

#[test]
fn test_register_common_files() {
    let temp_dir = tempdir().expect("Failed to create temp directory");

    // Create common version files
    let files = vec![
        (
            "Cargo.toml",
            r#"[package]
name = "test"
version = "1.0.0""#,
        ),
        (
            "package.json",
            r#"{
  "name": "test",
  "version": "1.0.0"
}"#,
        ),
        (
            "pyproject.toml",
            r#"[project]
name = "test"
version = "1.0.0""#,
        ),
    ];

    for (name, content) in files {
        let file_path = temp_dir.path().join(name);
        fs::write(&file_path, content).unwrap_or_else(|_| panic!("Failed to write {}", name));
    }

    // Register and update all common files
    let mut version_manager = VersionManager::new();

    // Add each file with absolute path
    version_manager
        .add_version_file(
            temp_dir.path().join("Cargo.toml"),
            r#"version\s*=\s*"[^"]*""#,
            r#"version = "{}""#,
        )
        .expect("Failed to add Cargo.toml");

    version_manager
        .add_version_file(
            temp_dir.path().join("package.json"),
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )
        .expect("Failed to add package.json");

    version_manager
        .add_version_file(
            temp_dir.path().join("pyproject.toml"),
            r#"version\s*=\s*"[^"]*""#,
            r#"version = "{}""#,
        )
        .expect("Failed to add pyproject.toml");

    // Update all versions
    version_manager
        .update_all_versions("2.0.0")
        .expect("Failed to update versions");

    // Verify updates
    let cargo_toml =
        fs::read_to_string(temp_dir.path().join("Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(cargo_toml.contains("version = \"2.0.0\""));

    let package_json = fs::read_to_string(temp_dir.path().join("package.json"))
        .expect("Failed to read package.json");
    assert!(package_json.contains("\"version\": \"2.0.0\""));

    let pyproject_toml = fs::read_to_string(temp_dir.path().join("pyproject.toml"))
        .expect("Failed to read pyproject.toml");
    assert!(pyproject_toml.contains("version = \"2.0.0\""));
}

#[test]
fn test_version_with_v_prefix() {
    let temp_dir = setup_version_files();
    let mut version_manager = VersionManager::new();

    version_manager
        .add_version_file(
            temp_dir.path().join("Cargo.toml"),
            r#"version\s*=\s*"[^"]*""#,
            r#"version = "{}""#,
        )
        .expect("Failed to add Cargo.toml");

    // Update with v-prefixed version
    let updated_files = version_manager
        .update_all_versions("v2.0.0")
        .expect("Failed to update versions");

    assert_eq!(updated_files.len(), 1);

    // Verify the v prefix was stripped
    let cargo_contents =
        fs::read_to_string(temp_dir.path().join("Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(cargo_contents.contains("version = \"2.0.0\""));
}

#[test]
fn test_file_with_multiple_versions() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let test_file = temp_dir.path().join("test.json");

    // Create a file with multiple version fields
    fs::write(
        &test_file,
        r#"{
  "version": "1.0.0",
  "dependencies": {
    "some-pkg": {
      "version": "1.0.0"
    }
  }
}"#,
    )
    .expect("Failed to write test file");

    let mut version_manager = VersionManager::new();
    version_manager
        .add_version_file(
            test_file.clone(),
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )
        .expect("Failed to add test file");

    let updated_files = version_manager
        .update_all_versions("2.0.0")
        .expect("Failed to update versions");

    assert_eq!(updated_files.len(), 1);

    // Verify both versions were updated
    let contents = fs::read_to_string(test_file).expect("Failed to read test file");
    let version_count = contents.matches("\"version\": \"2.0.0\"").count();
    assert_eq!(version_count, 2, "Both version fields should be updated");
}

#[test]
fn test_version_bump_with_commit() {
    // Create temporary directory
    let temp_path = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_path.path();

    // Initialize git repository
    let repo = git2::Repository::init(temp_path).expect("Failed to init repo");
    let sig =
        git2::Signature::now("Test User", "test@example.com").expect("Failed to create signature");

    // Set up git config
    let mut config = repo.config().expect("Failed to get config");
    config
        .set_str("user.name", "Test User")
        .expect("Failed to set user.name");
    config
        .set_str("user.email", "test@example.com")
        .expect("Failed to set user.email");

    // Create version files
    let cargo_toml = temp_path.join("Cargo.toml");
    let package_json = temp_path.join("package.json");

    std::fs::write(
        &cargo_toml,
        r#"[package]
name = "test-package"
version = "1.0.0"
"#,
    )
    .expect("Failed to write Cargo.toml");

    std::fs::write(
        &package_json,
        r#"{
  "name": "test-package",
  "version": "1.0.0"
}"#,
    )
    .expect("Failed to write package.json");

    // Add files to git
    let mut index = repo.index().expect("Failed to get index");
    index
        .add_path(std::path::Path::new("Cargo.toml"))
        .expect("Failed to add Cargo.toml");
    index
        .add_path(std::path::Path::new("package.json"))
        .expect("Failed to add package.json");
    let tree_id = index.write_tree().expect("Failed to write tree");
    let tree = repo.find_tree(tree_id).expect("Failed to find tree");

    // Create initial commit
    let initial_commit = repo
        .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .expect("Failed to create initial commit");

    // Create and checkout main branch
    repo.branch("main", &repo.find_commit(initial_commit).unwrap(), false)
        .expect("Failed to create main branch");
    repo.set_head("refs/heads/main")
        .expect("Failed to set HEAD to main");

    // Register version files
    let mut version_manager = VersionManager::new();
    version_manager
        .add_version_file(&cargo_toml, r#"version\s*=\s*"[^"]*""#, r#"version = "{}""#)
        .expect("Failed to add Cargo.toml");

    version_manager
        .add_version_file(
            &package_json,
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )
        .expect("Failed to add package.json");

    // Create tag generator with version bump enabled
    let options = TagGeneratorOptions::from_iter_safe(&[
        "test",
        "--default-bump",
        "minor",
        "--source",
        &temp_path.to_string_lossy(),
        "--force-without-change",
        "--not-publish",
        "--release-branches",
        "main,master",
        "--initial-version",
        "1.0.0",
        "--prerelease-suffix",
        "beta",
        "--none-string-token",
        "#none",
    ])
    .expect("Failed to create options");

    let tag_generator = TagGenerator::new(options, true);

    // Run the tag generator
    tag_generator.run().expect("Failed to run tag generator");

    // Verify version files were updated
    let cargo_toml_content =
        std::fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml");
    let package_json_content =
        std::fs::read_to_string(&package_json).expect("Failed to read package.json");

    assert!(
        cargo_toml_content.contains(r#"version = "1.1.0""#),
        "Cargo.toml version not updated"
    );
    assert!(
        package_json_content.contains(r#""version": "1.1.0""#),
        "package.json version not updated"
    );

    // Verify commit was created
    let head_commit = repo
        .head()
        .expect("Failed to get HEAD")
        .peel_to_commit()
        .expect("Failed to get HEAD commit");
    let commit_message = head_commit.message().unwrap_or("");
    println!("Actual commit message: {}", commit_message);
    assert!(
        commit_message.contains("chore: bump version to 1.1.0"),
        "Commit message incorrect"
    );

    // Verify tag was created and points to the version bump commit
    let tag_ref = repo
        .find_reference("refs/tags/v1.1.0")
        .expect("Failed to find tag reference");
    let tag_commit = tag_ref
        .peel_to_commit()
        .expect("Failed to get tag's commit");
    assert_eq!(
        tag_commit.id(),
        head_commit.id(),
        "Tag does not point to version bump commit"
    );
}
