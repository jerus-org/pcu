use regex::Error as RegexError;
use std::{env, ffi::OsString, num::ParseIntError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    /// Unable to acquire the child process' standard input to write the commit data for signing
    #[error("Failed to acquire standard input handler")]
    Stdin,

    /// Unable to retrieve the signed data from the child process
    #[error("Failed to get output of signing process call: {0}")]
    Stdout(String),

    #[error("{0}")]
    GpgError(String),
    #[error("Environment variable PCU_BRANCH not set")]
    EnvVarBranchNotSet,
    #[error("Environment specified in PCU_BRANCH not set")]
    EnvVarBranchNotFound,
    #[error("Environment variable PCU_PULL_REQUEST not set")]
    EnvVarPullRequestNotSet,
    #[error("Environment specified in PCU_PULL_REQUEST not found")]
    EnvVarPullRequestNotFound,
    #[error("Default change log file name not set")]
    DefaultChangeLogNotSet,
    #[error("Invalid path for changelog file {0:?}")]
    InvalidPath(OsString),
    #[error("Regex string is not valid.")]
    InvalidRegex,
    #[error("Keep a changelog says: {0}")]
    KeepAChangelog(String),
    #[error("On default branch")]
    OnDefaultBranch,
    #[error("Unknown format for pull request: {0}")]
    UknownPullRequestFormat(String),
    #[error("No default changelog file found")]
    NoChangeLogFileFound,
    #[error("ParseInt says: {0:?}")]
    ParseInt(#[from] ParseIntError),
    #[error("Octocrab says: {0:?}")]
    Octocrab(#[from] octocrab::Error),
    #[error("Url says: {0:?}")]
    UrlParse(#[from] url::ParseError),
    #[error("Git2 says: {0:?}")]
    Git2(#[from] git2::Error),
    #[error("env var says: {0:?}")]
    EnvVar(#[from] env::VarError),
    #[error("io error says: {0:?}")]
    IO(#[from] std::io::Error),
    #[error("utf8 error says: {0:?}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("config error says: {0:?}")]
    Config(#[from] config::ConfigError),
    #[error("regex error says: {0:?}")]
    Regex(#[from] RegexError),
}
