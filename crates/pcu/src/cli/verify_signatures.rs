use std::{collections::HashMap, env};

use clap::Parser;
use octocrab::Octocrab;
use owo_colors::OwoColorize;

use super::CIExit;
use crate::{
    ops::{
        git_signature_ops::extract_commits,
        signature_ops::verify_commits_with,
        trust_fetcher::{fetch_trust_list, import_allow_list_keys},
        verify_config::VerifyConfig,
    },
    Error,
};

#[derive(Parser, Clone)]
pub struct VerifySignatures {
    /// Base ref for commit range
    #[clap(long, default_value = "origin/main")]
    pub base: String,

    /// Ref the verification policy (`pcu.toml`) is read from: the allow list
    /// and the hard stop. Deliberately separate from `--base`, which only
    /// chooses where the commit range starts: `--base HEAD~3` must not make the
    /// policy come from a commit that predates it. Must be a ref the branch
    /// under review cannot change, normally the default branch.
    #[clap(long, default_value = "origin/main")]
    pub policy_ref: String,

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

    /// Deprecated and always on: this switch cannot be turned off from the
    /// command line, and a failure always fails the run. Kept so existing
    /// command lines keep working. It used to decide only whether the PR
    /// comment was posted before failing.
    #[clap(long, default_value_t = true)]
    pub fail_on_unsigned: bool,

    /// Fail for commits from authors who are neither trusted nor on the allow
    /// list. Can also be set with `fail_on_external = true` under
    /// `[verify_signatures]` in `pcu.toml` at the base ref, which a branch
    /// cannot change.
    #[clap(long)]
    pub fail_on_external: bool,

    /// Post verification results as a PR comment
    #[clap(long)]
    pub post_comment: bool,

    /// PR number to comment on (required if --post-comment is used)
    #[clap(long, required_if_eq("post_comment", "true"))]
    pub pr_number: Option<u64>,

    /// GitHub token for the read-only work (trust list, keys, PR opener).
    /// If not provided, GITHUB_TOKEN is used. Ignored when a GitHub App is
    /// configured (`PCU_APP_ID` and `PCU_PRIVATE_KEY`), which is preferred for
    /// reading because it can be read-only. The PR comment always uses
    /// GITHUB_TOKEN.
    #[clap(long)]
    pub github_token: Option<String>,
}

impl std::fmt::Debug for VerifySignatures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifySignatures")
            .field("base", &self.base)
            .field("policy_ref", &self.policy_ref)
            .field("head", &self.head)
            .field("repo_owner", &self.repo_owner)
            .field("repo_name", &self.repo_name)
            .field("fetch_depth", &self.fetch_depth)
            .field("fail_on_unsigned", &self.fail_on_unsigned)
            .field("fail_on_external", &self.fail_on_external)
            .field("post_comment", &self.post_comment)
            .field("pr_number", &self.pr_number)
            .field("github_token", &super::Redacted(&self.github_token))
            .finish()
    }
}

/// Longest author email or subject written into the PR comment.
const MAX_UNTRUSTED_LEN: usize = 120;

/// Make attacker-controlled text safe inside a markdown code span (the author
/// email in the table). Control characters, backticks and pipes would end the
/// span, add a table column or start a new line, letting a commit author
/// render their own markdown in a comment posted under the bot's name.
fn sanitize_code_span(text: &str) -> String {
    let mut out: String = text
        .chars()
        .take(MAX_UNTRUSTED_LEN)
        .map(|c| {
            if c.is_control() || c == '`' || c == '|' {
                '?'
            } else {
                c
            }
        })
        .collect();
    if text.chars().count() > MAX_UNTRUSTED_LEN {
        out.push('…');
    }
    out
}

