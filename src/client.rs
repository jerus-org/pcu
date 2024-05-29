use std::{env, ffi::OsString, str::FromStr};

use keep_a_changelog::ChangeKind;
use url::Url;

use crate::Error;
use crate::PrTitle;

const CHANGELOG_FILENAME: &str = "CHANGELOG.md";

#[derive(Debug, Default)]
pub struct Client {
    branch: String,
    pull_request: String,
    title: String,
    #[allow(dead_code)]
    owner: String,
    #[allow(dead_code)]
    repo: String,
    pr_number: u64,
    changelog: OsString,
    changelog_update: Option<PrTitle>,
}

impl Client {
    pub async fn new() -> Result<Self, Error> {
        // Use the PCU_BRANCH env variable to direct to the appropriate CI environment variable to find the branch data
        let pcu_branch = env::var("PCU_BRANCH").map_err(|_| Error::EnvVarBranchNotSet)?;

        let branch = env::var(pcu_branch).map_err(|_| Error::EnvVarBranchNotFound)?;

        // Use the PCU_PULL_REQUEST env variable to direct to the appropriate CI environment variable to find the PR data
        let pcu_pull_request =
            env::var("PCU_PULL_REQUEST").map_err(|_| Error::EnvVarPullRequestNotSet)?;

        let pull_request =
            env::var(pcu_pull_request).map_err(|_| Error::EnvVarPullRequestNotFound)?;

        let (owner, repo, pr_number) = get_keys(&pull_request)?;

        let pr_number = pr_number.parse::<u64>()?;

        // Get the github pull release and store the title in the client struct
        // The title can be edited by the calling programme if desired before creating the prtitle
        let pr = octocrab::instance()
            .pulls(&owner, &repo)
            .get(pr_number)
            .await?;

        let title = pr.title.unwrap_or("".to_owned());

        // Get the name of the changelog file
        let mut changelog = OsString::from(CHANGELOG_FILENAME);
        if let Ok(files) = std::fs::read_dir(".") {
            for file in files.into_iter().flatten() {
                println!("File: {:?}", file.path());

                if file.file_name().to_string_lossy().contains("change")
                    && file.file_type().unwrap().is_file()
                {
                    changelog = file.file_name();
                    break;
                }
            }
        };

        Ok(Self {
            branch,
            pull_request,
            title,
            owner,
            repo,
            pr_number,
            changelog,
            changelog_update: None,
        })
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn pull_release(&self) -> &str {
        &self.pull_request
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    pub fn create_entry(&mut self) -> Result<(), Error> {
        let mut pr_title = PrTitle::parse(&self.title);
        pr_title.pr_id = Some(self.pr_number);
        pr_title.pr_url = Some(Url::from_str(&self.pull_request)?);
        pr_title.calculate_section_and_entry();

        self.changelog_update = Some(pr_title);

        Ok(())
    }

    pub fn update_changelog(&mut self) -> Result<(), Error> {
        if self.changelog_update.is_none() {
            return Err(Error::NoChangeLogFileFound);
        }

        if let Some(update) = &mut self.changelog_update {
            update.update_changelog(&self.changelog);
        }
        Ok(())
    }
}

fn get_keys(pull_request: &str) -> Result<(String, String, String), Error> {
    if pull_request.contains("github.com") {
        let parts = pull_request.splitn(7, '/').collect::<Vec<&str>>();
        Ok((
            parts[3].to_string(),
            parts[4].to_string(),
            parts[6].to_string(),
        ))
    } else {
        Err(Error::UknownPullRequestFormat(pull_request.to_string()))
    }
}

impl Client {
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
    pub fn pr_number(&self) -> &str {
        if self.pull_request.contains("github.com") {
            let parts = self.pull_request.splitn(7, '/').collect::<Vec<&str>>();
            parts[6]
        } else {
            ""
        }
    }

    pub fn pr_number_as_u64(&self) -> u64 {
        if self.pull_request.contains("github.com") {
            let parts = self.pull_request.splitn(7, '/').collect::<Vec<&str>>();

            if let Ok(pr_number) = parts[6].parse::<u64>() {
                pr_number
            } else {
                0
            }
        } else {
            0
        }
    }

    pub fn owner(&self) -> &str {
        if self.pull_request.contains("github.com") {
            let parts = self.pull_request.splitn(7, '/').collect::<Vec<&str>>();
            parts[3]
        } else {
            ""
        }
    }

    pub fn repo(&self) -> &str {
        if self.pull_request.contains("github.com") {
            let parts = self.pull_request.splitn(7, '/').collect::<Vec<&str>>();
            parts[4]
        } else {
            ""
        }
    }

    pub fn changelog(&self) -> &str {
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
