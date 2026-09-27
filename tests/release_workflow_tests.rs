//! The release workflow is the last place a version mistake can be caught before
//! binaries reach users, so the guard itself is worth pinning down.

use std::fs;
use std::path::Path;

fn release_workflow() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/cargo-release.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn release_workflow_verifies_cargo_version_against_the_tag() {
    let workflow = release_workflow();

    assert!(
        workflow.contains("verify-version:"),
        "the release workflow must carry a job that compares Cargo.toml with the tag"
    );
    assert!(
        workflow.contains(r#"if [ "v$VERSION" != "$TAG" ]; then"#),
        "the guard must compare the Cargo.toml version with the tag name"
    );
}

#[test]
fn every_release_job_waits_for_the_version_guard() {
    let workflow = release_workflow();

    assert!(
        workflow.contains("needs: verify-version"),
        "create-release must not publish a release before the version guard passes"
    );
    assert!(
        workflow.contains("needs: [verify-version, create-release]"),
        "the build matrix must not upload assets before the version guard passes"
    );
}
