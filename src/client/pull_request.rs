use std::{env, sync::Arc};

use config::Config;
use octocrab::Octocrab;

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
    pub(crate) pr_number: u64,
}

impl PullRequest {
    pub async fn new_pull_request_opt(settings: &Config) -> Result<Option<Self>, Error> {
        // Use the command config to check the command client is run for
        log::trace!("command: {:?}", settings.get::<String>("command"));
        let command: String = settings.get("command").map_err(|_| Error::CommandNotSet)?;

        // If the command is not pull-request then return None
        if command != "pull-request" {
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
        let pr_number = pr_number.parse::<u64>()?;

        // Get the github pull release and store the title in the client struct
        // The title can be edited by the calling programme if desired before creating the prtitle

        let octocrab = match settings.get::<String>("pat") {
            Ok(pat) => {
                log::debug!("Using personal access token for authentication");
                Arc::new(
                    Octocrab::builder()
                        .base_uri("https://api.github.com")?
                        .personal_token(pat)
                        .build()?,
                )
            }
            // base_uri: https://api.github.com
            // auth: None
            // client: http client with the octocrab user agent.
            Err(_) => {
                log::debug!("Creating un-authenticated instance");
                octocrab::instance()
            }
        };
        log::debug!("Using Octocrab instance: {octocrab:#?}");
        let pr_handler = octocrab.pulls(&owner, &repo);
        log::debug!("Pull handler acquired");
        let pr = pr_handler.get(pr_number).await?;

        let title = pr.title.unwrap_or("".to_owned());

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
