use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Environment variable PCU_BRANCH not set")]
    EnvVarBranchNotSet,
    #[error("Environment specified in PCU_BRANCH not set")]
    EnvVarBranchNotFound,
    #[error("Environment variable PCU_PULL_REQUEST not set")]
    EnvVarPullRequestNotSet,
    #[error("Environment specified in PCU_PULL_REQUEST not set")]
    EnvVarPullRequestNotFound,
    #[error("On default branch")]
    OnDefaultBranch,
    #[error("Unknown format for pull request: {0}")]
    UknownPullRequestFormat(String),
    #[error("0:?")]
    ParseInt(#[from] ParseIntError),
    #[error("0:?")]
    Octocrab(#[from] octocrab::Error),
    #[error("0:?")]
    UrlParse(#[from] url::ParseError),
}
