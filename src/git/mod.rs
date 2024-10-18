mod repository;
mod commit;
mod tag;

pub use repository::has_staged_changes;
pub use commit::{commit_changes, format_commit_message};
pub use tag::{TagGenerator, TagGeneratorOptions};