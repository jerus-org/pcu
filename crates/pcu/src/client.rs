use std::{collections::HashMap, env, ffi::OsString, fmt::Debug};

pub(crate) mod graphql;
mod pull_request;

use config::Config;
use git2::Repository;
use keep_a_changelog::{ChangeKind, ChangelogParseOptions};
use octocrate::{APIConfig, AppAuthorization, GitHubAPI, PersonalAccessToken};
use owo_colors::{OwoColorize, Style};

use self::graphql::GraphQLGetReleases;
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

        Ok(Self {
            git_repo,
            github_rest,
            github_graphql,
            github_token,
            default_branch,
            branch,
            owner,
            repo,
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

        Ok(Self {
            git_repo,
            github_rest,
            github_graphql,
            github_token: String::new(),
            owner,
            repo,
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
    /// One GraphQL round trip gathers every candidate; see
    /// [`GraphQLGetReleases`] for why both a by-tag lookup and a listing are
    /// required, and why the listing is affordable in GraphQL but not in REST.
    ///
    /// `Ok(None)` means no release exists for the tag — a legitimate answer when
    /// deciding whether to create one, and an error only at the call sites that
    /// require a release to already be there.
    pub(crate) async fn find_release_for_tag(
        &self,
        tag: &str,
    ) -> Result<Option<ReleaseRef>, Error> {
        lookup_with_retry(
            tag,
            RELEASE_LOOKUP_ATTEMPTS,
            RELEASE_LOOKUP_DELAY,
            || async { self.probe_release_for_tag(tag).await },
        )
        .await
    }

    /// One pass of the lookup: fetch the candidates and pick between them.
    ///
    /// Retried by [`Self::find_release_for_tag`] because GitHub's API lags
    /// briefly behind release creation. The retry wraps the whole query rather
    /// than any half of it, so a draft costs no more attempts than a published
    /// release — the draft path is the normal one under draft-first, and must
    /// not spend the retry window, or fill the log with warnings, to reach it.
    async fn probe_release_for_tag(&self, tag: &str) -> Result<Option<ReleaseRef>, Error> {
        let candidates = self.get_release_candidates(tag).await?;

        let Some(id) = find_release_id_by_tag(
            candidates.iter().filter_map(|r| {
                r.database_id
                    .map(|id| (r.tag_name.as_str(), id, r.is_draft))
            }),
            tag,
        ) else {
            return Ok(None);
        };

        let chosen = candidates
            .iter()
            .find(|r| r.database_id == Some(id))
            .ok_or_else(|| {
                Error::GitError(format!(
                    "release {id} for '{tag}' vanished from the candidates"
                ))
            })?;

        log::info!(
            "Found release {id} for tag '{tag}' (draft={}, immutable={})",
            chosen.is_draft,
            chosen.immutable
        );

        Ok(Some(ReleaseRef {
            id,
            draft: chosen.is_draft,
            immutable: chosen.immutable,
        }))
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
    /// Looks up the release by `tag` (draft-aware, same as
    /// `upload_release_asset`), finds the asset by name in the release's
    /// asset list, then fetches the raw bytes.
    ///
    /// octocrate's typed `send()` always deserializes the response as JSON,
    /// so it has no path for raw binary content — the byte fetch goes
    /// through a bare `reqwest` GET against GitHub's asset API endpoint
    /// instead, with `Accept: application/octet-stream` and the same bearer
    /// token used elsewhere. This works uniformly for public and private
    /// repos, unlike `asset.browser_download_url`, which only works
    /// unauthenticated and only for public repos.
    ///
    /// Immutability does not apply here: it only blocks writes to a
    /// release's assets, and a published/immutable release is the steady
    /// state this reads from.
    pub async fn download_release_asset(
        &self,
        tag: &str,
        asset_name: &str,
    ) -> Result<Vec<u8>, Error> {
        let release_ref = self.require_release_for_tag(tag).await?;

        let asset_id = self
            .find_asset_in_release(release_ref.id, asset_name)
            .await?
            .ok_or_else(|| asset_not_found_error(asset_name, tag))?;

        let url = asset_download_url(&self.owner, &self.repo, asset_id);

        let response = reqwest::Client::new()
            .get(&url)
            .header("Accept", "application/octet-stream")
            .header("Authorization", format!("Bearer {}", self.github_token))
            .header("User-Agent", "pcu")
            .send()
            .await
            .map_err(|e| {
                Error::GitError(format!("failed to download asset '{asset_name}': {e}"))
            })?;

        check_download_response_status(response.status(), tag, asset_name)?;

        let bytes = response.bytes().await.map_err(|e| {
            Error::GitError(format!("failed to read body of asset '{asset_name}': {e}"))
        })?;

        log::info!("Downloaded {} bytes for asset '{asset_name}'", bytes.len());
        Ok(bytes.to_vec())
    }

    /// Look up the release for `tag`, erroring if none exists.
    ///
    /// Shared by `upload_release_asset` and `download_release_asset` — both
    /// need the release to already exist and give up identically when it
    /// doesn't.
    async fn require_release_for_tag(&self, tag: &str) -> Result<ReleaseRef, Error> {
        self.find_release_for_tag(tag)
            .await?
            .ok_or_else(|| release_not_found_error(tag))
    }

    /// Fetch `release_id`'s current asset list and look up `asset_name` in
    /// it, returning its id if present.
    ///
    /// Shared by `upload_release_asset` (to decide whether to
    /// delete-then-replace) and `download_release_asset` (which requires the
    /// asset to already exist).
    async fn find_asset_in_release(
        &self,
        release_id: i64,
        asset_name: &str,
    ) -> Result<Option<i64>, Error> {
        let release = self
            .github_rest
            .repos
            .get_release(&self.owner, &self.repo, release_id)
            .send()
            .await?;

        Ok(find_existing_asset_id(
            release.assets.iter().map(|a| (a.name.as_str(), a.id)),
            asset_name,
        ))
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

/// Find the id of a release asset whose name matches `name`, if present.
/// Pure helper so the match logic is unit-testable without constructing
/// octocrate types.
fn find_existing_asset_id<'a>(
    assets: impl IntoIterator<Item = (&'a str, i64)>,
    name: &str,
) -> Option<i64> {
    assets
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, id)| id)
}

/// Build GitHub's REST asset-download URL for `asset_id`.
///
/// Deliberately not `asset.browser_download_url`: that field only works
/// unauthenticated, and only for public repos. This endpoint accepts the
/// same bearer token used for every other GitHub call and works uniformly
/// for public and private repos.
fn asset_download_url(owner: &str, repo: &str, asset_id: i64) -> String {
    format!("https://api.github.com/repos/{owner}/{repo}/releases/assets/{asset_id}")
}

/// Build the error for "no release exists for this tag".
/// Pure so the message is unit-testable without a network round trip.
fn release_not_found_error(tag: &str) -> Error {
    Error::GitError(format!("GitHub release for tag '{tag}' not found"))
}

/// Build the error for "this release has no asset by this name".
/// Pure so the message is unit-testable without a network round trip.
fn asset_not_found_error(asset_name: &str, tag: &str) -> Error {
    Error::GitError(format!(
        "no asset named '{asset_name}' found on release for tag '{tag}'"
    ))
}

/// Translate a non-success HTTP status from the asset-download endpoint into
/// an `Error`, or `Ok(())` if the status indicates success.
/// Pure so the message is unit-testable without a real HTTP round trip.
fn check_download_response_status(
    status: reqwest::StatusCode,
    tag: &str,
    asset_name: &str,
) -> Result<(), Error> {
    if status.is_success() {
        return Ok(());
    }
    Err(Error::GitError(format!(
        "GitHub returned {status} downloading asset '{asset_name}' for tag '{tag}'"
    )))
}

/// Attempts made before concluding no release exists for a tag.
const RELEASE_LOOKUP_ATTEMPTS: u32 = 5;
/// Delay between release-lookup attempts, covering GitHub's
/// eventual-consistency window after a release is created.
const RELEASE_LOOKUP_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

/// A release located for a tag, reduced to what the callers act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReleaseRef {
    /// REST id — asset upload is REST-only, so this is what it needs.
    pub(crate) id: i64,
    pub(crate) draft: bool,
    /// GitHub has frozen this release's assets. Only ever true for a published
    /// release; a draft is by definition still open.
    pub(crate) immutable: bool,
}

/// Select the release id for `tag` from a listing, preferring a published
/// release over a draft.
///
/// A tag can carry both. GitHub rejects a second *published* release for a tag
/// but happily accepts multiple drafts, so a failed run leaves a draft behind
/// that a later successful run does not replace — jerus-org/gen-circleci-orb
/// carries an abandoned empty draft for `v0.0.41` alongside the published
/// release created 37 minutes later.
///
/// The published release wins because it is the finished article: in a
/// draft-first pipeline its existence means the run already completed, so the
/// right response is to do nothing. Reusing the stale draft instead would upload
/// assets into a release nobody can see, then attempt to publish a second
/// release for a tag that already has one. With no published release, the first
/// draft is reused — the retry case draft-first depends on.
///
/// Matching is exact: `v0.1.6` must not match inside `gen-changelog-v0.1.6`.
fn find_release_id_by_tag<'a>(
    releases: impl IntoIterator<Item = (&'a str, i64, bool)>,
    tag: &str,
) -> Option<i64> {
    let mut draft_id = None;
    for (tag_name, id, draft) in releases {
        if tag_name != tag {
            continue;
        }
        if !draft {
            return Some(id);
        }
        draft_id.get_or_insert(id);
    }
    draft_id
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

/// Retry `probe` until it finds something, distinguishing "not there" from
/// "could not tell".
///
/// `Ok(None)` is a legitimate answer — it is how the creation path decides to
/// create a release — so exhausting the attempts on `Ok(None)` returns
/// `Ok(None)` rather than an error. An `Err`, by contrast, means the lookup was
/// undetermined: it is retried, and if every attempt fails the last error is
/// propagated. Coercing that case into "absent" is what let a release-creation
/// step go green having created nothing, stranding gen-circleci-orb 0.0.53.
async fn lookup_with_retry<F, Fut, T>(
    tag: &str,
    max_attempts: u32,
    retry_delay: std::time::Duration,
    mut probe: F,
) -> Result<Option<T>, Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Option<T>, Error>>,
{
    let mut last_error = None;

    for attempt in 1..=max_attempts {
        match probe().await {
            Ok(Some(found)) => return Ok(Some(found)),
            Ok(None) => {
                log::debug!("no release for '{tag}' yet (attempt {attempt}/{max_attempts})");
            }
            Err(e) => {
                log::warn!(
                    "release lookup for '{tag}' failed (attempt {attempt}/{max_attempts}): {e}"
                );
                last_error = Some(e);
            }
        }
        if attempt < max_attempts {
            tokio::time::sleep(retry_delay).await;
        }
    }

    match last_error {
        Some(e) => Err(e),
        None => Ok(None),
    }
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
    fn find_existing_asset_id_matches_by_name() {
        let assets = [("tool_mcp-linux-x86_64", 11i64), ("tool.tar.gz.sig", 22i64)];
        assert_eq!(
            find_existing_asset_id(assets.iter().copied(), "tool.tar.gz.sig"),
            Some(22)
        );
        assert_eq!(
            find_existing_asset_id(assets.iter().copied(), "tool_mcp-linux-x86_64"),
            Some(11)
        );
        assert_eq!(
            find_existing_asset_id(assets.iter().copied(), "absent"),
            None
        );
    }

    #[test]
    fn asset_download_url_builds_the_rest_assets_endpoint() {
        assert_eq!(
            asset_download_url("jerus-org", "jci-audit", 999),
            "https://api.github.com/repos/jerus-org/jci-audit/releases/assets/999"
        );
    }

    #[test]
    fn release_not_found_error_names_the_tag() {
        let msg = release_not_found_error("pcu-v1.0.0").to_string();
        assert!(msg.contains("pcu-v1.0.0"), "unexpected: {msg}");
    }

    #[test]
    fn asset_not_found_error_names_the_asset_and_tag() {
        let msg = asset_not_found_error("asset.json", "pcu-v1.0.0").to_string();
        assert!(msg.contains("asset.json"), "unexpected: {msg}");
        assert!(msg.contains("pcu-v1.0.0"), "unexpected: {msg}");
    }

    #[test]
    fn check_download_response_status_ok_on_success() {
        assert!(check_download_response_status(
            reqwest::StatusCode::OK,
            "pcu-v1.0.0",
            "asset.json"
        )
        .is_ok());
    }

    #[test]
    fn check_download_response_status_errors_on_failure() {
        let msg = check_download_response_status(
            reqwest::StatusCode::NOT_FOUND,
            "pcu-v1.0.0",
            "asset.json",
        )
        .unwrap_err()
        .to_string();
        assert!(msg.contains("404"), "unexpected: {msg}");
        assert!(msg.contains("asset.json"), "unexpected: {msg}");
        assert!(msg.contains("pcu-v1.0.0"), "unexpected: {msg}");
    }

    #[tokio::test]
    async fn lookup_with_retry_returns_on_first_success() {
        let attempts = std::sync::atomic::AtomicU32::new(0);
        let found = lookup_with_retry("pcu-v1.0.0", 5, std::time::Duration::ZERO, || {
            attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async { Ok(Some(7i64)) }
        })
        .await
        .unwrap();
        assert_eq!(found, Some(7));
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn lookup_with_retry_tolerates_api_lag() {
        // A release is not visible immediately after creation.
        let attempts = std::sync::atomic::AtomicU32::new(0);
        let found = lookup_with_retry("pcu-v1.0.0", 5, std::time::Duration::ZERO, || {
            let n = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if n >= 2 {
                    Ok(Some(7i64))
                } else {
                    Ok(None)
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(found, Some(7));
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn lookup_with_retry_reports_absence_without_error() {
        // "No release exists" is a legitimate answer — it is how the creation
        // path decides to create one — so it must not surface as an error.
        let found = lookup_with_retry("pcu-v1.0.0", 3, std::time::Duration::ZERO, || async {
            Ok(None::<i64>)
        })
        .await
        .unwrap();
        assert_eq!(found, None);
    }

    #[tokio::test]
    async fn lookup_with_retry_propagates_a_persistent_api_error() {
        // An undetermined lookup must NOT be coerced into "absent": that is the
        // silent skip that stranded gen-circleci-orb 0.0.53.
        let result = lookup_with_retry("pcu-v1.0.0", 3, std::time::Duration::ZERO, || async {
            Err::<Option<i64>, Error>(Error::GitError("api 500".into()))
        })
        .await;
        assert!(result.is_err(), "a persistent API error must surface");
    }

    #[tokio::test]
    async fn lookup_with_retry_recovers_from_a_transient_api_error() {
        let attempts = std::sync::atomic::AtomicU32::new(0);
        let found = lookup_with_retry("pcu-v1.0.0", 5, std::time::Duration::ZERO, || {
            let n = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if n == 0 {
                    Err(Error::GitError("api 500".into()))
                } else {
                    Ok(Some(7i64))
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(found, Some(7));
    }

    #[test]
    fn find_release_id_by_tag_matches_published_release() {
        let releases = [("pcu-v0.6.28", 10i64, false), ("pcu-v0.6.29", 11, false)];
        assert_eq!(
            find_release_id_by_tag(releases.iter().copied(), "pcu-v0.6.29"),
            Some(11)
        );
    }

    #[test]
    fn find_release_id_by_tag_matches_draft_release() {
        // The case get_release_by_tag cannot serve: drafts 404 on that endpoint,
        // so the listing scan is the only way to find one.
        let releases = [("pcu-v0.6.29", 11i64, true)];
        assert_eq!(
            find_release_id_by_tag(releases.iter().copied(), "pcu-v0.6.29"),
            Some(11)
        );
    }

    #[test]
    fn find_release_id_by_tag_prefers_published_over_draft() {
        // A tag really can carry both: jerus-org/gen-circleci-orb v0.0.41 has an
        // abandoned empty draft (13:06) alongside the published release that
        // followed it (13:43). The published one is the finished article, and in
        // a draft-first pipeline its existence means the run already completed —
        // so it wins. Preferring the draft would upload assets into an invisible
        // release and then try to publish a second release for the same tag.
        let releases = [
            ("pcu-v0.6.29", 10i64, true),
            ("pcu-v0.6.29", 11, false),
            ("pcu-v0.6.29", 12, true),
        ];
        assert_eq!(
            find_release_id_by_tag(releases.iter().copied(), "pcu-v0.6.29"),
            Some(11)
        );
    }

    #[test]
    fn find_release_id_by_tag_falls_back_to_the_first_draft() {
        // With no published release the draft is the one to reuse — the retry
        // case a draft-first pipeline depends on.
        let releases = [("pcu-v0.6.29", 11i64, true), ("pcu-v0.6.29", 12, true)];
        assert_eq!(
            find_release_id_by_tag(releases.iter().copied(), "pcu-v0.6.29"),
            Some(11)
        );
    }

    #[test]
    fn find_release_id_by_tag_returns_none_when_absent() {
        let releases = [("pcu-v0.6.28", 10i64, false)];
        assert_eq!(
            find_release_id_by_tag(releases.iter().copied(), "pcu-v0.6.29"),
            None
        );
    }

    #[test]
    fn find_release_id_by_tag_requires_an_exact_match() {
        // The nextsv prefix-regex class of bug: 'v0.1.6' must not match inside
        // 'gen-changelog-v0.1.6'.
        let releases = [("gen-changelog-v0.1.6", 10i64, false)];
        assert_eq!(
            find_release_id_by_tag(releases.iter().copied(), "v0.1.6"),
            None
        );
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
