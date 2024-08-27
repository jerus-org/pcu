use std::env;

use config::Config;

use crate::Error;

pub(crate) struct PullRequest {
    pub(crate) pull_request: String,
    pub(crate) title: String,
    #[allow(dead_code)]
    pub(crate) owner: String,
    #[allow(dead_code)]
    pub(crate) repo: String,
    #[allow(dead_code)]
    pub(crate) repo_url: String,
    pub(crate) pr_number: i64,
}

impl PullRequest {
    pub async fn new_pull_request_opt(
        settings: &Config,
        graphql: &gql_client::Client,
    ) -> Result<Option<Self>, Error> {
        // Use the command config to check the command client is run for
        log::trace!("command: {:?}", settings.get::<String>("command"));
        let command: String = settings.get("command").map_err(|_| Error::CommandNotSet)?;

        // If the command is not pull-request then return None
        if command != "pull-request" || command != "rebase" {
            return Ok(None);
        }

        // Use the pull_request config setting to direct to the appropriate CI environment variable to find the PR data
        log::trace!("pull_request: {:?}", settings.get::<String>("pull_request"));
        let pcu_pull_request: String = settings
            .get("pull_request")
            .map_err(|_| Error::EnvVarPullRequestNotSet)?;
        log::trace!("pcu_pull_request: {:?}", pcu_pull_request);
        let pull_request =
            env::var(pcu_pull_request).map_err(|_| Error::EnvVarPullRequestNotFound)?;

        let (owner, repo, pr_number, repo_url) = PullRequest::get_keys(&pull_request)?;
        log::debug!(
            "Owner: {}, repo: {}, pr_number: {}, repo_url: {}",
            owner,
            repo,
            pr_number,
            repo_url
        );
        let pr_number = pr_number.parse::<i64>()?;

        log::debug!("********* Using GraphQL");
        let title =
            super::graphql::get_pull_request_title(graphql, &owner, &repo, pr_number).await?;

        Ok(Some(Self {
            pull_request,
            title,
            owner,
            repo,
            repo_url,
            pr_number,
        }))
    }

    fn get_keys(pull_request: &str) -> Result<(String, String, String, String), Error> {
        if pull_request.contains("github.com") {
            let parts = pull_request.splitn(7, '/').collect::<Vec<&str>>();
            Ok((
                parts[3].to_string(),
                parts[4].to_string(),
                parts[6].to_string(),
                format!("https://github.com/{}/{}", parts[3], parts[4]),
            ))
        } else {
            Err(Error::UknownPullRequestFormat(pull_request.to_string()))
        }
    }
}
