use crate::error::CliError;
use git2::{Repository, Status};

pub fn has_staged_changes() -> Result<bool, CliError> {
    let repo = Repository::open(".")?;
    let statuses = repo.statuses(None)?;

    Ok(statuses.iter().any(|status| {
        status.status().intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        )
    }))
}
