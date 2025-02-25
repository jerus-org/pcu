use std::{collections::HashMap, env, ffi::OsString, fmt::Debug};

pub(crate) mod graphql;
mod pull_request;

use self::pull_request::PullRequest;

use config::Config;
use git2::Repository;
use keep_a_changelog::{ChangeKind, ChangelogParseOptions};
use octocrate::{APIConfig, AppAuthorization, GitHubAPI, PersonalAccessToken};
use owo_colors::{OwoColorize, Style};

use crate::Error;
use crate::PrTitle;

const END_POINT: &str = "https://api.github.com/graphql";

pub struct Client {
    #[allow(dead_code)]
    // pub(crate) settings: Config,
    pub(crate) git_repo: Repository,
    pub(crate) github_rest: GitHubAPI,
    pub(crate) github_graphql: gql_client::Client,
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) default_branch: String,
    pub(crate) branch: Option<String>,
    pull_request: Option<PullRequest>,
    pub(crate) changelog: OsString,
    pub(crate) line_limit: usize,
    pub(crate) changelog_parse_options: ChangelogParseOptions,
    pub(crate) changelog_update: Option<PrTitle>,
    pub(crate) commit_message: String,
}

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("github_graphql", &self.github_graphql)
            .field("owner", &self.owner)
            .field("repo", &self.repo)
            .field("default_branch", &self.default_branch)
            .field("branch", &self.branch)
            .field("pull_request", &self.pull_request)
            .field("changelog", &self.changelog)
            .field("line_limit", &self.line_limit)
            .field("changelog_parse_options", &self.changelog_parse_options)
            .field("changelog_update", &self.changelog_update)
            .field("commit_message", &self.commit_message)
            .finish()

    }
}

impl Client {
    pub async fn new_with(settings: Config) -> Result<Self, Error> {
        let cmd = settings
            .get::<String>("command")
            .map_err(|_| Error::CommandNotSet)?;
        log::trace!("cmd: {:?}", cmd);

        // Use the username config settings to direct to the appropriate CI environment variable to find the owner
        log::trace!("owner: {:?}", settings.get::<String>("username"));
        let pcu_owner: String = settings
            .get("username")
            .map_err(|_| Error::EnvVarBranchNotSet)?;
        let owner = env::var(pcu_owner).map_err(|_| Error::EnvVarBranchNotFound)?;

        // Use the reponame config settings to direct to the appropriate CI environment variable to find the repo
        log::trace!("repo: {:?}", settings.get::<String>("reponame"));
        let pcu_owner: String = settings
            .get("reponame")
            .map_err(|_| Error::EnvVarBranchNotSet)?;
        let repo = env::var(pcu_owner).map_err(|_| Error::EnvVarBranchNotFound)?;

        let default_branch = settings
            .get::<String>("default_branch")
            .unwrap_or("main".to_string());

        let commit_message = settings
            .get::<String>("commit_message")
            .unwrap_or("".to_string());

        let line_limit = settings.get::<usize>("line_limit").unwrap_or(10);

        let (github_rest, github_graphql) =
            Client::get_github_apis(&settings, &owner, &repo).await?;

        log::trace!("Executing for command: {}", &cmd);
        let (branch, pull_request) = if &cmd == "pr" {
            // Use the branch config settings to direct to the appropriate CI environment variable to find the branch data
            log::trace!("branch: {:?}", settings.get::<String>("branch"));
            let pcu_branch: String = settings
                .get("branch")
                .map_err(|_| Error::EnvVarBranchNotSet)?;
            let branch = env::var(pcu_branch).map_err(|_| Error::EnvVarBranchNotFound)?;
            let branch = if branch.is_empty() {
                None
            } else {
                Some(branch)
            };

            let pull_request =
                PullRequest::new_pull_request_opt(&settings, &github_graphql).await?;

            (branch, pull_request)
        } else {
            let branch = None;
            let pull_request = None;
            (branch, pull_request)
        };
        log::trace!("branch: {:?} and pull_request: {:?}", branch, pull_request);

        // Use the log config setting to set the default change log file name
        log::trace!("log: {:?}", settings.get::<String>("log"));
        let default_change_log: String = settings
            .get("log")
            .map_err(|_| Error::DefaultChangeLogNotSet)?;

        // Get the name of the changelog file
        let mut changelog = OsString::from(default_change_log);
        if let Ok(files) = std::fs::read_dir(".") {
            for file in files.into_iter().flatten() {
                log::trace!("File: {:?}", file.path());

                if file
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("change")
                    && file.file_type().unwrap().is_file()
                {
                    changelog = file.file_name();
                    break;
                }
            }
        };

        let git_repo = git2::Repository::open(".")?;

        let svs_root = settings
            .get("dev_platform")
            .unwrap_or_else(|_| "https://github.com/".to_string());
        let prefix = settings
            .get("version_prefix")
            .unwrap_or_else(|_| "v".to_string());
        let repo_url = Some(format!("{}{}/{}", svs_root, owner, repo));
        let changelog_parse_options = ChangelogParseOptions {
            url: repo_url,
            head: Some("HEAD".to_string()),
            tag_prefix: Some(prefix),
        };

        Ok(Self {
            git_repo,
            github_rest,
            github_graphql,
            default_branch,
            branch,
            owner,
            repo,
            pull_request,
            changelog,
            line_limit,
            changelog_parse_options,
            changelog_update: None,
            commit_message,
        })
    }

