use git2::Error as Git2Error;
use std::io::Error as IoError;
use structopt::clap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Git error: {0}")]
    GitError(#[from] Git2Error),

    #[error("IO error: {0}")]
    IoError(#[from] IoError),

    #[error("Input error: {0}")]
    InputError(String),

    #[error("No staged changes found\nFor help, run 'committy --help'")]
    NoStagedChanges,

    #[error(
        "Please commit your staged changes before doing that\nFor help, run 'committy tag --help'"
    )]
    StagedChanges,

    #[error("Git user configuration is missing: {0}\nFor help, run 'git config --global user.name \"Your Name\"' and 'git config --global user.email \"your.email@example.com\"'")]
    GitConfigError(String),

    #[error("{0}")]
    Generic(String),

    #[error("SemVer error: {0}")]
    SemVerError(String),

    #[error("RegexError error: {0}")]
    RegexError(String),

    #[error("Found {0} commit(s) with lint issues")]
    LintIssues(usize),
}

impl From<clap::Error> for CliError {
    fn from(error: clap::Error) -> Self {
        CliError::Generic(error.to_string())
    }
}
