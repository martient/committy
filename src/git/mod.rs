mod branch;
mod commit;
mod process;
mod repository;
mod tag;

#[allow(unused_imports)]
pub use branch::{
    branch_exists, branch_exists_in, checkout_branch, checkout_branch_in, create_branch,
    create_branch_in, validate_branch_name,
};
#[allow(unused_imports)]
pub use commit::{commit_changes, commit_changes_in, format_commit_message, stage_file};
#[allow(unused_imports)]
pub use commit::{commit_changes_in_with_config, commit_changes_with_config};
#[allow(unused_imports)]
pub use process::{
    resolve_git_command_config, run_git, run_git_capture, run_git_with_input, GitCommandConfig,
};
#[allow(unused_imports)]
pub use repository::{
    discover_repository, discover_repository_from, has_staged_changes, has_staged_changes_from,
    list_changed_files, list_changed_files_from, validate_git_config, validate_git_config_from,
};
pub use tag::{TagGenerator, TagGeneratorOptions};
