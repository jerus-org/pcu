use std::{collections::HashMap, env, ffi::OsString};

mod pull_request;

use self::pull_request::PullRequest;

use config::Config;
use git2::Repository;
use keep_a_changelog::{ChangeKind, ChangelogParseOptions};

use crate::Error;
use crate::PrTitle;

pub struct Client {
    #[allow(dead_code)]
    pub(crate) settings: Config,
    pub(crate) git_repo: Repository,
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) branch: Option<String>,
    pull_request: Option<PullRequest>,
    pub(crate) changelog: OsString,
    pub(crate) changelog_parse_options: ChangelogParseOptions,
    pub(crate) changelog_update: Option<PrTitle>,
}

impl Client {
    pub async fn new_with(settings: Config) -> Result<Self, Error> {
        log::trace!(
            "new_with settings: {:#?}",
            settings
                .clone()
                .try_deserialize::<HashMap<String, String>>()?,
        );

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

        let (branch, pull_request) = if &cmd == "pull-request" {
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

            let pull_request = PullRequest::new_pull_request_opt(&settings).await?;
            (branch, pull_request)
        } else {
            let branch = None;
            let pull_request = None;
            (branch, pull_request)
        };

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
            settings,
            git_repo,
            branch,
            owner,
            repo,
            pull_request,
            changelog,
            changelog_parse_options,
            changelog_update: None,
        })
    }

    pub fn branch(&self) -> &str {
        if let Some(branch) = self.branch.as_ref() {
            branch
        } else {
            "main"
        }
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

    pub fn set_title(&mut self, title: &str) {
        if self.pull_request.is_some() {
            self.pull_request.as_mut().unwrap().title = title.to_string();
        }
    }

    pub fn is_default_branch(&self) -> bool {
        let default_branch = self
            .settings
            .get::<String>("default_branch")
            .unwrap_or("main".to_string());
        self.branch == Some(default_branch)
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
