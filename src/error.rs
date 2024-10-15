use thiserror::Error;
use structopt::clap;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Git error: {0}")]
    GitError(#[from] git2::Error),
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Input error: {0}")]
    InputError(String),
    
    // #[error("Validation error: {0}")]
    // ValidationError(String),

    #[error("No staged changes found")]
    NoStagedChanges(),

    #[error("dd {0}")]
    Generic(String),
}

impl From<clap::Error> for CliError {
    fn from(error: clap::Error) -> Self {
        CliError::Generic(error.to_string())
    }
}