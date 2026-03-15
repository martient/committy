use git2::{build::CheckoutBuilder, BranchType, Error as GitError, Reference};

use super::repository::{discover_repository, discover_repository_from};
use crate::error::CliError;
use std::path::Path;

#[allow(dead_code)]
pub fn create_branch(name: &str, force: bool) -> Result<(), CliError> {
    let repo = discover_repository()?;
    create_branch_in_repo(&repo, name, force)
}

pub fn create_branch_in(path: &Path, name: &str, force: bool) -> Result<(), CliError> {
    let repo = discover_repository_from(path)?;
    create_branch_in_repo(&repo, name, force)
}

fn create_branch_in_repo(repo: &git2::Repository, name: &str, force: bool) -> Result<(), CliError> {
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    repo.branch(name, &head_commit, force)?;
    Ok(())
}

#[allow(dead_code)]
pub fn branch_exists(name: &str) -> Result<bool, CliError> {
    let repo = discover_repository()?;
    branch_exists_in_repo(&repo, name)
}

pub fn branch_exists_in(path: &Path, name: &str) -> Result<bool, CliError> {
    let repo = discover_repository_from(path)?;
    branch_exists_in_repo(&repo, name)
}

fn branch_exists_in_repo(repo: &git2::Repository, name: &str) -> Result<bool, CliError> {
    let exists = repo.find_branch(name, BranchType::Local).is_ok();
    Ok(exists)
}

pub fn validate_branch_name(name: &str) -> Result<(), CliError> {
    let ref_name = format!("refs/heads/{name}");
    if Reference::is_valid_name(&ref_name) {
        Ok(())
    } else {
        Err(CliError::InputError(format!(
            "Invalid branch name '{name}'"
        )))
    }
}

#[allow(dead_code)]
pub fn checkout_branch(name: &str) -> Result<(), CliError> {
    let repo = discover_repository()?;
    checkout_branch_in_repo(&repo, name)
}

pub fn checkout_branch_in(path: &Path, name: &str) -> Result<(), CliError> {
    let repo = discover_repository_from(path)?;
    checkout_branch_in_repo(&repo, name)
}

fn checkout_branch_in_repo(repo: &git2::Repository, name: &str) -> Result<(), CliError> {
    let mut checkout_builder = CheckoutBuilder::default();

    // Find the branch reference
    let branch = repo
        .find_branch(name, BranchType::Local)
        .map_err(CliError::from)?;
    let branch_ref = branch
        .get()
        .name()
        .ok_or_else(|| CliError::from(GitError::from_str("Invalid branch ref name")))?;

    // Set HEAD to the branch
    repo.set_head(branch_ref).map_err(CliError::from)?;

    // Checkout working tree
    repo.checkout_head(Some(&mut checkout_builder))
        .map_err(CliError::from)?;

    Ok(())
}
