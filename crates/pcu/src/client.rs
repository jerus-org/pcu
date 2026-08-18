use std::{collections::HashMap, env, ffi::OsString, fmt::Debug};

pub(crate) mod graphql;
mod pull_request;

use config::Config;
use git2::Repository;
use keep_a_changelog::{ChangeKind, ChangelogParseOptions};
use octocrate::{APIConfig, AppAuthorization, GitHubAPI, PersonalAccessToken};
use owo_colors::{OwoColorize, Style};

use self::pull_request::PullRequest;
use crate::{Error, PrTitle};

const END_POINT: &str = "https://api.github.com/graphql";

pub struct Client {
    #[allow(dead_code)]
    // pub(crate) settings: Config,
    pub(crate) git_repo: Repository,
    pub(crate) github_rest: GitHubAPI,
    pub(crate) github_graphql: gql_client::Client,
    pub(crate) github_token: String,
    pub(crate) owner: String,
    pub(crate) repo: String,
    /// Owns the release-lookup/asset-download read path (jerus-org/pcu#1051)
    /// — kept as its own crate so a consumer that only needs this capability
    /// (e.g. jci-audit) can depend on it without pulling in git2/clap/config.
    /// Deliberately builds its own `github_rest`/`github_graphql` from the
    /// same token rather than sharing `Client`'s: reusing them would require
    /// passing pcu's auth objects into the standalone crate, recoupling the
    /// two and defeating the isolation this split exists for. The extra
    /// client construction is one-time, per-`Client`, not per-call.
    release_assets: pcu_release_assets::ReleaseAssetClient,
    pub(crate) default_branch: String,
    pub(crate) branch: Option<String>,
    pull_request: Option<PullRequest>,
    pub(crate) prlog: OsString,
    pub(crate) line_limit: usize,
    pub(crate) prlog_parse_options: ChangelogParseOptions,
    pub(crate) prlog_update: Option<PrTitle>,
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
            .field("prlog", &self.prlog)
            .field("line_limit", &self.line_limit)
            .field("prlog_parse_options", &self.prlog_parse_options)
            .field("prlog_update", &self.prlog_update)
            .field("commit_message", &self.commit_message)
            .finish()
    }
}

impl Client {
    pub async fn new_with(settings: &Config) -> Result<Self, Error> {
        let cmd = settings
            .get::<String>("command")
            .map_err(|_| Error::CommandNotSet)?;
        log::trace!("cmd: {cmd:?}");

        // Use the username config settings to direct to the appropriate CI environment
        // variable to find the owner
        log::trace!("owner: {:?}", settings.get::<String>("username"));
        let pcu_owner: String = settings
            .get("username")
            .map_err(|_| Error::EnvVarBranchNotSet)?;
        let owner = env::var(pcu_owner).map_err(|_| Error::EnvVarBranchNotFound)?;

        // Use the reponame config settings to direct to the appropriate CI environment
        // variable to find the repo
        log::trace!("repo: {:?}", settings.get::<String>("reponame"));
        let pcu_repo: String = settings
            .get("reponame")
            .map_err(|_| Error::EnvVarBranchNotSet)?;
        let repo = env::var(pcu_repo).map_err(|_| Error::EnvVarBranchNotFound)?;

        let default_branch = settings
            .get::<String>("default_branch")
            .unwrap_or("main".to_string());

        let commit_message = settings
            .get::<String>("commit_message")
            .unwrap_or("".to_string());

        let line_limit = settings.get::<usize>("line_limit").unwrap_or(10);

        log::trace!("Getting the github api with {settings:#?}, {owner}, {repo}");
        let (github_rest, github_graphql, github_token) =
            Client::get_github_apis(settings, &owner, &repo).await?;

        let git_repo = git2::Repository::open(".")?;

        log::trace!("Executing for command: {cmd}");
        let (branch, pull_request) = if &cmd == "pr" || &cmd == "push" {
            // Use the branch config settings to direct to the appropriate CI environment
            // variable to find the branch data
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

            // Check if from_merge flag is set
            let from_merge = settings.get::<bool>("from_merge").unwrap_or(false);

            let pull_request = if from_merge {
                log::info!("Using from_merge mode - looking up PR from HEAD commit");
                Some(
                    PullRequest::from_head_commit(&git_repo, &github_graphql, &owner, &repo)
                        .await?,
                )
            } else {
                PullRequest::new_pull_request_opt(settings, &github_graphql).await?
            };

            (branch, pull_request)
        } else {
            let branch = None;
            let pull_request = None;
            (branch, pull_request)
        };
        log::trace!("branch: {branch:?} and pull_request: {pull_request:?}");

        log::trace!("log: {:?}", settings.get::<String>("prlog"));
        let prlog: String = settings
            .get("prlog")
            .map_err(|_| Error::DefaultChangeLogNotSet)?;
        let prlog = OsString::from(prlog);

        let svs_root = settings
            .get("dev_platform")
            .unwrap_or_else(|_| "https://github.com/".to_string());
        let prefix = settings
            .get("version_prefix")
            .unwrap_or_else(|_| "v".to_string());
        let repo_url = Some(format!("{svs_root}{owner}/{repo}"));
        let prlog_parse_options = ChangelogParseOptions {
            url: repo_url,
            head: Some("HEAD".to_string()),
            tag_prefix: Some(prefix),
        };

        let release_assets =
            pcu_release_assets::ReleaseAssetClient::new(owner.clone(), repo.clone(), &github_token);

        Ok(Self {
            git_repo,
            github_rest,
            github_graphql,
            github_token,
            default_branch,
            branch,
            owner,
            repo,
            release_assets,
            pull_request,
            prlog,
            line_limit,
            prlog_parse_options,
            prlog_update: None,
            commit_message,
        })
    }

