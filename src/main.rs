use clap::{Arg, Command};
use dialoguer::{Input, Select};
use git2::{DiffOptions, ErrorCode, Repository};
use std::process::exit;

fn main() -> Result<(), git2::Error> {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("short-commit")
                .short('s')
                .long("short-commit")
                .help("Short commit message")
                .value_name("SHORT-COMMIT"),
        )
        .subcommand(Command::new("amend").about("Amend the previous commit"))
        .get_matches();

    let amend: bool = matches.subcommand_matches("amend").is_some();
    let short_commit: Option<String> = matches.get_one::<String>("short-commit").cloned();

    if !amend {
        match has_staged_changes(".") {
            Ok(true) => {}
            Ok(false) => {
                println!("No staged changes found.");
                exit(1);
            }
            Err(e) => {
                println!("Error: {}", e);
                exit(1);
            }
        }
    }

    let commit_type = select_commit_type();
    let breaking_change = confirm_breaking_change();
    let scope: String = input_scope();
    let short_message = if short_commit.as_ref().map_or(true, |s| s.len() < 1) {
        input_short_message()
    } else {
        short_commit.unwrap()
    };
    let long_message = input_long_message();

    let mut full_message = if scope.is_empty() {
        format!(
            "{}{}: {}",
            commit_type,
            if breaking_change { "!" } else { "" },
            short_message
        )
    } else {
        format!(
            "{}({}){}: {}",
            commit_type,
            scope,
            if breaking_change { "!" } else { "" },
            short_message
        )
    };

    if !long_message.is_empty() {
        full_message = format!("{}\n\n{}", full_message, long_message);
    }

    if let Err(e) = commit_changes(&full_message, amend) {
        eprintln!("Error committing changes: {}", e);
        exit(1);
    }
    Ok(())
}

fn has_staged_changes(repo_path: &str) -> Result<bool, git2::Error> {
    let repo = Repository::open(repo_path)?;

    let index = repo.index()?;

    let head_commit_tree = match repo.head() {
        Ok(head) => {
            let commit = head.peel_to_commit()?;
            Some(commit.tree()?)
        }
        Err(ref e) if e.code() == ErrorCode::UnbornBranch || e.code() == ErrorCode::NotFound => {
            None
        }
        Err(e) => return Err(e),
    };

    let mut diff_options = DiffOptions::new();
    diff_options.include_untracked(true);
    diff_options.recurse_untracked_dirs(true);

    let diff = repo.diff_tree_to_index(
        head_commit_tree.as_ref(),
        Some(&index),
        Some(&mut diff_options),
    )?;

    let has_changes = diff.deltas().len() > 0;
    Ok(has_changes)
}

fn select_commit_type() -> String {
    let items = vec![
        "feat", "fix", "build", "chore", "ci", "cd", "docs", "perf", "refactor", "revert", "style",
        "test",
    ];
    let selection = Select::new()
        .items(&items)
        .with_prompt("Select the type of commit")
        .default(0)
        .interact()
        .unwrap();

    items[selection].to_string()
}

fn confirm_breaking_change() -> bool {
    let confirmation = Select::new()
        .items(&["No", "Yes"])
        .with_prompt("Is this a breaking change?")
        .default(0)
        .interact()
        .unwrap();
    confirmation == 1
}

fn input_scope() -> String {
    Input::new()
        .with_prompt("Enter the scope of the commit (e.g., 'api', 'docker', etc.)")
        .allow_empty(true)
        .interact_text()
        .unwrap()
}

fn input_short_message() -> String {
    Input::new()
        .with_prompt("Enter a short description (max 150 characters)")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.len() <= 150 {
                Ok(())
            } else {
                Err("The message must be 150 characters or less")
            }
        })
        .interact_text()
        .unwrap()
}

fn input_long_message() -> String {
    Input::new()
        .with_prompt("Enter a detailed description")
        .allow_empty(true)
        .interact_text()
        .unwrap()
}

fn commit_changes(message: &str, amend: bool) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    let sig = repo.signature()?;
    let mut index = repo.index()?;
    let _ = index.write();
    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;

    if amend {
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;
        let _ = commit.amend(
            Some("HEAD"),
            Some(&sig),
            Some(&sig),
            None,
            Some(message),
            Some(&tree),
        );
    } else {
        let head = repo.head()?.peel_to_commit()?;
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])?;
    }

    Ok(())
}
