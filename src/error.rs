use structopt::clap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Git error: {0}")]
    GitError(#[from] git2::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Input error: {0}")]
    InputError(String),

    #[error("No staged changes found")]
    NoStagedChanges,

    #[error("Please commit your staged changes before doing that")]
    StagedChanges,

    #[error("{0}")]
    Generic(String),

    #[error("SemVer error: {0}")]
    SemVerError(String),

    #[error("RegexError error: {0}")]
    RegexError(String),
}

impl From<clap::Error> for CliError {
    fn from(error: clap::Error) -> Self {
        CliError::Generic(error.to_string())
    }
}
