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
    /// The GitHub App private key setting is absent. See
    /// [`Error::InvalidGitHubAPIPrivateKey`] for a key that is present but
    /// unusable.
    #[error("No GitHub API private key found: set PCU_PRIVATE_KEY (or `private_key` in pcu.toml)")]
    NoGitHubAPIPrivateKey,
    /// The GitHub App private key setting is present but is not a PEM RSA key
    /// that can be used to sign. The message says what the value appears to
    /// be; it never contains the value.
    #[error("The GitHub App private key is set but cannot be used: {0}")]
    InvalidGitHubAPIPrivateKey(String),
    #[error("No GitHub API Authorisation found")]
    NoGitHubAPIAuth,
    /// A GitHub App with write access was offered for a read-only job.
    #[error("The GitHub App has more than read access ({0}); refusing to read with it. Use a read-only App, or unset PCU_APP_ID to read with the token")]
    AppNotReadOnly(String),
    #[error("Invalid GitHub App id {0:?}: expected a numeric id")]
    InvalidAppId(String),
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
    #[error("Octocrab says: {0:?}")]
    Octocrab(#[from] octocrab::Error),
    #[error("GraphQL says: {0:?}")]
    GraphQL(#[from] GraphQLWrapper),
    #[error("Url says: {0:?}")]
    UrlParse(#[from] url::ParseError),
    #[error("Git2 says: {0:?}")]
    Git2(#[from] git2::Error),
    #[error("Git error: {0}")]
    GitError(String),
    /// Transparent — `pcu_release_assets::Error` already carries its own
    /// context (not-found, still-a-draft, HTTP status).
    #[error("{0}")]
    ReleaseAssets(#[from] pcu_release_assets::Error),
    #[error("{0}")]
    MissingSigningScaffold(String),
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
    #[error("serde_json error says: {0:?}")]
    SerdeJson(#[from] serde_json::Error),
    #[cfg(feature = "bsky")]
    #[error("gen-bsky draft error says: {0:?}")]
    DraftError(#[from] gen_bsky::DraftError),
    #[cfg(feature = "bsky")]
    #[error("gen-bsky post error says: {0:?}")]
    PostError(#[from] gen_bsky::PostError),
    /// Errors arising from the gen-linkedin client
    #[cfg(feature = "linkedin")]
    #[error("gen-linkedin says: {0:?}")]
    LinkedIn(#[from] gen_linkedin::Error),
    #[cfg(feature = "linkedin")]
    #[error("gen-linkedin draft error: {0:?}")]
    LinkedinDraftError(#[from] gen_linkedin::DraftError),
    #[cfg(feature = "linkedin")]
    #[error("gen-linkedin post error: {0:?}")]
    LinkedinPostError(#[from] gen_linkedin::PostError),
    #[error("missing configuration: {0}")]
    MissingConfig(String),
    /// Signature verification failed
    #[error("Signature verification failed: {0} failure(s) detected")]
    SignatureVerificationFailed(usize),
    /// The `[verify_signatures]` policy in `pcu.toml` is invalid
    #[error("Invalid verify-signatures policy: {0}")]
    VerifyConfig(String),
    /// SLSA attestation error
    #[error("Attestation error: {0}")]
    Attestation(String),
    /// Webhook trigger error
    #[error("Trigger error: {0}")]
    Trigger(String),
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