/// Make attacker-controlled text safe as plain markdown text (the commit
/// subject): neutralise markup, links, images, HTML and `@mentions`, and keep
/// it on one line.
fn escape_markdown_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().take(MAX_UNTRUSTED_LEN) {
        match c {
            c if c.is_control() => out.push(' '),
            '@' => out.push_str("@\u{200B}"),
            '\\' | '`' | '*' | '_' | '[' | ']' | '(' | ')' | '<' | '>' | '|' | '~' | '!' | '#'
            | '&' => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    if text.chars().count() > MAX_UNTRUSTED_LEN {
        out.push('…');
    }
    out
}

/// Replace control characters (terminal escape sequences) in text printed to
/// the CI log.
fn strip_control(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

impl VerifySignatures {
    pub async fn run_verify(self) -> Result<CIExit, Error> {
        log::info!("=== Commit Signature Verification ===");

        // Open git repository
        let git_repo = git2::Repository::open(".")?;

        // Get owner and repo (auto-detect from git config if not provided)
        let (owner, repo) = detect_repository(&git_repo, &self.repo_owner, &self.repo_name)?;

        // The policy comes from the policy ref, never the branch under review:
        // a PR must not be able to allow-list itself or lower the minimum.
        let mut policy = resolve_policy(&git_repo, &self.policy_ref, self.fail_on_external)?;
        log::info!(
            "Policy from {}: {} allow-list entr{}, hard stop for external contributors {}, \
             at least {} trusted collaborator(s)",
            self.policy_ref,
            policy.allow.len(),
            if policy.allow.len() == 1 { "y" } else { "ies" },
            if policy.fail_on_external { "on" } else { "off" },
            policy.min_trusted
        );

        // Reads use a read-only GitHub App when one is configured, else a token.
        // The collaborator list needs a caller with write access, which an App
        // does not have, so `PCU_COLLAB_READ` (a read-only token) lists it.
        // The PR comment below uses `GITHUB_TOKEN`: reads and writes never
        // share a credential.
        let settings = env_settings(None)?;
        let env_token = env::var("GITHUB_TOKEN").ok();
        let token = self.github_token.as_deref().or(env_token.as_deref());
        let readonly_client = match choose_read_auth(&settings, token)? {
            ReadAuth::App(app_id) => {
                log::info!("Reading from GitHub as the GitHub App (PCU_APP_ID)");
                crate::client::app_installation_client(&app_id, &settings, &owner, &repo, true)
                    .await?
                    .0
            }
            ReadAuth::Token(token) => {
                log::info!("Reading from GitHub with a token (no GitHub App configured)");
                Octocrab::builder().personal_token(token).build()?
            }
        };
        let collaborators_client = match collaborator_token(&settings) {
            Some(token) => {
                log::info!("Listing collaborators with PCU_COLLAB_READ");
                Octocrab::builder().personal_token(token).build()?
            }
            None => readonly_client.clone(),
        };

        println!("\n{}\n", "=== Commit Signature Verification ===".bold());
        println!("Repository: {owner}/{repo}\n");

        // Step 1: Fetch trust list from GitHub
        log::info!("Fetching trust list from GitHub...");
        let trust_map = fetch_trust_list(
            &collaborators_client,
            &readonly_client,
            &owner,
            &repo,
            policy.min_trusted,
        )
        .await?;

        // Allow-listed keys must be in the keyring for git to verify them.
        import_allow_list_keys(&readonly_client, &policy).await?;

        // Entries that name GitHub's web-flow key also need the login that
        // opened the PR, which GitHub authenticates. Only look it up when an
        // entry needs it. Unknown stays None, and those entries then fail.
        if policy.needs_pr_author() {
            let pr = self.pr_number.or_else(pr_number_from_env);
            policy.pr_author = match pr {
                Some(number) => fetch_pr_author(&readonly_client, &owner, &repo, number).await,
                None => {
                    log::warn!(
                        "No pull request number (--pr-number or CIRCLE_PULL_REQUEST): \
                         allow-list entries that require a PR author will fail"
                    );
                    None
                }
            };
        }

        // Step 2: Extract commits from git
        log::info!("Extracting commits from {}..{}", self.base, self.head);
        let commits = extract_commits(&git_repo, &self.base, &self.head)?;

        if commits.is_empty() {
            println!("{}\n", "ℹ  No commits to verify".yellow());
            return Ok(CIExit::VerificationPassed);
        }

        println!("ℹ  Found {} commit(s) to verify\n", commits.len());

        // Step 3: Verify commits against the trust map and the policy
        let (results, summary) = verify_commits_with(commits, &trust_map, &policy);

        // Step 4: Display results (privacy-safe)
        display_results(&results);

        // Step 5: Display summary
        display_summary(&summary, &results);

        // Step 6: Post PR comment if requested
        if self.post_comment {
            if let Some(pr_number) = self.pr_number {
                log::info!("Posting verification results to PR #{pr_number}");

                // Create a separate client with write access for posting
                // comments This requires GITHUB_TOKEN env var
                // with write permissions
                match initialize_github_client_from_env() {
                    Ok(write_client) => {
                        if let Err(e) = post_verification_comment(
                            &write_client,
                            &owner,
                            &repo,
                            pr_number,
                            &results,
                            &summary,
                        )
                        .await
                        {
                            log::warn!("Failed to post PR comment: {e}");
                            eprintln!("⚠  Warning: Failed to post PR comment: {e}");
                        }
                    }
                    Err(e) => {
                        log::warn!("Cannot post PR comment - GITHUB_TOKEN env var not set for write access: {e}");
                        eprintln!("⚠  Warning: Cannot post PR comment - GITHUB_TOKEN env var required for write access");
                    }
                }
            }
        }

        // Fail only after the comment is posted, so a failing run replaces
        // an earlier "success" comment instead of leaving it on the PR.
        finish(&summary)
    }
}

/// The run's outcome: any failure fails it. Nothing, including the deprecated
/// `--fail-on-unsigned`, can turn a failure into a pass.
fn finish(summary: &crate::ops::signature_ops::VerificationSummary) -> Result<CIExit, Error> {
    if summary.failures > 0 {
        Err(Error::SignatureVerificationFailed(summary.failures))
    } else {
        Ok(CIExit::VerificationPassed)
    }
}

/// Read the policy from `policy_ref` and apply the `--fail-on-external` flag,
/// which can only switch the hard stop on, never off.
fn resolve_policy(
    repo: &git2::Repository,
    policy_ref: &str,
    fail_on_external_flag: bool,
) -> Result<VerifyConfig, Error> {
    let mut policy = VerifyConfig::load_from_ref(repo, policy_ref)?;
    if fail_on_external_flag {
        policy.fail_on_external = true;
    }
    Ok(policy)
}

/// The pull request number from CircleCI's `CIRCLE_PULL_REQUEST` URL.
fn pr_number_from_env() -> Option<u64> {
    parse_pr_number(&env::var("CIRCLE_PULL_REQUEST").ok()?)
}

/// The number at the end of a pull request URL.
fn parse_pr_number(url: &str) -> Option<u64> {
    url.trim()
        .trim_end_matches('/')
        .rsplit('/')
        .next()?
        .parse()
        .ok()
}

/// The GitHub login that opened a pull request, or `None` if it cannot be
/// determined (the failure is logged without the response body).
async fn fetch_pr_author(
    github: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
) -> Option<String> {
    match github.pulls(owner, repo).get(number).await {
        Ok(pr) => pr.user.map(|user| user.login),
        Err(e) => {
            log::warn!("Could not look up who opened PR #{number}: {e}");
            None
        }
    }
}

/// Post verification results as a PR comment
async fn post_verification_comment(
    github: &Octocrab,
    owner: &str,
    repo: &str,
    pr_number: u64,
    results: &[crate::ops::signature_ops::VerificationResult],
    summary: &crate::ops::signature_ops::VerificationSummary,
) -> Result<(), Error> {
    let comment = build_comment(results, summary)?;
    post_comment_to_github(github, owner, repo, pr_number, &comment).await?;
    Ok(())
}

/// Build the verification comment body
fn build_comment(
    results: &[crate::ops::signature_ops::VerificationResult],
    summary: &crate::ops::signature_ops::VerificationSummary,
) -> Result<String, Error> {
    use std::fmt::Write;

    let mut comment = String::new();
    let write_err = |e| Error::GpgError(format!("Failed to write comment: {e}"));

    if summary.failures > 0 {
        writeln!(&mut comment, "## ❌ Commit Signature Verification - FAILED")
            .map_err(write_err)?;
        writeln!(&mut comment).map_err(write_err)?;
        writeln!(
            &mut comment,
            "**⚠️ Security Alert:** One or more commits failed signature verification."
        )
        .map_err(write_err)?;
        writeln!(&mut comment).map_err(write_err)?;

        // List failed commits
        writeln!(&mut comment, "### Failed Commits").map_err(write_err)?;
        writeln!(&mut comment).map_err(write_err)?;

        write_failed_commits(&mut comment, results).map_err(write_err)?;

        writeln!(&mut comment).map_err(write_err)?;
        write_summary(&mut comment, summary, true).map_err(write_err)?;

        if summary.external_contributors > 0 {
            writeln!(&mut comment).map_err(write_err)?;
            write_external_contributors(&mut comment, results).map_err(write_err)?;
        }

        if summary.allow_listed > 0 {
            writeln!(&mut comment).map_err(write_err)?;
            write_allow_listed(&mut comment, results).map_err(write_err)?;
        }

        writeln!(&mut comment).map_err(write_err)?;
        write_action_required(&mut comment).map_err(write_err)?;
    } else {
        writeln!(
            &mut comment,
            "## ✅ Commit Signature Verification - Success"
        )
        .map_err(write_err)?;
        writeln!(&mut comment).map_err(write_err)?;
        writeln!(&mut comment, "All commits have been verified successfully.")
            .map_err(write_err)?;
        writeln!(&mut comment).map_err(write_err)?;
        write_summary(&mut comment, summary, false).map_err(write_err)?;

        if summary.external_contributors > 0 {
            writeln!(&mut comment).map_err(write_err)?;
            write_external_contributors(&mut comment, results).map_err(write_err)?;
        }

        if summary.allow_listed > 0 {
            writeln!(&mut comment).map_err(write_err)?;
            write_allow_listed(&mut comment, results).map_err(write_err)?;
        }

        writeln!(&mut comment).map_err(write_err)?;
        writeln!(&mut comment, "No impersonation attempts detected.").map_err(write_err)?;
    }

    Ok(comment)
}

/// Write failed commits section
fn write_failed_commits(
    comment: &mut String,
    results: &[crate::ops::signature_ops::VerificationResult],
) -> std::fmt::Result {
    use std::fmt::Write;

    for result in results.iter().filter(|r| !r.passed) {
        let sha_short = &result.commit.sha[..8.min(result.commit.sha.len())];
        writeln!(
            comment,
            "- `{}` {} - {}",
            sha_short,
            escape_markdown_text(&result.commit.subject),
            result.reason.display_message()
        )?;
    }
    Ok(())
}

/// Write summary section
fn write_summary(
    comment: &mut String,
    summary: &crate::ops::signature_ops::VerificationSummary,
    include_failures: bool,
) -> std::fmt::Result {
    use std::fmt::Write;

    writeln!(comment, "### Summary")?;
    writeln!(
        comment,
        "- **Commits checked**: {}",
        summary.commits_checked
    )?;
    writeln!(
        comment,
        "- **Trusted verified**: {}",
        summary.trusted_verified
    )?;
    if summary.allow_listed > 0 {
        writeln!(comment, "- **Allow-listed**: {}", summary.allow_listed)?;
    }
    writeln!(
        comment,
        "- **External contributors**: {}",
        summary.external_contributors
    )?;
    if include_failures {
        writeln!(comment, "- **Failures**: ❗ {}", summary.failures)?;
    }
    Ok(())
}

/// Write action required section
fn write_action_required(comment: &mut String) -> std::fmt::Result {
    use std::fmt::Write;

    writeln!(comment, "### Action Required")?;
    writeln!(comment, "- Review the failed commits immediately")?;
    writeln!(comment, "- Verify the committer's identity")?;
    writeln!(comment, "- **DO NOT MERGE** if impersonation is suspected")?;
    Ok(())
}

/// Whether a verification result is for an author who is neither a trusted
/// identity nor allow-listed.
fn is_external(reason: &crate::ops::signature_ops::VerificationReason) -> bool {
    use crate::ops::signature_ops::VerificationReason;

    matches!(
        reason,
        VerificationReason::ExternalUnsigned
            | VerificationReason::ExternalInvalidSignature
            | VerificationReason::ExternalNotAllowed { .. }
    )
}

/// Write external contributors table section. Writes nothing when there are
/// no external contributors.
fn write_external_contributors(
    comment: &mut String,
    results: &[crate::ops::signature_ops::VerificationResult],
) -> std::fmt::Result {
    use std::fmt::Write;

    use crate::ops::signature_ops::VerificationReason;

    let external: Vec<_> = results.iter().filter(|r| is_external(&r.reason)).collect();
    if external.is_empty() {
        return Ok(());
    }

    writeln!(comment, "### External Contributors")?;
    writeln!(comment)?;
    writeln!(comment, "| Commit | Author | Status |")?;
    writeln!(comment, "|--------|--------|--------|")?;

    for result in external {
        let sha_short = &result.commit.sha[..8.min(result.commit.sha.len())];
        let status = match &result.reason {
            VerificationReason::ExternalNotAllowed { .. } => "❌ Not on the allow list",
            VerificationReason::ExternalInvalidSignature => {
                "⚠️ Invalid signature (bad, expired or revoked key)"
            }
            _ => "Unsigned (expected for bots/external)",
        };
        writeln!(
            comment,
            "| `{}` | `{}` | {} |",
            sha_short,
            sanitize_code_span(&result.commit.author_email),
            status
        )?;
    }
    Ok(())
}

/// Write the allow-listed identities table. Writes nothing when none matched.
fn write_allow_listed(
    comment: &mut String,
    results: &[crate::ops::signature_ops::VerificationResult],
) -> std::fmt::Result {
    use std::fmt::Write;

    use crate::ops::signature_ops::VerificationReason;

    let allowed: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.reason, VerificationReason::AllowListed))
        .collect();
    if allowed.is_empty() {
        return Ok(());
    }

    writeln!(comment, "### Allow-listed Identities")?;
    writeln!(comment)?;
    writeln!(comment, "| Commit | Author | Signing key |")?;
    writeln!(comment, "|--------|--------|-------------|")?;

    for result in allowed {
        let sha_short = &result.commit.sha[..8.min(result.commit.sha.len())];
        writeln!(
            comment,
            "| `{}` | `{}` | `{}` |",
            sha_short,
            sanitize_code_span(&result.commit.author_email),
            sanitize_code_span(result.commit.key_id.as_deref().unwrap_or(""))
        )?;
    }
    Ok(())
}

