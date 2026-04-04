use clap::Parser;
use color_eyre::Result;
use octocrate::issues;
use octocrate::{APIConfig, GitHubAPI, PersonalAccessToken};
use std::env;

use super::CIExit;
use crate::Error;

#[derive(Debug, Parser, Clone)]
/// Post a markdown comment on the current pull request
pub struct CommentPr {
    /// Body of the comment (markdown).
    #[clap(long, default_value = "")]
    pub body: String,

    /// PR number to comment on. Defaults to CIRCLE_PR_NUMBER env var, or
    /// parsed from CIRCLE_PULL_REQUEST URL.
    #[clap(long)]
    pub pr_number: Option<u64>,

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

impl CommentPr {
    /// Resolve the PR number from (in priority order): the explicit flag,
    /// the CIRCLE_PR_NUMBER env var value, or the CIRCLE_PULL_REQUEST URL.
    pub(crate) fn resolve_pr_number(
        explicit: Option<u64>,
        circle_pr_number: Option<String>,
        circle_pull_request: Option<String>,
    ) -> Result<u64, Error> {
        explicit
            .map(Ok)
            .or_else(|| {
                circle_pr_number.map(|s| {
                    s.parse::<u64>()
                        .map_err(|_| Error::MissingConfig("could not parse CIRCLE_PR_NUMBER".to_string()))
                })
            })
            .or_else(|| {
                circle_pull_request.map(|url| {
                    url.split('/')
                        .next_back()
                        .and_then(|n| n.parse::<u64>().ok())
                        .ok_or_else(|| Error::MissingConfig("could not parse PR number from CIRCLE_PULL_REQUEST".to_string()))
                })
            })
            .ok_or_else(|| {
                Error::MissingConfig(
                    "pr-number not provided and neither CIRCLE_PR_NUMBER nor CIRCLE_PULL_REQUEST is set".to_string(),
                )
            })?
    }

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

        let pr_number = Self::resolve_pr_number(
            self.pr_number,
            env::var("CIRCLE_PR_NUMBER").ok(),
            env::var("CIRCLE_PULL_REQUEST").ok(),
        )?;

        if self.dry_run {
            log::info!(
                "Dry run: would comment on {owner}/{repo}#{pr_number}: {}",
                self.body
            );
            return Ok(CIExit::PrCommentCreated(format!(
                "https://github.com/{owner}/{repo}/pull/{pr_number}"
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

        let request = issues::create_comment::Request {
            body: self.body.clone(),
        };

        let comment = api
            .issues
            .create_comment(&owner, &repo, pr_number as i64)
            .body(&request)
            .send()
            .await?;

        let url = comment.html_url;
        println!("PR comment created: {url}");
        Ok(CIExit::PrCommentCreated(url))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::Cli;

    use super::*;

    #[test]
    fn test_comment_pr_parses_with_no_flags() {
        let args = Cli::try_parse_from(["pcu", "comment-pr"]).unwrap();
        match args.command {
            crate::Commands::CommentPr(cmd) => {
                assert_eq!(cmd.body, "");
                assert!(cmd.pr_number.is_none());
                assert!(cmd.owner.is_none());
                assert!(cmd.repo.is_none());
                assert!(cmd.github_token.is_none());
                assert!(!cmd.dry_run);
            }
            _ => panic!("expected CommentPr command"),
        }
    }

    #[test]
    fn test_comment_pr_parses_all_flags() {
        let args = Cli::try_parse_from([
            "pcu",
            "comment-pr",
            "--body",
            "nightly broke",
            "--pr-number",
            "42",
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
            crate::Commands::CommentPr(cmd) => {
                assert_eq!(cmd.body, "nightly broke");
                assert_eq!(cmd.pr_number, Some(42));
                assert_eq!(cmd.owner.as_deref(), Some("jerus-org"));
                assert_eq!(cmd.repo.as_deref(), Some("my-crate"));
                assert_eq!(cmd.github_token.as_deref(), Some("ghp_test"));
                assert!(cmd.dry_run);
            }
            _ => panic!("expected CommentPr command"),
        }
    }

    #[tokio::test]
    async fn test_comment_pr_dry_run_returns_placeholder_url() {
        let cmd = CommentPr {
            body: "test comment".to_string(),
            pr_number: Some(99),
            owner: Some("jerus-org".to_string()),
            repo: Some("my-crate".to_string()),
            github_token: None,
            dry_run: true,
        };
        let result = cmd.run().await.unwrap();
        match result {
            crate::CIExit::PrCommentCreated(url) => {
                assert_eq!(url, "https://github.com/jerus-org/my-crate/pull/99");
            }
            _ => panic!("expected PrCommentCreated"),
        }
    }

    #[test]
    fn test_resolve_pr_number_from_circle_pr_number() {
        let result = CommentPr::resolve_pr_number(None, Some("88".to_string()), None);
        assert_eq!(result.unwrap(), 88);
    }

    #[test]
    fn test_resolve_pr_number_from_circle_pull_request_url() {
        let result = CommentPr::resolve_pr_number(
            None,
            None,
            Some("https://github.com/jerus-org/my-crate/pull/55".to_string()),
        );
        assert_eq!(result.unwrap(), 55);
    }

    #[test]
    fn test_resolve_pr_number_explicit_takes_precedence() {
        let result = CommentPr::resolve_pr_number(
            Some(99),
            Some("88".to_string()),
            Some("https://github.com/jerus-org/my-crate/pull/55".to_string()),
        );
        assert_eq!(result.unwrap(), 99);
    }

    #[test]
    fn test_resolve_pr_number_circle_pr_number_beats_url() {
        let result = CommentPr::resolve_pr_number(
            None,
            Some("88".to_string()),
            Some("https://github.com/jerus-org/my-crate/pull/55".to_string()),
        );
        assert_eq!(result.unwrap(), 88);
    }

    #[test]
    fn test_resolve_pr_number_none_returns_error() {
        let result = CommentPr::resolve_pr_number(None, None, None);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_comment_pr_missing_pr_number_returns_error() {
        std::env::remove_var("CIRCLE_PR_NUMBER");
        std::env::remove_var("CIRCLE_PULL_REQUEST");
        let cmd = CommentPr {
            body: String::new(),
            pr_number: None,
            owner: Some("jerus-org".to_string()),
            repo: Some("my-crate".to_string()),
            github_token: Some("ghp_fake".to_string()),
            dry_run: false,
        };
        assert!(cmd.run().await.is_err());
    }

    #[tokio::test]
    async fn test_comment_pr_missing_owner_returns_error() {
        std::env::remove_var("CIRCLE_PROJECT_USERNAME");
        let cmd = CommentPr {
            body: String::new(),
            pr_number: Some(1),
            owner: None,
            repo: Some("my-crate".to_string()),
            github_token: Some("ghp_fake".to_string()),
            dry_run: false,
        };
        assert!(cmd.run().await.is_err());
    }

    #[tokio::test]
    async fn test_comment_pr_missing_repo_returns_error() {
        std::env::remove_var("CIRCLE_PROJECT_REPONAME");
        let cmd = CommentPr {
            body: String::new(),
            pr_number: Some(1),
            owner: Some("jerus-org".to_string()),
            repo: None,
            github_token: Some("ghp_fake".to_string()),
            dry_run: false,
        };
        assert!(cmd.run().await.is_err());
    }
}
