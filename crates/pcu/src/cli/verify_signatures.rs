use super::CIExit;
use crate::ops::{
    git_signature_ops::extract_commits, signature_ops::verify_commits,
    trust_fetcher::fetch_trust_list,
};
use crate::Error;
use clap::Parser;
use octocrate::{APIConfig, GitHubAPI, PersonalAccessToken};
use owo_colors::OwoColorize;
use std::env;

#[derive(Debug, Parser, Clone)]
pub struct VerifySignatures {
    /// Base ref for commit range
    #[clap(long, default_value = "origin/main")]
    pub base: String,

    /// Head ref for commit range
    #[clap(long, default_value = "HEAD")]
    pub head: String,

    /// Repository owner (auto-detected if not provided)
    #[clap(long)]
    pub repo_owner: Option<String>,

    /// Repository name (auto-detected if not provided)
    #[clap(long)]
    pub repo_name: Option<String>,

    /// Git fetch depth
    #[clap(long, default_value = "200")]
    pub fetch_depth: usize,

    /// Fail if trusted identities have unsigned commits
    #[clap(long, default_value_t = true)]
    pub fail_on_unsigned: bool,
}

impl VerifySignatures {
    pub async fn run_verify(self) -> Result<CIExit, Error> {
        log::info!("=== Commit Signature Verification ===");

        // Get GitHub API token
        let github_token = env::var("GITHUB_TOKEN").map_err(|_| {
            Error::GpgError("GITHUB_TOKEN environment variable not set".to_string())
        })?;

        // Initialize GitHub API client
        let pat = PersonalAccessToken::new(github_token);
        let config = APIConfig::with_token(pat).shared();
        let github_rest = GitHubAPI::new(&config);

        // Open git repository
        let git_repo = git2::Repository::open(".")?;

        // Get owner and repo (auto-detect from git config if not provided)
        let (owner, repo) = if let (Some(o), Some(r)) = (&self.repo_owner, &self.repo_name) {
            (o.clone(), r.clone())
        } else {
            // Try to auto-detect from git remote
            let remote = git_repo.find_remote("origin")?;
            let url = remote
                .url()
                .ok_or_else(|| Error::GitError("No URL for origin remote".to_string()))?;

            // Parse GitHub URL (https://github.com/owner/repo.git or git@github.com:owner/repo.git)
            let (parsed_owner, parsed_repo) = parse_github_url(url)?;

            (
                self.repo_owner.unwrap_or(parsed_owner),
                self.repo_name.unwrap_or(parsed_repo),
            )
        };

        println!("\n{}\n", "=== Commit Signature Verification ===".bold());
        println!("Repository: {owner}/{repo}\n");

        // Step 1: Fetch trust list from GitHub
        log::info!("Fetching trust list from GitHub...");
        let trust_map = fetch_trust_list(&github_rest, &owner, &repo).await?;

        // Step 2: Extract commits from git
        log::info!("Extracting commits from {}..{}", self.base, self.head);
        let commits = extract_commits(&git_repo, &self.base, &self.head)?;

        if commits.is_empty() {
            println!("{}\n", "ℹ  No commits to verify".yellow());
            return Ok(CIExit::VerificationPassed);
        }

        println!("ℹ  Found {} commit(s) to verify\n", commits.len());

        // Step 3: Verify commits
        let (results, summary) = verify_commits(commits, &trust_map);

        // Step 4: Display results (privacy-safe)
        for result in &results {
            let sha_short = &result.commit.sha[..8.min(result.commit.sha.len())];
            let status_symbol = if result.passed {
                "✓".green().bold().to_string()
            } else {
                "✗".red().bold().to_string()
            };

            println!(
                "{} {}   {} {}",
                status_symbol,
                if result.passed { "OK" } else { "FAIL" },
                sha_short.bold(),
                result.commit.subject
            );

            // Show reason (privacy-safe, no PII)
            println!("    {}", result.reason.display_message());
            println!();
        }

        // Step 5: Display summary
        println!("{}\n", "=== Verification Summary ===".bold());
        println!("Commits checked:         {}", summary.commits_checked);
        println!(
            "Trusted verified:        {}",
            summary.trusted_verified.to_string().green()
        );
        println!(
            "External contributors:   {}",
            summary.external_contributors.to_string().green()
        );

        if summary.failures > 0 {
            println!(
                "Failures:                {}\n",
                summary.failures.to_string().red().bold()
            );
            println!("{}", "✗ Signature verification FAILED!".red().bold());
            println!("\n{}:", "Action required".bold());
            println!("  - Review failed commits immediately");
            println!("  - Verify the committer's identity");
            println!("  - Do NOT merge if impersonation is suspected\n");

            if self.fail_on_unsigned {
                return Ok(CIExit::VerificationFailed);
            }
        } else {
            println!();
            println!("{}", "✓ All signature checks passed!".green().bold());
            println!("\nNo impersonation attempts detected.\n");
        }

        Ok(if summary.failures == 0 {
            CIExit::VerificationPassed
        } else {
            CIExit::VerificationFailed
        })
    }
}

/// Parse GitHub URL to extract owner and repo
/// Supports both HTTPS and SSH formats
fn parse_github_url(url: &str) -> Result<(String, String), Error> {
    // Remove .git suffix if present
    let url = url.trim_end_matches(".git");

    if url.starts_with("https://github.com/") {
        // HTTPS format: https://github.com/owner/repo
        let parts: Vec<&str> = url
            .trim_start_matches("https://github.com/")
            .split('/')
            .collect();
        if parts.len() >= 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    } else if url.starts_with("git@github.com:") {
        // SSH format: git@github.com:owner/repo
        let parts: Vec<&str> = url
            .trim_start_matches("git@github.com:")
            .split('/')
            .collect();
        if parts.len() >= 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    Err(Error::GitError(format!(
        "Unable to parse GitHub URL: {url}"
    )))
}
