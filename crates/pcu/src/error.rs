use std::{ffi::OsString, fmt::Display, num::ParseIntError};

use regex::Error as RegexError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    /// Unable to acquire the child process' standard input to write the commit
    /// data for signing
    #[error("failed to acquire standard input handler")]
    Stdin,

    /// Unable to retrieve the signed data from the child process
    #[error("failed to get output of signing process call: {0}")]
    Stdout(String),

    #[error("{0}")]
    GpgError(String),
    #[error("Environment variable PCU_BRANCH not set")]
    EnvVarBranchNotSet,
    #[error("Environment variable specified in PCU_BRANCH not found")]
    EnvVarBranchNotFound,
    #[error("Environment variable PCU_PULL_REQUEST not set")]
    EnvVarPullRequestNotSet,
    #[error("Environment variable specified in PCU_PULL_REQUEST not found")]
    EnvVarPullRequestNotFound,
    #[error("Unreleased section not found in pull request log")]
    NoUnreleasedSection,
    #[error("Command not set")]
    CommandNotSet,
    #[error("Semver needs to be set for release")]
    MissingSemver,
    #[error("No package specified for the release")]
    NoPackageSpecified,
    #[error("Tag not found {0:?}")]
    TagNotFound(String),
    #[error("Invalid version string")]
    InvalidVersion(String),
    #[error("Default pull request log file name not set")]
    DefaultChangeLogNotSet,
    #[error("Invalid path for prlog file {0:?}")]
    InvalidPath(OsString),
    #[error("Keep a prlog says: {0}")]
    KeepAChangelog(String),
    #[error("No GitHub API private key found")]
    NoGitHubAPIPrivateKey,
    #[error("No GitHub API Authorisation found")]
    NoGitHubAPIAuth,
    #[error("Unknown format for pull request: {0}")]
    UnknownPullRequestFormat(String),
    #[error("No default prlog file found")]
    NoChangeLogFileFound,
    #[error("HEAD is not a merge commit")]
    NotAMergeCommit,
    #[error("Merge commit message does not contain a pull request number")]
    InvalidMergeCommitMessage,
    #[error("ParseInt says: {0:?}")]
    ParseInt(#[from] ParseIntError),
    #[error("Octocrate says: {0:?}")]
    Octocrate(#[from] octocrate::Error),
    #[error("GraphQL says: {0:?}")]
    GraphQL(#[from] GraphQLWrapper),
    #[error("Url says: {0:?}")]
    UrlParse(#[from] url::ParseError),
    #[error("Git2 says: {0:?}")]
    Git2(#[from] git2::Error),
    #[error("io error says: {0:?}")]
    IO(#[from] std::io::Error),
    #[error("utf8 error says: {0:?}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("config error says: {0:?}")]
    Config(#[from] config::ConfigError),
    #[error("regex error says: {0:?}")]
    Regex(#[from] RegexError),
    #[error("cargo_toml error says: {0:?}")]
    CargoToml(#[from] cargo_toml::Error),
    #[error("toml deserialization error says: {0:?}")]
    Toml(#[from] toml::de::Error),
    #[error("gen-bsky draft error says: {0:?}")]
    DraftError(#[from] gen_bsky::DraftError),
    #[error("gen-bsky post error says: {0:?}")]
    PostError(#[from] gen_bsky::PostError),
    /// Errors arising from the gen-linkedin client
    #[error("gen-linkedin says: {0:?}")]
    LinkedIn(#[from] gen_linkedin::Error),
}

#[derive(Debug)]
pub struct GraphQLWrapper(gql_client::GraphQLError);

impl std::error::Error for GraphQLWrapper {
    fn description(&self) -> &str {
        "A GraphQL error occurred"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        None
    }
}

impl From<gql_client::GraphQLError> for GraphQLWrapper {
    fn from(err: gql_client::GraphQLError) -> Self {
        GraphQLWrapper(err)
    }
}

impl Display for GraphQLWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
