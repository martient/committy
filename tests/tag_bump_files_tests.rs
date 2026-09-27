mod common;

use git2::{Repository, Signature};
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// A repository on `master` with a Cargo.toml and one feature commit, so the tag
/// command has something to calculate a bump from.
fn setup_cargo_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.0.1\"\nedition = \"2021\"\n",
    )
    .unwrap();

    let signature = Signature::now("Test User", "test@example.com").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("Cargo.toml")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "feat: add the demo package",
        &tree,
        &[],
    )
    .unwrap();

    drop(tree);
    dir
}

fn cargo_version(dir: &Path) -> String {
    let content = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    content
        .lines()
        .find_map(|line| line.strip_prefix("version = "))
        .unwrap()
        .trim_matches('"')
        .to_string()
}

fn head_message(repo: &Repository) -> String {
    repo.head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .message()
        .unwrap()
        .trim()
        .to_string()
}

#[test]
fn tagging_bumps_version_files_by_default() {
    let dir = setup_cargo_repo();

    let mut cmd = common::committy_cmd();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag");
    cmd.assert().success();

    let repo = Repository::open(dir.path()).unwrap();
    let mut tags = Vec::new();
    repo.tag_foreach(|_, name| {
        tags.push(String::from_utf8_lossy(name).replace("refs/tags/", ""));
        true
    })
    .unwrap();
    assert_eq!(tags.len(), 1, "expected exactly one tag, got {tags:?}");
    let tag = &tags[0];

    // The version file carries the tagged version, not the stale one.
    assert_ne!(cargo_version(dir.path()), "0.0.1");
    assert_eq!(cargo_version(dir.path()), tag.trim_start_matches('v'));

    // And the bump is committed, so a build from the tag reports the new version.
    assert!(
        head_message(&repo).starts_with("chore: bump version to"),
        "expected a bump commit at HEAD, got {:?}",
        head_message(&repo)
    );
    let tagged_commit = repo
        .revparse_single(&format!("refs/tags/{tag}"))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    assert_eq!(
        tagged_commit.id(),
        repo.head().unwrap().peel_to_commit().unwrap().id(),
        "the tag must point at the bump commit"
    );
}

#[test]
fn no_bump_files_leaves_version_files_untouched() {
    let dir = setup_cargo_repo();

    let mut cmd = common::committy_cmd();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--no-bump-files");
    cmd.assert().success();

    assert_eq!(cargo_version(dir.path()), "0.0.1");

    let repo = Repository::open(dir.path()).unwrap();
    assert_eq!(head_message(&repo), "feat: add the demo package");
}

#[test]
fn bump_files_flag_is_still_accepted() {
    let dir = setup_cargo_repo();

    let mut cmd = common::committy_cmd();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--bump-files");
    cmd.assert().success();

    assert_ne!(cargo_version(dir.path()), "0.0.1");
}

#[test]
fn conflicting_bump_flags_are_rejected() {
    let dir = setup_cargo_repo();

    let mut cmd = common::committy_cmd();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag")
        .arg("--bump-files")
        .arg("--no-bump-files");
    cmd.assert().failure().stderr(predicate::str::contains(
        "Use either --bump-files or --no-bump-files, not both",
    ));

    assert_eq!(cargo_version(dir.path()), "0.0.1");
}

/// Multi-package repositories used to stage the bumped version files without
/// committing them, so the tags landed on a commit that still carried the old
/// versions — the same defect that shipped a v1.10.0 tarball reporting 1.9.1.
#[test]
fn multi_package_bump_is_committed_before_tagging() {
    let dir = tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    fs::create_dir_all(dir.path().join("alpha")).unwrap();
    fs::write(
        dir.path().join("alpha/Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.0.1\"\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join(".committy")).unwrap();
    fs::write(
        dir.path().join(".committy/config.toml"),
        r#"[repository]
name = "workspace"
type = "multi-package"

[versioning]
strategy = "independent"

[[packages]]
name = "alpha"
type = "rust-cargo"
path = "alpha"
version_file = "Cargo.toml"
version_field = "package.version"
"#,
    )
    .unwrap();

    let signature = Signature::now("Test User", "test@example.com").unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["."], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "feat(alpha): add the alpha package",
        &tree,
        &[],
    )
    .unwrap();
    drop(tree);

    let mut cmd = common::committy_cmd();
    cmd.current_dir(dir.path())
        .arg("--non-interactive")
        .arg("tag");
    cmd.assert().success();

    let repo = Repository::open(dir.path()).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert!(
        head_message(&repo).starts_with("chore: bump version to alpha@"),
        "expected a bump commit at HEAD, got {:?}",
        head_message(&repo)
    );

    // The bumped file is in the tagged tree, not just in the working directory.
    let mut tags = Vec::new();
    repo.tag_foreach(|_, name| {
        tags.push(String::from_utf8_lossy(name).replace("refs/tags/", ""));
        true
    })
    .unwrap();
    assert_eq!(tags.len(), 1, "expected exactly one tag, got {tags:?}");
    let tagged_commit = repo
        .revparse_single(&format!("refs/tags/{}", tags[0]))
        .unwrap()
        .peel_to_commit()
        .unwrap();
    assert_eq!(tagged_commit.id(), head.id());

    let version = tags[0].trim_start_matches("alpha-v").to_string();
    let blob = tagged_commit
        .tree()
        .unwrap()
        .get_path(Path::new("alpha/Cargo.toml"))
        .unwrap()
        .to_object(&repo)
        .unwrap();
    let content = String::from_utf8(blob.as_blob().unwrap().content().to_vec()).unwrap();
    assert!(
        content.contains(&format!("version = \"{version}\"")),
        "tagged tree still carries the old version: {content}"
    );
}
