use regex::Error as RegexError;
use std::{env, ffi::OsString, fmt::Display, num::ParseIntError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    /// Unable to acquire the child process' standard input to write the commit data for signing
    #[error("failed to acquire standard input handler")]
    Stdin,

    /// Unable to retrieve the signed data from the child process
    #[error("failed to get output of signing process call: {0}")]
    Stdout(String),

    #[error("bluesky post for `{0}` contains too many characters: {1}")]
    PostTooCharacters(String, usize),
    #[error("bluesky post for `{0}` contains too many graphemes: {1}")]
    PostTooManyGraphemes(String, usize),

    #[error("{0}")]
    GpgError(String),
    #[error("No bluesky identifier provided")]
    NoBlueskyIdentifier,
    #[error("No bluesky password provided")]
    NoBlueskyPassword,
    #[error("Future capacity is too large")]
    FutureCapacityTooLarge,
    #[error("Path not found: {0}")]
    PathNotFound(String),
    #[error("File extension invalid (must be `{1}`): {0}")]
    FileExtensionInvalid(String, String),
    #[error("Environment variable PCU_BRANCH not set")]
    EnvVarBranchNotSet,
    #[error("Environment variable specified in PCU_BRANCH not found")]
    EnvVarBranchNotFound,
    #[error("Environment variable PCU_PULL_REQUEST not set")]
    EnvVarPullRequestNotSet,
    #[error("Environment variable specified in PCU_PULL_REQUEST not found")]
    EnvVarPullRequestNotFound,
    #[error("Unreleased section not found in change log")]
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
    #[error("Default change log file name not set")]
    DefaultChangeLogNotSet,
    #[error("Invalid path for changelog file {0:?}")]
    InvalidPath(OsString),
    #[error("Regex string is not valid.")]
    InvalidRegex,
    #[error("Keep a changelog says: {0}")]
    KeepAChangelog(String),
    #[error("No GitHub API private key found")]
    NoGitHubAPIPrivateKey,
    #[error("No GitHub API Authorisation found")]
    NoGitHubAPIAuth,
    #[error("On default branch")]
    OnDefaultBranch,
    #[error("Unknown format for pull request: {0}")]
    UnknownPullRequestFormat(String),
    #[error("No default changelog file found")]
    NoChangeLogFileFound,
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
    #[error("cargo_toml error says: {0:?}")]
    CargoToml(#[from] cargo_toml::Error),
    #[error("toml deserialization error says: {0:?}")]
    Toml(#[from] toml::de::Error),
    #[error("bsky_sdk error says: {0:?}")]
    BskySdk(#[from] bsky_sdk::Error),
    #[error("bsky_sdk create_session error says: {0:?}")]
    BlueskyLoginError(String),
    #[error("serde_json create_session error says: {0:?}")]
    SerdeJsonError(#[from] serde_json::error::Error),
    #[error("link-bridge error says: {0:?}")]
    RedirectorError(#[from] link_bridge::RedirectorError),
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