    /// Get the GitHub API instance
    async fn get_github_apis(
        settings: &Config,
        owner: &str,
        repo: &str,
    ) -> Result<(GitHubAPI, gql_client::Client), Error> {
        let bld_style = Style::new().bold();
        log::debug!("*******\nGet GitHub API instance");
        let (config, token) = match settings.get::<String>("app_id") {
            Ok(app_id) => {
                log::debug!("Using {} for authentication", "GitHub App".style(bld_style));

                let private_key = settings
                    .get::<String>("private_key")
                    .map_err(|_| Error::NoGitHubAPIPrivateKey)?;

                let app_authorization = AppAuthorization::new(app_id, private_key);
                let config = APIConfig::with_token(app_authorization).shared();

                let api = GitHubAPI::new(&config);

                let installation = api
                    .apps
                    .get_repo_installation(owner, repo)
                    .send()
                    .await
                    .unwrap();
                let installation_token = api
                    .apps
                    .create_installation_access_token(installation.id)
                    .send()
                    .await
                    .unwrap();

                (
                    APIConfig::with_token(installation_token.clone()).shared(),
                    installation_token.token,
                )
            }
            Err(_) => {
                let pat = settings
                    .get::<String>("pat")
                    .map_err(|_| Error::NoGitHubAPIAuth)?;
                log::debug!(
                    "Falling back to {} for authentication",
                    "Personal Access Token".style(bld_style)
                );

                // Create a personal access token
                let personal_access_token = PersonalAccessToken::new(&pat);

                // Use the personal access token to create a API configuration
                (APIConfig::with_token(personal_access_token).shared(), pat)
            }
        };

        let auth = format!("Bearer {}", token);

        let headers = HashMap::from([
            ("X-Github-Next-Global-ID", "1"),
            ("User-Agent", owner),
            ("Authorization", &auth),
        ]);

        let github_graphql = gql_client::Client::new_with_headers(END_POINT, headers);

        let github_rest = GitHubAPI::new(&config);

        Ok((github_rest, github_graphql))
    }

    pub fn branch_or_main(&self) -> &str {
        self.branch.as_ref().map_or("main", |v| v)
    }

    pub fn pull_request(&self) -> &str {
        if let Some(pr) = self.pull_request.as_ref() {
            &pr.pull_request
        } else {
            ""
        }
    }

    pub fn title(&self) -> &str {
        if let Some(pr) = self.pull_request.as_ref() {
            &pr.title
        } else {
            ""
        }
    }

    pub fn pr_number(&self) -> i64 {
        if let Some(pr) = self.pull_request.as_ref() {
            pr.pr_number
        } else {
            0
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }

    pub fn line_limit(&self) -> usize {
        self.line_limit
    }

    pub fn set_title(&mut self, title: &str) {
        if self.pull_request.is_some() {
            self.pull_request.as_mut().unwrap().title = title.to_string();
        }
    }

    pub fn is_default_branch(&self) -> bool {
        if let Some(branch) = &self.branch {
            *branch == self.default_branch
        } else {
            false
        }
    }

    pub fn section(&self) -> Option<&str> {
        if let Some(update) = &self.changelog_update {
            if let Some(section) = &update.section {
                match section {
                    ChangeKind::Added => Some("Added"),
                    ChangeKind::Changed => Some("Changed"),
                    ChangeKind::Deprecated => Some("Deprecated"),
                    ChangeKind::Fixed => Some("Fixed"),
                    ChangeKind::Removed => Some("Removed"),
                    ChangeKind::Security => Some("Security"),
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn entry(&self) -> Option<&str> {
        if let Some(update) = &self.changelog_update {
            Some(&update.entry)
        } else {
            None
        }
    }

    pub fn changelog_as_str(&self) -> &str {
        if let Some(cl) = &self.changelog.to_str() {
            cl
        } else {
            ""
        }
    }

    pub fn set_changelog(&mut self, changelog: &str) {
        self.changelog = changelog.into();
    }
}