/// Post comment to GitHub using octocrab's issues API
/// Updates existing comment if found, otherwise creates a new one
async fn post_comment_to_github(
    github: &Octocrab,
    owner: &str,
    repo: &str,
    pr_number: u64,
    comment: &str,
) -> Result<(), Error> {
    // First, try to find an existing verification comment
    let existing_comment_id =
        find_existing_verification_comment(github, owner, repo, pr_number).await?;

    if let Some(comment_id) = existing_comment_id {
        // Update existing comment
        log::info!("Updating existing PR comment (ID: {comment_id})");
        github
            .issues(owner, repo)
            .update_comment(comment_id, comment)
            .await
            .map_err(|e| Error::GpgError(format!("Failed to update PR comment: {e}")))?;
    } else {
        // Create new comment
        log::info!("Creating new PR comment");
        github
            .issues(owner, repo)
            .create_comment(pr_number, comment)
            .await
            .map_err(|e| Error::GpgError(format!("Failed to create PR comment: {e}")))?;
    }

    Ok(())
}

/// Find existing verification comment on the PR
/// Returns the comment ID if found
async fn find_existing_verification_comment(
    github: &Octocrab,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> Result<Option<octocrab::models::CommentId>, Error> {
    // List all comments on the PR
    let comments = github
        .issues(owner, repo)
        .list_comments(pr_number)
        .send()
        .await
        .map_err(|e| Error::GpgError(format!("Failed to list PR comments: {e}")))?;

    // Look for a comment containing our signature header
    for comment in comments {
        if let Some(body) = &comment.body {
            if body.contains("## ❌ Commit Signature Verification")
                || body.contains("## ✅ Commit Signature Verification")
            {
                return Ok(Some(comment.id));
            }
        }
    }

    Ok(None)
}

/// Initialize GitHub API client from GITHUB_TOKEN env var
fn initialize_github_client_from_env() -> Result<Octocrab, Error> {
    let github_token = env::var("GITHUB_TOKEN")
        .map_err(|_| Error::GpgError("GITHUB_TOKEN environment variable not set".to_string()))?;

    Ok(Octocrab::builder().personal_token(github_token).build()?)
}

/// How the read-only client authenticates.
#[derive(Debug, PartialEq, Eq)]
enum ReadAuth {
    /// The GitHub App with this id (`PCU_APP_ID`, with `PCU_PRIVATE_KEY`).
    App(String),
    /// A personal or fine-grained token.
    Token(String),
}

/// Settings from the environment (`PCU_APP_ID`, `PCU_PRIVATE_KEY`,
/// `PCU_COLLAB_READ`), never from `pcu.toml`: a file on the branch under review
/// must not choose the credential. `env` replaces the process environment in
/// tests.
fn env_settings(env: Option<HashMap<String, String>>) -> Result<config::Config, Error> {
    Ok(config::Config::builder()
        .add_source(
            config::Environment::with_prefix("PCU")
                .ignore_empty(true)
                .source(env),
        )
        .build()?)
}

/// Pick the credential for the read-only work: the GitHub App when one is
/// configured, otherwise the token. The App wins even when a token is also
/// supplied. A blank value counts as unset.
fn choose_read_auth(settings: &config::Config, token: Option<&str>) -> Result<ReadAuth, Error> {
    if let Some(app_id) = non_blank(settings, "app_id") {
        return Ok(ReadAuth::App(app_id));
    }
    match token.map(str::trim).filter(|t| !t.is_empty()) {
        Some(token) => Ok(ReadAuth::Token(token.to_string())),
        None => Err(Error::GpgError(
            "No GitHub credential for reading: set PCU_APP_ID and PCU_PRIVATE_KEY \
             (a read-only GitHub App), or use --github-token or GITHUB_TOKEN"
                .to_string(),
        )),
    }
}

/// The token that lists collaborators (`PCU_COLLAB_READ`), when set.
fn collaborator_token(settings: &config::Config) -> Option<String> {
    non_blank(settings, "collab_read")
}

/// A setting's value with surrounding whitespace removed, or `None` when it is
/// missing or blank.
fn non_blank(settings: &config::Config, key: &str) -> Option<String> {
    settings
        .get::<String>(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Detect repository owner and name from git config or CLI args
fn detect_repository(
    git_repo: &git2::Repository,
    repo_owner: &Option<String>,
    repo_name: &Option<String>,
) -> Result<(String, String), Error> {
    if let (Some(o), Some(r)) = (repo_owner, repo_name) {
        return Ok((o.clone(), r.clone()));
    }

    // Try to auto-detect from git remote
    let remote = git_repo.find_remote("origin")?;
    let url = remote
        .url()
        .map_err(|e| Error::GitError(format!("No valid URL for origin remote: {e}")))?;

    let (parsed_owner, parsed_repo) = parse_github_url(url)?;

    Ok((
        repo_owner.clone().unwrap_or(parsed_owner),
        repo_name.clone().unwrap_or(parsed_repo),
    ))
}

/// Display verification results for each commit
fn display_results(results: &[crate::ops::signature_ops::VerificationResult]) {
    for result in results {
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
            strip_control(&result.commit.subject)
        );

        // Show reason (privacy-safe, no PII)
        println!("    {}", result.reason.display_message());
        println!();
    }
}

/// Display verification summary
fn display_summary(
    summary: &crate::ops::signature_ops::VerificationSummary,
    results: &[crate::ops::signature_ops::VerificationResult],
) {
    println!("{}\n", "=== Verification Summary ===".bold());
    println!("Commits checked:         {}", summary.commits_checked);
    println!(
        "Trusted verified:        {}",
        summary.trusted_verified.to_string().green()
    );
    if summary.allow_listed > 0 {
        println!(
            "Allow-listed:            {}",
            summary.allow_listed.to_string().green()
        );
    }
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
    } else {
        println!();
        println!("{}", "✓ All signature checks passed!".green().bold());
        println!("\nNo impersonation attempts detected.\n");
    }

    // List external contributors with their identities
    let external: Vec<_> = results.iter().filter(|r| is_external(&r.reason)).collect();

    if !external.is_empty() {
        println!("{}", "External Contributors:".bold());
        for result in external {
            let sha_short = &result.commit.sha[..8.min(result.commit.sha.len())];
            println!(
                "  {} {}",
                sha_short.bold(),
                strip_control(&result.commit.author_email)
            );
        }
        println!();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::signature_ops::{
        CommitInfo, SignatureStatus, VerificationReason, VerificationResult, VerificationSummary,
    };

    fn make_external_result(sha: &str, email: &str) -> VerificationResult {
        VerificationResult {
            commit: CommitInfo {
                sha: sha.to_string(),
                author_email: email.to_string(),
                author_name: "External User".to_string(),
                subject: "external commit".to_string(),
                signature_status: SignatureStatus::None,
                key_id: None,
                signer: None,
            },
            passed: true,
            reason: VerificationReason::ExternalUnsigned,
        }
    }

    /// RED: issue #862 — success comment must include "External Contributors"
    /// section when there are external contributor results
    #[test]
    fn test_build_comment_includes_external_contributors_section() {
        let results = vec![make_external_result(
            "a1b2c3d4e5f6",
            "renovate[bot]@users.noreply.github.com",
        )];
        let summary = VerificationSummary {
            commits_checked: 1,
            trusted_verified: 0,
            allow_listed: 0,
            external_contributors: 1,
            failures: 0,
        };

        let comment = build_comment(&results, &summary).unwrap();
        assert!(
            comment.contains("External Contributors"),
            "Comment should contain 'External Contributors' section, got:\n{comment}"
        );
    }

    /// RED: issue #862 — success comment must include author email of external
    /// contributor
    #[test]
    fn test_build_comment_includes_external_contributor_email() {
        let results = vec![make_external_result(
            "a1b2c3d4e5f6",
            "renovate[bot]@users.noreply.github.com",
        )];
        let summary = VerificationSummary {
            commits_checked: 1,
            trusted_verified: 0,
            allow_listed: 0,
            external_contributors: 1,
            failures: 0,
        };

        let comment = build_comment(&results, &summary).unwrap();
        assert!(
            comment.contains("renovate[bot]@users.noreply.github.com"),
            "Comment should include external contributor email, got:\n{comment}"
        );
    }

    /// RED: issue #862 — success comment must include short SHA of external
    /// contributor commit
    #[test]
    fn test_build_comment_includes_external_contributor_sha() {
        let results = vec![make_external_result(
            "a1b2c3d4e5f6",
            "renovate[bot]@users.noreply.github.com",
        )];
        let summary = VerificationSummary {
            commits_checked: 1,
            trusted_verified: 0,
            allow_listed: 0,
            external_contributors: 1,
            failures: 0,
        };

        let comment = build_comment(&results, &summary).unwrap();
        // short SHA = first 8 chars
        assert!(
            comment.contains("a1b2c3d4"),
            "Comment should include short SHA of external contributor, got:\n{comment}"
        );
    }

    /// Verify that the comment does NOT include external contributor section
    /// when there are no external contributors (regression guard)
    #[test]
    fn test_build_comment_no_external_contributors_no_section() {
        let results = vec![];
        let summary = VerificationSummary {
            commits_checked: 0,
            trusted_verified: 0,
            allow_listed: 0,
            external_contributors: 0,
            failures: 0,
        };

        let comment = build_comment(&results, &summary).unwrap();
        assert!(
            !comment.contains("### External Contributors"),
            "Comment should not contain external contributors section when there are none, got:\n{comment}"
        );
    }

    fn summary_for(failures: usize, external: usize) -> VerificationSummary {
        VerificationSummary {
            commits_checked: failures + external,
            trusted_verified: 0,
            allow_listed: 0,
            external_contributors: external,
            failures,
        }
    }

    /// An author email that closes the code span must not be able to render
    /// markup (a link, an image) in the comment posted under the bot's name.
    #[test]
    fn test_comment_neutralises_markdown_in_author_email() {
        let hostile = "a`[Approve](https://evil.example)`@b.test";
        let comment = build_comment(
            &[make_external_result("a1b2c3d4e5f6", hostile)],
            &summary_for(0, 1),
        )
        .unwrap();

        let row = comment
            .lines()
            .find(|l| l.contains("a1b2c3d4"))
            .expect("table row");
        assert_eq!(
            row.matches('`').count(),
            4,
            "only the sha and email spans may use backticks: {row}"
        );
        assert!(!row.contains('\n'));
    }

    #[test]
    fn test_comment_keeps_the_table_to_three_columns() {
        let comment = build_comment(
            &[make_external_result(
                "a1b2c3d4e5f6",
                "x|y|z@b.test\n## fake",
            )],
            &summary_for(0, 1),
        )
        .unwrap();

        let row = comment
            .lines()
            .find(|l| l.contains("a1b2c3d4"))
            .expect("table row");
        assert_eq!(row.matches('|').count(), 4, "three columns: {row}");
        assert!(!comment.contains("\n## fake"));
    }

    #[test]
    fn test_escape_markdown_text_neutralises_links_mentions_and_newlines() {
        let escaped = escape_markdown_text("[click](https://evil.example) @maintainer\n## hi <b>");

        assert!(!escaped.contains("]("), "no live link: {escaped}");
        assert!(!escaped.contains('\n'));
        assert!(
            escaped.contains("@\u{200B}maintainer"),
            "mention broken: {escaped}"
        );
        assert!(!escaped.contains("<b>"));
    }

    #[test]
    fn test_untrusted_text_is_truncated() {
        let long = "a".repeat(MAX_UNTRUSTED_LEN + 50);

        assert_eq!(
            sanitize_code_span(&long).chars().count(),
            MAX_UNTRUSTED_LEN + 1
        );
        assert_eq!(
            escape_markdown_text(&long).chars().count(),
            MAX_UNTRUSTED_LEN + 1
        );
    }

    #[test]
    fn test_strip_control_removes_terminal_escapes() {
        assert_eq!(strip_control("ok\x1b[31mred\x07"), "ok?[31mred?");
    }

    /// The token must not appear when the CLI arguments are debug-logged.
    #[test]
    fn test_debug_output_redacts_the_github_token() {
        use clap::Parser;

        let secret = "ghp_SECRETVALUE0123456789";
        let verify = VerifySignatures::parse_from(["verify-signatures", "--github-token", secret]);
        let shown = format!("{verify:?}");
        assert!(!shown.contains(secret), "token leaked: {shown}");
        assert!(shown.contains("<redacted>"), "got: {shown}");

        let comment =
            crate::cli::comment_pr::CommentPr::parse_from(["comment-pr", "--github-token", secret]);
        assert!(!format!("{comment:?}").contains(secret));

        let issue = crate::cli::create_issue::CreateIssue::parse_from([
            "create-issue",
            "--title",
            "t",
            "--github-token",
            secret,
        ]);
        assert!(!format!("{issue:?}").contains(secret));
    }

    // --- allow list and hard stop (#1132) -----------------------------------

    fn result_with(
        sha: &str,
        email: &str,
        reason: VerificationReason,
        passed: bool,
        key: Option<&str>,
    ) -> VerificationResult {
        VerificationResult {
            commit: CommitInfo {
                sha: sha.to_string(),
                author_email: email.to_string(),
                author_name: "Someone".to_string(),
                subject: "a commit".to_string(),
                signature_status: SignatureStatus::Good,
                key_id: key.map(str::to_string),
                signer: None,
            },
            passed,
            reason,
        }
    }

    fn summary_with(allow_listed: usize, external: usize, failures: usize) -> VerificationSummary {
        VerificationSummary {
            commits_checked: allow_listed + external + failures,
            trusted_verified: 0,
            allow_listed,
            external_contributors: external,
            failures,
        }
    }

    #[test]
    fn test_comment_lists_allow_listed_identities_with_their_key() {
        let results = vec![result_with(
            "aabbccdd1122",
            "29139614+renovate[bot]@users.noreply.github.com",
            VerificationReason::AllowListed,
            true,
            Some("B5690EEEBB952194"),
        )];

        let comment = build_comment(&results, &summary_with(1, 0, 0)).unwrap();

        assert!(comment.contains("### Allow-listed Identities"), "{comment}");
        assert!(comment.contains("- **Allow-listed**: 1"), "{comment}");
        assert!(comment.contains("`B5690EEEBB952194`"), "{comment}");
        assert!(comment.contains("`aabbccdd`"), "{comment}");
        assert!(!comment.contains("### External Contributors"), "{comment}");
        assert!(
            comment.contains("All commits have been verified"),
            "{comment}"
        );
    }

    #[test]
    fn test_comment_omits_the_allow_listed_section_when_none_matched() {
        let results = vec![make_external_result("a1b2c3d4e5f6", "x@example.test")];

        let comment = build_comment(&results, &summary_with(0, 1, 0)).unwrap();

        assert!(!comment.contains("Allow-listed"), "{comment}");
    }

    /// With the hard stop on, a commit from an unlisted external author fails,
    /// is listed under the failed commits and shown as not allowed.
    #[test]
    fn test_comment_reports_the_hard_stop_as_a_failure() {
        let results = vec![result_with(
            "deadbeef0000",
            "stranger@example.test",
            VerificationReason::ExternalNotAllowed {
                invalid_signature: false,
            },
            false,
            None,
        )];

        let comment = build_comment(&results, &summary_with(0, 1, 1)).unwrap();

        assert!(comment.contains("FAILED"), "{comment}");
        assert!(comment.contains("### Failed Commits"), "{comment}");
        assert!(
            comment.contains("External contributor not on the allow list"),
            "{comment}"
        );
        assert!(comment.contains("### External Contributors"), "{comment}");
        assert!(comment.contains("❌ Not on the allow list"), "{comment}");
        let table_rows = comment
            .lines()
            .filter(|l| l.starts_with("| `deadbeef`"))
            .count();
        assert_eq!(table_rows, 1, "the table must not be empty: {comment}");
    }

    #[test]
    fn test_comment_reports_an_allow_listed_identity_with_a_bad_signature_as_failed() {
        let results = vec![result_with(
            "cafe00001111",
            "29139614+renovate[bot]@users.noreply.github.com",
            VerificationReason::AllowListedBadSignature,
            false,
            Some("1111111111111111"),
        )];

        let comment = build_comment(&results, &summary_with(0, 0, 1)).unwrap();

        assert!(comment.contains("FAILED"), "{comment}");
        assert!(
            comment.contains(
                "Allow-listed identity: signature missing, invalid or from an unapproved key"
            ),
            "{comment}"
        );
    }

    #[test]
    fn test_fail_on_external_flag_is_parsed_and_off_by_default() {
        use clap::Parser;

        let off = VerifySignatures::parse_from(["verify-signatures"]);
        let on = VerifySignatures::parse_from(["verify-signatures", "--fail-on-external"]);

        assert!(!off.fail_on_external);
        assert!(on.fail_on_external);
        assert!(format!("{on:?}").contains("fail_on_external: true"));
    }

    // --- policy resolution, outcome and PR number (#1132) -------------------

    use crate::ops::verify_config::test_support::{commit_policy, temp_repo};

    const RENOVATE_POLICY: &str = r#"
        [verify_signatures]
        fail_on_external = false

        [[verify_signatures.allow]]
        email = "29139614+renovate[bot]@users.noreply.github.com"
        keys = ["B5690EEEBB952194"]
        pr_authors = ["renovate[bot]"]
    "#;

    /// `--base` only chooses where the range starts. A commit from before the
    /// policy existed must not become the policy source.
    #[test]
    fn test_policy_comes_from_the_policy_ref_not_from_an_older_base() {
        let (_dir, repo) = temp_repo();
        let old = commit_policy(&repo, "refs/heads/old", None, None);
        commit_policy(&repo, "refs/heads/main", Some(RENOVATE_POLICY), Some(old));

        let from_policy_ref = resolve_policy(&repo, "main", false).unwrap();
        let from_old_commit = resolve_policy(&repo, "old", false).unwrap();

        assert_eq!(from_policy_ref.allow.len(), 1);
        assert!(
            from_old_commit.allow.is_empty(),
            "reading it from the old commit is what --policy-ref avoids"
        );
    }

    #[test]
    fn test_policy_ref_and_base_are_independent_options() {
        use clap::Parser;

        let defaults = VerifySignatures::parse_from(["verify-signatures"]);
        let shifted_base = VerifySignatures::parse_from(["verify-signatures", "--base", "HEAD~3"]);
        let explicit =
            VerifySignatures::parse_from(["verify-signatures", "--policy-ref", "origin/trunk"]);

        assert_eq!(defaults.policy_ref, "origin/main");
        assert_eq!(shifted_base.base, "HEAD~3");
        assert_eq!(
            shifted_base.policy_ref, "origin/main",
            "unaffected by --base"
        );
        assert_eq!(explicit.policy_ref, "origin/trunk");
        assert!(format!("{explicit:?}").contains("origin/trunk"));
    }

    #[test]
    fn test_the_flag_can_switch_the_hard_stop_on_but_never_off() {
        let (_dir, repo) = temp_repo();
        commit_policy(&repo, "refs/heads/main", Some(RENOVATE_POLICY), None);
        commit_policy(
            &repo,
            "refs/heads/strict",
            Some("[verify_signatures]\nfail_on_external = true\n"),
            None,
        );

        assert!(
            !resolve_policy(&repo, "main", false)
                .unwrap()
                .fail_on_external
        );
        assert!(
            resolve_policy(&repo, "main", true)
                .unwrap()
                .fail_on_external
        );
        assert!(
            resolve_policy(&repo, "strict", false)
                .unwrap()
                .fail_on_external
        );
        assert!(
            resolve_policy(&repo, "strict", true)
                .unwrap()
                .fail_on_external
        );
    }

    #[test]
    fn test_an_unresolvable_or_invalid_policy_is_an_error_not_a_default() {
        let (_dir, repo) = temp_repo();
        commit_policy(
            &repo,
            "refs/heads/main",
            Some("[verify_signatures]\nfail_on_extrenal = true\n"),
            None,
        );

        assert!(resolve_policy(&repo, "main", false).is_err());
        assert!(resolve_policy(&repo, "no-such-ref", false).is_err());
    }

    /// Any failure fails the run. The deprecated `--fail-on-unsigned` is not an
    /// input to `finish`, so nothing can turn a hard-stop failure into a pass.
    #[test]
    fn test_any_failure_fails_the_run() {
        assert!(matches!(
            finish(&summary_with(0, 1, 1)),
            Err(Error::SignatureVerificationFailed(1))
        ));
        assert!(matches!(
            finish(&summary_with(1, 0, 0)),
            Ok(CIExit::VerificationPassed)
        ));
    }

    #[test]
    fn test_pr_number_is_the_last_segment_of_the_url() {
        assert_eq!(
            parse_pr_number("https://github.com/jerus-org/pcu/pull/1133"),
            Some(1133)
        );
        assert_eq!(
            parse_pr_number("https://github.com/jerus-org/pcu/pull/1133/\n"),
            Some(1133)
        );
        assert_eq!(parse_pr_number("42"), Some(42));
        assert_eq!(parse_pr_number(""), None);
        assert_eq!(
            parse_pr_number("https://github.com/jerus-org/pcu/pulls"),
            None
        );
        assert_eq!(parse_pr_number("not a url"), None);
    }

    #[test]
    fn test_comment_explains_a_wrong_opener_failure() {
        let results = vec![result_with(
            "feed00001111",
            "29139614+renovate[bot]@users.noreply.github.com",
            VerificationReason::AllowListedWrongOpener,
            false,
            Some("B5690EEEBB952194"),
        )];

        let comment = build_comment(&results, &summary_with(0, 0, 1)).unwrap();

        assert!(comment.contains("FAILED"), "{comment}");
        assert!(
            comment.contains("pull request not opened by one of its accounts"),
            "{comment}"
        );
    }

    fn settings(pairs: &[(&str, &str)]) -> config::Config {
        let mut builder = config::Config::builder();
        for (key, value) in pairs {
            builder = builder.set_override(*key, *value).unwrap();
        }
        builder.build().unwrap()
    }

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn a_configured_app_is_used_for_reads() {
        let app = settings(&[("app_id", "123456")]);
        assert_eq!(
            choose_read_auth(&app, None).unwrap(),
            ReadAuth::App("123456".to_string())
        );
    }

    #[test]
    fn the_app_wins_even_when_a_token_is_also_supplied() {
        let app = settings(&[("app_id", "123456")]);
        assert_eq!(
            choose_read_auth(&app, Some("a-write-token")).unwrap(),
            ReadAuth::App("123456".to_string())
        );
    }

    #[test]
    fn a_blank_app_id_counts_as_unset_and_the_token_is_used() {
        for blank in ["", "   "] {
            let app = settings(&[("app_id", blank)]);
            assert_eq!(
                choose_read_auth(&app, Some("a-token")).unwrap(),
                ReadAuth::Token("a-token".to_string()),
                "{blank:?}"
            );
        }
    }

    #[test]
    fn without_an_app_the_token_is_used() {
        let none = settings(&[]);
        assert_eq!(
            choose_read_auth(&none, Some(" a-token ")).unwrap(),
            ReadAuth::Token("a-token".to_string())
        );
    }

    #[test]
    fn no_app_and_no_usable_token_is_an_error_naming_both_options() {
        let none = settings(&[]);
        for token in [None, Some(""), Some("   ")] {
            let message = choose_read_auth(&none, token).unwrap_err().to_string();
            assert!(message.contains("PCU_APP_ID"), "{message}");
            assert!(message.contains("GITHUB_TOKEN"), "{message}");
        }
    }

    #[test]
    fn a_private_key_alone_does_not_select_the_app() {
        let key_only = settings(&[("private_key", "not-used")]);
        assert!(choose_read_auth(&key_only, None).is_err());
    }

    #[test]
    fn the_environment_maps_pcu_variables_to_settings() {
        let cfg = env_settings(Some(env(&[
            ("PCU_APP_ID", "42"),
            ("PCU_PRIVATE_KEY", "key"),
            ("PCU_COLLAB_READ", "tok"),
            ("UNRELATED", "x"),
        ])))
        .unwrap();
        assert_eq!(cfg.get::<String>("app_id").unwrap(), "42");
        assert_eq!(cfg.get::<String>("private_key").unwrap(), "key");
        assert_eq!(cfg.get::<String>("collab_read").unwrap(), "tok");
        assert!(cfg.get::<String>("unrelated").is_err());
    }

    #[test]
    fn an_empty_environment_value_is_unset() {
        let cfg = env_settings(Some(env(&[("PCU_APP_ID", ""), ("PCU_COLLAB_READ", "")]))).unwrap();
        assert!(cfg.get::<String>("app_id").is_err());
        assert_eq!(collaborator_token(&cfg), None);
        assert_eq!(
            choose_read_auth(&cfg, Some("t")).unwrap(),
            ReadAuth::Token("t".to_string())
        );
    }

    #[test]
    fn the_collaborator_token_is_trimmed_and_blank_means_none() {
        let set = settings(&[("collab_read", "  github_pat_x\n")]);
        assert_eq!(collaborator_token(&set), Some("github_pat_x".to_string()));
        for blank in ["", "  \n"] {
            let cfg = settings(&[("collab_read", blank)]);
            assert_eq!(collaborator_token(&cfg), None, "{blank:?}");
        }
        assert_eq!(collaborator_token(&settings(&[])), None);
    }
}