    /// Get the GitHub API instance
    async fn get_github_apis(
        settings: &Config,
        owner: &str,
        repo: &str,
    ) -> Result<(GitHubAPI, gql_client::Client, String), Error> {
        let bld_style = Style::new().bold();
        log::info!("\n***Get GitHub API instance***\n");
        log::trace!("Settings: {settings:#?}");
        let (config, token) = match settings.get::<String>("app_id") {
            Ok(app_id) => {
                log::info!("Using {} for authentication", "GitHub App".style(bld_style));

                let private_key = settings
                    .get::<String>("private_key")
                    .map_err(|_| Error::NoGitHubAPIPrivateKey)?;

                log::trace!("Using private key {private_key:#?} for authentication");

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
                log::warn!(
                    "Falling back to {} for authentication — PAT lacks branch protection bypass authority",
                    "Personal Access Token".style(bld_style)
                );

                // Create a personal access token
                let personal_access_token = PersonalAccessToken::new(&pat);

                // Use the personal access token to create a API configuration
                (APIConfig::with_token(personal_access_token).shared(), pat)
            }
        };

        let auth = format!("Bearer {token}");

        let headers = HashMap::from([
            ("X-Github-Next-Global-ID", "1"),
            ("User-Agent", owner),
            ("Authorization", &auth),
        ]);

        let github_graphql = gql_client::Client::new_with_headers(END_POINT, headers);

        let github_rest = GitHubAPI::new(&config);

        Ok((github_rest, github_graphql, token))
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

    pub fn body(&self) -> &str {
        if let Some(pr) = self.pull_request.as_ref() {
            &pr.body
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
        if let Some(pr) = self.pull_request.as_mut() {
            pr.title = title.to_string();
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
        if let Some(update) = &self.prlog_update {
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
        if let Some(update) = &self.prlog_update {
            Some(&update.entry)
        } else {
            None
        }
    }

    pub fn prlog_as_str(&self) -> &str {
        if let Some(cl) = &self.prlog.to_str() {
            cl
        } else {
            ""
        }
    }

    pub fn set_prlog(&mut self, value: &str) {
        self.prlog = value.into();
    }

    /// Construct a `Client` that reads only the local git repository.
    ///
    /// Unlike [`Client::new_with`], this constructor does not make any GitHub API calls
    /// and does not require CI environment variables.  It derives `owner` and
    /// `repo` from the `origin` remote URL of the repository at the current
    /// working directory.
    ///
    /// Methods that require GitHub authentication (e.g. `push_commit` with App
    /// bypass) will fail unless credentials are available through the usual
    /// environment variables — but operations that are purely local (e.g.
    /// `commit_staged`, `stage_paths`) work without any network access.
    pub fn new_local() -> Result<Self, Error> {
        Self::new_local_at(std::path::Path::new("."))
    }

    /// Like [`Client::new_local`] but opens the repository at an explicit `path`.
    ///
    /// Intended for tests that operate on a temporary git repository.
    pub fn new_local_at(path: &std::path::Path) -> Result<Self, Error> {
        let git_repo = git2::Repository::open(path)?;

        let (owner, repo) = extract_owner_repo_from_git(&git_repo)
            .unwrap_or_else(|| ("local".to_string(), "local".to_string()));

        let branch = git_repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().ok().map(str::to_string));

        // Create minimal API stubs — any method that actually uses these will
        // fail with an auth error, which is expected for a local-only client.
        let dummy_pat = PersonalAccessToken::new("");
        let dummy_config = APIConfig::with_token(dummy_pat).shared();
        let github_rest = GitHubAPI::new(&dummy_config);
        let github_graphql = gql_client::Client::new_with_headers(
            END_POINT,
            HashMap::from([
                ("X-Github-Next-Global-ID", "1"),
                ("User-Agent", "pcu-local"),
                ("Authorization", "Bearer local"),
            ]),
        );

        let prlog = OsString::from("PRLOG.md");
        let prlog_parse_options = ChangelogParseOptions {
            url: None,
            head: Some("HEAD".to_string()),
            tag_prefix: Some("v".to_string()),
        };

        let release_assets =
            pcu_release_assets::ReleaseAssetClient::new(owner.clone(), repo.clone(), "");

        Ok(Self {
            git_repo,
            github_rest,
            github_graphql,
            github_token: String::new(),
            owner,
            repo,
            release_assets,
            default_branch: "main".to_string(),
            branch,
            pull_request: None,
            prlog,
            line_limit: 10,
            prlog_parse_options,
            prlog_update: None,
            commit_message: String::new(),
        })
    }

    /// Locate the release for `tag`, including drafts.
    ///
    /// Supersedes the former `github_release_exists`, which answered only
    /// "yes/no" and could see published releases only. Both callers need more
    /// than that: creation must distinguish a reusable draft from a finished
    /// release, and the asset paths need the id. A release counts only when its
    /// `tag_name` matches exactly — a bare `get_release_by_tag(...).is_ok()`
    /// could coerce a 404 or a phantom response into "exists" and silently skip
    /// release creation, the failure that stranded gen-circleci-orb 0.0.53.
    ///
    /// `Ok(None)` means no release exists for the tag — a legitimate answer when
    /// deciding whether to create one, and an error only at the call sites that
    /// require a release to already be there.
    ///
    /// Delegates to [`pcu_release_assets::ReleaseAssetClient`] — see
    /// jerus-org/pcu#1051 for why the release lookup and asset fetch live in
    /// their own crate.
    pub(crate) async fn find_release_for_tag(
        &self,
        tag: &str,
    ) -> Result<Option<pcu_release_assets::ReleaseRef>, Error> {
        Ok(self.release_assets.find_release_for_tag(tag).await?)
    }

    /// Upload a binary asset to an existing GitHub release.
    ///
    /// Looks up the release by `tag`, then uploads `binary` as `asset_name` to
    /// `uploads.github.com`.  Retries the release lookup up to 5 times with a
    /// 2-second delay to handle GitHub's eventual-consistency window after
    /// release creation.
    /// Upload a binary as a GitHub release asset.
    ///
    /// Idempotent: if an asset with the same name already exists on the
    /// release it is deleted first (delete-then-replace), so retries don't
    /// fail with GitHub's 422 "Validation Failed".
    pub async fn upload_release_asset(
        &self,
        tag: &str,
        binary: &std::path::Path,
        asset_name: &str,
    ) -> Result<(), Error> {
        use octocrate::PersonalAccessToken;

        if !binary.exists() {
            return Err(Error::GitError(format!(
                "Asset file not found: {}",
                binary.display()
            )));
        }

        let release_ref = self.require_release_for_tag(tag).await?;

        // Assets are frozen at publication, so neither the upload nor the
        // delete-then-replace below can succeed. Refusing here names the cause
        // before anything is attempted, rather than translating the API's
        // rejection after the fact.
        if release_ref.immutable {
            return Err(Error::ImmutableRelease(
                tag.to_string(),
                "the release is published with immutable assets".to_string(),
            ));
        }

        // Delete-then-replace: if an asset of the same name already exists on
        // the release (e.g. from a previous, partially-completed run), GitHub
        // rejects a fresh upload with HTTP 422. Remove it first so the upload
        // is idempotent across retries. On a published immutable release the
        // deletion is refused too, which is why the error is translated rather
        // than surfaced raw.
        if let Some(asset_id) = self
            .find_asset_in_release(release_ref.id, asset_name)
            .await?
        {
            log::info!("Replacing existing asset '{asset_name}' (id={asset_id})");
            let del_token = PersonalAccessToken::new(self.github_token.clone());
            let del_config = APIConfig::with_token(del_token).shared();
            let del_api = GitHubAPI::new(&del_config);
            del_api
                .repos
                .delete_release_asset(&self.owner, &self.repo, asset_id)
                .send()
                .await
                .map_err(|e| map_asset_upload_error(tag, &e.to_string()))?;
        }

        // Binary uploads must go to uploads.github.com, not api.github.com.
        let upload_token = PersonalAccessToken::new(self.github_token.clone());
        let upload_config = APIConfig::new("https://uploads.github.com", upload_token);
        let upload_api = GitHubAPI::new(&upload_config);

        let file = tokio::fs::File::open(binary).await?;
        let content_length = file.metadata().await?.len();

        let content_type = if asset_name.ends_with(".sig") {
            "text/plain"
        } else {
            "application/octet-stream"
        };

        let query = octocrate::repos::upload_release_asset::Query::builder()
            .name(asset_name)
            .build();

        upload_api
            .repos
            .upload_release_asset(&self.owner, &self.repo, release_ref.id)
            .query(&query)
            .header("Content-Type", content_type)
            .header("Content-Length", content_length.to_string())
            .file(file)
            .send()
            .await
            .map_err(|e| map_asset_upload_error(tag, &e.to_string()))?;

        log::info!("Successfully uploaded {asset_name}");
        Ok(())
    }

    /// Download a named asset from an existing GitHub release, returning its
    /// raw bytes.
    ///
    /// Refuses a draft release unless `allow_draft` is `true` — a draft's
    /// assets can still be replaced, so callers that need a finalised
    /// artefact (e.g. an audit record) should read from a published release
    /// by default and opt in explicitly if a draft is acceptable.
    ///
    /// Delegates to [`pcu_release_assets::ReleaseAssetClient`] (jerus-org/pcu#1051).
    /// For a read-only, checkout-free, no-draft-access client see
    /// [`pcu_release_assets::ReleaseAssetClient::download_release_asset`] directly.
    pub async fn download_release_asset(
        &self,
        tag: &str,
        asset_name: &str,
        allow_draft: bool,
    ) -> Result<Vec<u8>, Error> {
        Ok(self
            .release_assets
            .download_release_asset_allowing_draft(tag, asset_name, allow_draft)
            .await?)
    }

    /// Look up the release for `tag`, erroring if none exists.
    ///
    /// Used by `upload_release_asset`, which needs the release to already
    /// exist before it can attach to it. `download_release_asset` has its
    /// own copy of this lookup inside `pcu_release_assets::ReleaseAssetClient`
    /// (jerus-org/pcu#1051) — it no longer calls this method.
    async fn require_release_for_tag(
        &self,
        tag: &str,
    ) -> Result<pcu_release_assets::ReleaseRef, Error> {
        self.find_release_for_tag(tag)
            .await?
            .ok_or_else(|| release_not_found_error(tag))
    }

    /// Fetch `release_id`'s current asset list and look up `asset_name` in
    /// it, returning its id if present.
    ///
    /// Used by `upload_release_asset` to decide whether to
    /// delete-then-replace. `download_release_asset` delegates straight to
    /// `pcu_release_assets::ReleaseAssetClient`, which has its own copy of
    /// this lookup — it no longer calls this method.
    async fn find_asset_in_release(
        &self,
        release_id: i64,
        asset_name: &str,
    ) -> Result<Option<i64>, Error> {
        Ok(self
            .release_assets
            .find_asset_in_release(release_id, asset_name)
            .await?)
    }

    /// Publish the release for `tag` when it is still a draft.
    ///
    /// Unconditional by design — the release pipeline runs this as its last
    /// step with no condition attached — so it must be a
    /// no-op wherever there is nothing to publish. `make_latest` is set here
    /// rather than at creation because GitHub does not accept it on a draft.
    pub(crate) async fn publish_release(&self, release_id: i64) -> Result<(), Error> {
        let request = octocrate::repos::update_release::Request {
            body: None,
            discussion_category_name: None,
            draft: Some(false),
            make_latest: Some(octocrate::repos::update_release::RequestMakeLatest::True),
            name: None,
            prerelease: None,
            tag_name: None,
            target_commitish: None,
        };

        self.github_rest
            .repos
            .update_release(&self.owner, &self.repo, release_id)
            .body(&request)
            .send()
            .await?;

        Ok(())
    }
}

/// Parse `owner` and `repo` from the `origin` remote URL of `repo`.
///
/// Handles both SCP-style (`git@github.com:org/repo.git`) and HTTPS
/// (`https://github.com/org/repo.git`) URLs.  Returns `None` if the remote
/// is absent or the URL cannot be parsed.
fn extract_owner_repo_from_git(repo: &Repository) -> Option<(String, String)> {
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url().ok()?;
    parse_owner_repo_from_url(url)
}

/// Extract `(owner, repo)` from a GitHub remote URL.
fn parse_owner_repo_from_url(url: &str) -> Option<(String, String)> {
    // Normalise SSH → HTTPS so one parser handles all formats
    let https = if let Some(rest) = url.strip_prefix("git@github.com:") {
        format!("https://github.com/{rest}")
    } else if let Some(rest) = url
        .strip_prefix("ssh://git@github.com/")
        .or_else(|| url.strip_prefix("ssh://github.com/"))
    {
        format!("https://github.com/{rest}")
    } else {
        url.to_string()
    };

    let path = https.strip_prefix("https://github.com/")?;
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut parts = path.splitn(2, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    Some((owner, repo))
}

/// Build the error for "no release exists for this tag".
/// Pure so the message is unit-testable without a network round trip.
fn release_not_found_error(tag: &str) -> Error {
    Error::GitError(format!("GitHub release for tag '{tag}' not found"))
}

/// Translate an asset-upload failure into an actionable error.
///
/// The immutable-release rejection is otherwise opaque: the raw API message
/// says what was refused but not why the pipeline is wrong or what to do about
/// it. Every other failure passes through unchanged.
pub(crate) fn map_asset_upload_error(tag: &str, api_message: &str) -> Error {
    if api_message.to_lowercase().contains("immutable release") {
        return Error::ImmutableRelease(tag.to_string(), api_message.to_string());
    }
    Error::GitError(api_message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_https_url() {
        let (owner, repo) =
            parse_owner_repo_from_url("https://github.com/jerus-org/pcu.git").unwrap();
        assert_eq!(owner, "jerus-org");
        assert_eq!(repo, "pcu");
    }

    #[test]
    fn parse_scp_url() {
        let (owner, repo) = parse_owner_repo_from_url("git@github.com:jerus-org/pcu.git").unwrap();
        assert_eq!(owner, "jerus-org");
        assert_eq!(repo, "pcu");
    }

    #[test]
    fn parse_https_url_no_dot_git() {
        let (owner, repo) = parse_owner_repo_from_url("https://github.com/jerus-org/pcu").unwrap();
        assert_eq!(owner, "jerus-org");
        assert_eq!(repo, "pcu");
    }

    #[test]
    fn new_local_at_derives_owner_repo_from_remote() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        repo.remote("origin", "https://github.com/test-org/test-repo.git")
            .unwrap();

        let client = Client::new_local_at(dir.path()).unwrap();
        assert_eq!(client.owner(), "test-org");
        assert_eq!(client.repo(), "test-repo");
    }

    #[test]
    fn new_local_at_falls_back_when_no_remote() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();

        // No remote configured — should fall back to "local"/"local"
        let client = Client::new_local_at(dir.path()).unwrap();
        assert_eq!(client.owner(), "local");
        assert_eq!(client.repo(), "local");
    }

    #[test]
    fn upload_release_asset_returns_error_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let client = Client::new_local_at(dir.path()).unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.upload_release_asset(
            "v1.0.0",
            std::path::Path::new("/nonexistent/binary"),
            "binary",
        ));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Asset file not found"), "unexpected: {msg}");
    }

    #[test]
    fn release_not_found_error_names_the_tag() {
        let msg = release_not_found_error("pcu-v1.0.0").to_string();
        assert!(msg.contains("pcu-v1.0.0"), "unexpected: {msg}");
    }

    #[test]
    fn map_asset_upload_error_translates_immutable_release() {
        let err = map_asset_upload_error(
            "pcu-v0.6.29",
            "Cannot upload assets to an immutable release.",
        );
        let msg = err.to_string();
        assert!(msg.contains("pcu-v0.6.29"), "tag missing: {msg}");
        assert!(msg.contains("immutable"), "cause missing: {msg}");
        assert!(
            msg.contains("draft"),
            "should name the draft-first remedy: {msg}"
        );
        assert!(
            msg.contains("next patch version"),
            "should say the release cannot be repaired in place: {msg}"
        );
    }

    #[test]
    fn map_asset_upload_error_is_case_insensitive() {
        let err = map_asset_upload_error(
            "pcu-v0.6.29",
            "cannot upload assets to an IMMUTABLE release",
        );
        assert!(matches!(err, Error::ImmutableRelease(_, _)));
    }

    #[test]
    fn map_asset_upload_error_passes_other_failures_through() {
        let err = map_asset_upload_error("pcu-v0.6.29", "Validation Failed");
        assert!(!matches!(err, Error::ImmutableRelease(_, _)));
        assert!(err.to_string().contains("Validation Failed"));
    }
}
