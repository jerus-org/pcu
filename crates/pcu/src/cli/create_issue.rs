use clap::Parser;
use color_eyre::Result;
use octocrate::issues;
use octocrate::{APIConfig, GitHubAPI, PersonalAccessToken, StringOrInteger};
use std::env;

use super::CIExit;
use crate::Error;

#[derive(Debug, Parser, Clone)]
/// Create a GitHub issue on the target repository
pub struct CreateIssue {
    /// Title of the issue to create
    #[clap(long)]
    pub title: String,

    /// Body of the issue (markdown). If not provided, the issue body will be empty.
    #[clap(long, default_value = "")]
    pub body: String,

    /// Repository owner. Defaults to CIRCLE_PROJECT_USERNAME env var.
    #[clap(long)]
    pub owner: Option<String>,

    /// Repository name. Defaults to CIRCLE_PROJECT_REPONAME env var.
    #[clap(long)]
    pub repo: Option<String>,

    /// GitHub personal access token. Defaults to GITHUB_TOKEN env var.
    #[clap(long)]
    pub github_token: Option<String>,

    /// Log what would happen without making the API request
    #[clap(long)]
    pub dry_run: bool,
}

impl CreateIssue {
    pub async fn run(&self) -> Result<CIExit, Error> {
        let owner = self
            .owner
            .clone()
            .or_else(|| env::var("CIRCLE_PROJECT_USERNAME").ok())
            .ok_or_else(|| {
                Error::MissingConfig(
                    "owner not provided and CIRCLE_PROJECT_USERNAME not set".to_string(),
                )
            })?;

        let repo = self
            .repo
            .clone()
            .or_else(|| env::var("CIRCLE_PROJECT_REPONAME").ok())
            .ok_or_else(|| {
                Error::MissingConfig(
                    "repo not provided and CIRCLE_PROJECT_REPONAME not set".to_string(),
                )
            })?;

        if self.dry_run {
            log::info!(
                "Dry run: would create issue on {owner}/{repo}: {}",
                self.title
            );
            return Ok(CIExit::IssueCreated(format!(
                "https://github.com/{owner}/{repo}/issues/0"
            )));
        }

        let env_token = env::var("GITHUB_TOKEN").ok();
        let token = self
            .github_token
            .as_deref()
            .or(env_token.as_deref())
            .ok_or(Error::NoGitHubAPIAuth)?;

        let pat = PersonalAccessToken::new(token);
        let config = APIConfig::with_token(pat).shared();
        let api = GitHubAPI::new(&config);

        let body = if self.body.is_empty() {
            None
        } else {
            Some(self.body.clone())
        };

        let request = issues::create::Request {
            title: StringOrInteger::String(self.title.clone()),
            body,
            assignee: None,
            assignees: None,
            labels: None,
            milestone: None,
        };

        let issue = api
            .issues
            .create(&owner, &repo)
            .body(&request)
            .send()
            .await?;

        let url = issue.html_url;
        println!("Issue created: {url}");
        Ok(CIExit::IssueCreated(url))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::Cli;

    #[test]
    fn test_create_issue_parses_required_title() {
        let args = Cli::try_parse_from([
            "pcu",
            "create-issue",
            "--title",
            "nightly build failure on main",
        ])
        .unwrap();
        match args.command {
            crate::Commands::CreateIssue(ci) => {
                assert_eq!(ci.title, "nightly build failure on main");
                assert_eq!(ci.body, "");
                assert!(ci.owner.is_none());
                assert!(ci.repo.is_none());
                assert!(ci.github_token.is_none());
                assert!(!ci.dry_run);
            }
            _ => panic!("expected CreateIssue command"),
        }
    }

    #[test]
    fn test_create_issue_parses_all_flags() {
        let args = Cli::try_parse_from([
            "pcu",
            "create-issue",
            "--title",
            "ci: nightly failure",
            "--body",
            "nightly broke on main",
            "--owner",
            "jerus-org",
            "--repo",
            "my-crate",
            "--github-token",
            "ghp_test",
            "--dry-run",
        ])
        .unwrap();
        match args.command {
            crate::Commands::CreateIssue(ci) => {
                assert_eq!(ci.title, "ci: nightly failure");
                assert_eq!(ci.body, "nightly broke on main");
                assert_eq!(ci.owner.as_deref(), Some("jerus-org"));
                assert_eq!(ci.repo.as_deref(), Some("my-crate"));
                assert_eq!(ci.github_token.as_deref(), Some("ghp_test"));
                assert!(ci.dry_run);
            }
            _ => panic!("expected CreateIssue command"),
        }
    }

    #[test]
    fn test_create_issue_requires_title() {
        let result = Cli::try_parse_from(["pcu", "create-issue"]);
        assert!(result.is_err(), "create-issue without --title should fail");
    }

    #[tokio::test]
    async fn test_create_issue_dry_run_returns_placeholder_url() {
        let cmd = super::CreateIssue {
            title: "test issue".to_string(),
            body: String::new(),
            owner: Some("jerus-org".to_string()),
            repo: Some("my-crate".to_string()),
            github_token: None,
            dry_run: true,
        };
        let result = cmd.run().await.unwrap();
        match result {
            crate::CIExit::IssueCreated(url) => {
                assert_eq!(url, "https://github.com/jerus-org/my-crate/issues/0");
            }
            _ => panic!("expected IssueCreated"),
        }
    }

    #[tokio::test]
    async fn test_create_issue_missing_owner_returns_error() {
        // Ensure env var is not set
        let cmd = super::CreateIssue {
            title: "test".to_string(),
            body: String::new(),
            owner: None,
            repo: Some("my-crate".to_string()),
            github_token: Some("ghp_fake".to_string()),
            dry_run: false,
        };
        // Remove env var if set
        std::env::remove_var("CIRCLE_PROJECT_USERNAME");
        let result = cmd.run().await;
        assert!(result.is_err(), "should fail when owner is not resolvable");
    }

    #[tokio::test]
    async fn test_create_issue_missing_repo_returns_error() {
        let cmd = super::CreateIssue {
            title: "test".to_string(),
            body: String::new(),
            owner: Some("jerus-org".to_string()),
            repo: None,
            github_token: Some("ghp_fake".to_string()),
            dry_run: false,
        };
        std::env::remove_var("CIRCLE_PROJECT_REPONAME");
        let result = cmd.run().await;
        assert!(result.is_err(), "should fail when repo is not resolvable");
    }
}
