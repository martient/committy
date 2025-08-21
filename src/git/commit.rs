use super::repository::discover_repository;
use crate::error::CliError;

pub fn commit_changes(message: &str, amend: bool) -> Result<(), CliError> {
    let repo = discover_repository()?;
    let signature = repo.signature()?;
    let mut index = repo.index()?;
    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;

    if amend {
        let head = repo.head()?;
        let parent_commit = head.peel_to_commit()?;
        parent_commit.amend(
            Some("HEAD"),
            Some(&signature),
            Some(&signature),
            None,
            Some(message),
            Some(&tree),
        )?;
    } else {
        let parents = match repo.head() {
            Ok(head) => {
                let commit = head.peel_to_commit()?;
                vec![commit]
            }
            Err(_) => vec![],
        };

        let parents_refs: Vec<&git2::Commit> = parents.iter().collect();

        // Create the commit
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents_refs,
        )?;
    }

    Ok(())
}

pub fn format_commit_message(
    commit_type: &str,
    breaking_change: bool,
    scope: &str,
    short_message: &str,
    long_message: &str,
) -> String {
    let bang = if breaking_change { "!" } else { "" };
    let mut full_message = if scope.is_empty() {
        format!("{commit_type}{bang}: {short_message}")
    } else {
        format!("{commit_type}({scope}){bang}: {short_message}")
    };

    if !long_message.is_empty() {
        full_message = format!("{full_message}\n\n{long_message}");
    }

    full_message
}
