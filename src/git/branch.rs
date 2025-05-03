use git2::{build::CheckoutBuilder, BranchType, Error as GitError};

use super::repository::discover_repository;
use crate::error::CliError;

pub fn create_branch(name: &str, force: bool) -> Result<(), CliError> {
    let repo = discover_repository()?;
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    repo.branch(name, &head_commit, force)?;
    Ok(())
}

pub fn checkout_branch(name: &str) -> Result<(), CliError> {
    let repo = discover_repository()?;
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
