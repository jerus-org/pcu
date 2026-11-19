use std::{
    borrow::Cow, collections::HashMap, env, ffi::OsString, fmt::Debug, path::PathBuf, sync::Arc,
};

pub(crate) mod graphql;
mod pull_request;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use config::Config;
use git2::Repository;
use keep_a_changelog::{ChangeKind, ChangelogParseOptions};
use octocrab::{
    models::{AppId, InstallationId},
    Octocrab,
};
use owo_colors::{OwoColorize, Style};
use secrecy::ExposeSecret;
use tokio::sync::OnceCell;

use self::pull_request::PullRequest;
use crate::{Error, PrTitle};

const END_POINT: &str = "https://api.github.com/graphql";

pub struct Client {
    #[allow(dead_code)]
    // pub(crate) settings: Config,
    pub(crate) git_repo: Repository,
    /// Deferred: `Octocrab::builder().build()` spawns a tower buffer task
    /// internally, so it must never run outside an already-entered Tokio
    /// runtime. `new_with` (already `pub async fn`) pre-fills this via
    /// `octocrab()`'s first call; `new_local_at` (a sync fn, documented as
    /// callable before any runtime exists) leaves it empty and lets
    /// `octocrab()` build it lazily on first actual use — see
    /// jerus-org/pcu#1085, and `pcu_release_assets::ReleaseAssetClient`'s
    /// identical field, which has the same rationale in more detail.
    pub(crate) github_rest: Arc<OnceCell<Arc<Octocrab>>>,
    pub(crate) github_graphql: Arc<gql_client::Client>,
    pub(crate) github_token: String,
    pub(crate) owner: String,
    pub(crate) repo: String,
    /// Release-lookup/asset-download read path, in its own crate so a
    /// consumer like jci-audit can depend on just that (jerus-org/pcu#1051).
    /// Shares `github_rest`/`github_graphql` above via `Arc`.
    release_assets: pcu_release_assets::ReleaseAssetClient,
    /// Asset-upload/publish write path (jerus-org/pcu#1059, delegated here
    /// rather than duplicated per jerus-org/pcu#1061). Shares
    /// `github_rest`/`github_graphql` above via `Arc`, same as
    /// `release_assets`.
    release_asset_writer: pcu_release_assets::ReleaseAssetWriter,
    pub(crate) default_branch: String,
    pub(crate) branch: Option<String>,
    pull_request: Option<PullRequest>,
    pub(crate) prlog: OsString,
    pub(crate) line_limit: usize,
    pub(crate) prlog_parse_options: ChangelogParseOptions,
    pub(crate) prlog_update: Option<PrTitle>,
    pub(crate) commit_message: String,
    /// `GNUPGHOME` handed to the `gpg` and `git tag -s` subprocesses. `None`
    /// inherits the environment, as production wants; tests point it at an
    /// ephemeral keyring instead of mutating the process environment.
    pub(crate) gnupg_home: Option<PathBuf>,
}

/// Names of the installation's permissions that are more than read access.
///
/// Reads the raw `permissions` object: octocrab's typed model drops any
/// permission it does not list, which would hide a write permission. An
/// installation that reports no permissions object is refused, not assumed
/// read-only.
pub(crate) fn write_permissions(permissions: &serde_json::Value) -> Result<Vec<String>, Error> {
    let object = permissions.as_object().ok_or_else(|| {
        Error::AppNotReadOnly("the installation reported no permissions".to_string())
    })?;
    let mut names: Vec<String> = object
        .iter()
        .filter(|(_, level)| level.as_str() != Some("read"))
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();
    Ok(names)
}

/// A client for `owner/repo` authenticated as GitHub App `app_id` (with the
/// `private_key` in `settings`), plus the installation token it uses.
///
/// The token is scoped to the repository, limited to the App's own
/// permissions, and expires after an hour. With `read_only` the App is refused
/// when any of its permissions is more than read, so a caller that only reads
/// cannot end up holding a privileged App's credentials.
pub(crate) async fn app_installation_client(
    app_id: &str,
    settings: &Config,
    owner: &str,
    repo: &str,
    read_only: bool,
) -> Result<(Octocrab, String), Error> {
    let encoding_key = app_encoding_key(settings)?;
    let numeric_id: u64 = app_id
        .trim()
        .parse()
        .map_err(|_| Error::InvalidAppId(app_id.to_string()))?;

    let app_client = Octocrab::builder()
        .app(AppId(numeric_id), encoding_key)
        .build()?;

    // The leading slash matters: octocrab rejects a relative route.
    let installation: serde_json::Value = app_client
        .get::<_, _, ()>(format!("/repos/{owner}/{repo}/installation"), None)
        .await?;
    let installation_id = installation
        .get("id")
        .and_then(serde_json::Value::as_u64)
        .ok_or(Error::NoGitHubAPIAuth)?;
    if read_only {
        let writable = write_permissions(&installation["permissions"])?;
        if !writable.is_empty() {
            return Err(Error::AppNotReadOnly(writable.join(", ")));
        }
    }
    let (github_rest, token) = app_client
        .installation_and_token(InstallationId(installation_id))
        .await?;

    Ok((github_rest, token.expose_secret().to_string()))
}

/// The GitHub App's signing key from the `private_key` setting.
///
/// A missing setting and an unusable value are different problems and get
/// different errors: "not found" sends the reader looking for a missing or
/// misnamed variable, which is wrong when the variable is there and its value
/// is in the wrong shape (base64, escaped newlines, no PEM header).
fn app_encoding_key(settings: &Config) -> Result<octocrab::EncodingKey, Error> {
    let raw = match settings.get::<String>("private_key") {
        Ok(value) => value,
        Err(config::ConfigError::NotFound(_)) => return Err(Error::NoGitHubAPIPrivateKey),
        Err(_) => {
            return Err(Error::InvalidGitHubAPIPrivateKey(
                "the setting could not be read as text".to_string(),
            ))
        }
    };

    let (pem, stored_as) = normalise_private_key(&raw);
    if let Some(shape) = stored_as {
        log::warn!(
            "The GitHub App private key was stored {shape}; accepted, but store the PEM text itself"
        );
    }

    octocrab::EncodingKey::from_rsa_pem(pem.as_bytes())
        .map_err(|_| Error::InvalidGitHubAPIPrivateKey(describe_unusable_private_key(&pem)))
}

/// Undo the two common ways a multi-line PEM gets mangled when it is stored in
/// a single-line setting, so they work instead of failing with a vague error.
/// Returns the PEM to parse and, when it had to be changed, how it was stored.
///
/// - base64 of the PEM: CI contexts often need a multi-line value encoded
/// - newlines pasted as the two characters `\n`
///
/// Both are unambiguous: a value that is not already PEM text and decodes to
/// PEM text, or one with `\n` escapes and no real line breaks.
fn normalise_private_key(raw: &str) -> (Cow<'_, str>, Option<&'static str>) {
    let trimmed = raw.trim();

    if !trimmed.contains("-----BEGIN") {
        let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if let Ok(bytes) = STANDARD.decode(compact) {
            if let Ok(text) = String::from_utf8(bytes) {
                if text.contains("-----BEGIN") {
                    return (Cow::Owned(text), Some("as base64"));
                }
            }
        }
        return (Cow::Borrowed(raw), None);
    }

    if !trimmed.contains('\n') && trimmed.contains("\\n") {
        return (
            Cow::Owned(trimmed.replace("\\n", "\n")),
            Some("with literal \\n instead of line breaks"),
        );
    }

    (Cow::Borrowed(raw), None)
}

/// What is wrong with a value that is not a usable RSA private key, judged from
/// its shape. Never includes any of the value apart from a PEM label drawn from
/// a fixed list, because the value is a secret.
fn describe_unusable_private_key(value: &str) -> String {
    const WRONG_KINDS: [&str; 5] = [
        "EC PRIVATE KEY",
        "ENCRYPTED PRIVATE KEY",
        "OPENSSH PRIVATE KEY",
        "PUBLIC KEY",
        "CERTIFICATE",
    ];

    let value = value.trim();
    if value.is_empty() {
        return "the value is empty".to_string();
    }
    let Some(start) = value.find("-----BEGIN ") else {
        return "it has no `-----BEGIN ...-----` line; expected the PEM text of the app's private \
                key as downloaded from GitHub, not just the key body"
            .to_string();
    };

    let after = &value[start + "-----BEGIN ".len()..];
    let label = after.split("-----").next().unwrap_or("");
    match label {
        "RSA PRIVATE KEY" | "PRIVATE KEY" => "it has a PEM header but the key could not be \
            parsed; it may be truncated or corrupted, or have lost the line breaks inside the \
            key body"
            .to_string(),
        known if WRONG_KINDS.contains(&known) => format!(
            "it contains a `{known}`; expected an RSA private key (`BEGIN RSA PRIVATE KEY` or \
             `BEGIN PRIVATE KEY`)"
        ),
        _ => "its PEM header is not an RSA private key; expected `BEGIN RSA PRIVATE KEY` or \
              `BEGIN PRIVATE KEY`"
            .to_string(),
    }
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

        // Use the username config settings to direct to the appropriate CI
        // environment variable to find the owner
        log::trace!("owner: {:?}", settings.get::<String>("username"));
        let pcu_owner: String = settings
            .get("username")
            .map_err(|_| Error::EnvVarBranchNotSet)?;
        let owner = env::var(pcu_owner).map_err(|_| Error::EnvVarBranchNotFound)?;

        // Use the reponame config settings to direct to the appropriate CI
        // environment variable to find the repo
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

        // Does not dump `settings` — it can carry a GitHub App private key or
        // PAT, and `Config`'s Debug output has no secret redaction.
        log::trace!("Getting the github api for {owner}, {repo}");
        let (github_rest, github_graphql, github_token) =
            Client::get_github_apis(settings, &owner, &repo).await?;

        let git_repo = git2::Repository::open(".")?;

        log::trace!("Executing for command: {cmd}");
        let (branch, pull_request) = if &cmd == "pr" || &cmd == "push" {
            // Use the branch config settings to direct to the appropriate CI
            // environment variable to find the branch data
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

        let github_rest = Arc::new(github_rest);
        let github_graphql = Arc::new(github_graphql);
        let release_assets = pcu_release_assets::ReleaseAssetClient::from_shared(
            owner.clone(),
            repo.clone(),
            github_token.clone(),
            Arc::clone(&github_rest),
            Arc::clone(&github_graphql),
        );
        let release_asset_writer = pcu_release_assets::ReleaseAssetWriter::from_shared(
            owner.clone(),
            repo.clone(),
            github_token.clone(),
            Arc::clone(&github_rest),
            Arc::clone(&github_graphql),
        );
        let github_rest = Arc::new(OnceCell::new_with(Some(github_rest)));

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
            release_asset_writer,
            pull_request,
            prlog,
            line_limit,
            prlog_parse_options,
            prlog_update: None,
            commit_message,
            gnupg_home: None,
        })
    }

    /// Get the GitHub API instance
    async fn get_github_apis(
        settings: &Config,
        owner: &str,
        repo: &str,
    ) -> Result<(Octocrab, gql_client::Client, String), Error> {
        let bld_style = Style::new().bold();
        log::info!("\n***Get GitHub API instance***\n");
        // Does not dump `settings` — it can carry a GitHub App private key or
        // PAT, and `Config`'s Debug output has no secret redaction.
        let (github_rest, token) = match settings.get::<String>("app_id") {
            Ok(app_id) => {
                log::info!("Using {} for authentication", "GitHub App".style(bld_style));
                app_installation_client(&app_id, settings, owner, repo, false).await?
            }
            Err(_) => {
                let pat = settings
                    .get::<String>("pat")
                    .map_err(|_| Error::NoGitHubAPIAuth)?;
                log::warn!(
                    "Falling back to {} for authentication — PAT lacks branch protection bypass authority",
                    "Personal Access Token".style(bld_style)
                );

                let github_rest = Octocrab::builder().personal_token(pat.clone()).build()?;

                (github_rest, pat)
            }
        };

        let auth = format!("Bearer {token}");

        let headers = HashMap::from([
            ("X-Github-Next-Global-ID", "1"),
            ("User-Agent", owner),
            ("Authorization", &auth),
        ]);

        let github_graphql = gql_client::Client::new_with_headers(END_POINT, headers);

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

    /// The `Octocrab` REST client, building it on first use if this
    /// `Client` was constructed via [`Self::new_local`]/[`Self::new_local_at`]
    /// (deferred — see the field doc on `github_rest`) or returning the
    /// already-built one immediately if constructed via [`Self::new_with`].
    pub(crate) async fn octocrab(&self) -> Result<&Octocrab, Error> {
        let arc = self
            .github_rest
            .get_or_try_init(|| async {
                Octocrab::builder()
                    .personal_token(self.github_token.clone())
                    .build()
                    .map(Arc::new)
            })
            .await?;
        Ok(arc.as_ref())
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
    /// Unlike [`Client::new_with`], this constructor does not make any GitHub
    /// API calls and does not require CI environment variables.  It derives
    /// `owner` and `repo` from the `origin` remote URL of the repository at
    /// the current working directory.
    ///
    /// Methods that require GitHub authentication (e.g. `push_commit` with App
    /// bypass) will fail unless credentials are available through the usual
    /// environment variables — but operations that are purely local (e.g.
    /// `commit_staged`, `stage_paths`) work without any network access.
    pub fn new_local() -> Result<Self, Error> {
        Self::new_local_at(std::path::Path::new("."))
    }

    /// Like [`Client::new_local`] but opens the repository at an explicit
    /// `path`.
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

        // Deferred, like `github_rest` itself (see its field doc) — building
        // an `Octocrab` client eagerly here would need a Tokio runtime this
        // sync constructor is documented not to require. Any method that
        // actually uses one of these will fail with an auth error, which is
        // expected for a local-only client with an empty token.
        let github_rest = Arc::new(OnceCell::new());
        let github_graphql = Arc::new(gql_client::Client::new_with_headers(
            END_POINT,
            HashMap::from([
                ("X-Github-Next-Global-ID", "1"),
                ("User-Agent", "pcu-local"),
                ("Authorization", "Bearer local"),
            ]),
        ));

        let prlog = OsString::from("PRLOG.md");
        let prlog_parse_options = ChangelogParseOptions {
            url: None,
            head: Some("HEAD".to_string()),
            tag_prefix: Some("v".to_string()),
        };

        // `from_shared_cell`, not `from_shared` — `github_rest` above is
        // still empty, not a ready-built client, but sharing the same cell
        // (rather than each independently deferring its own, as a plain
        // `::new` would) means `Client`, `release_assets`, and
        // `release_asset_writer` all build and cache a single instance on
        // first real use instead of three.
        let release_assets = pcu_release_assets::ReleaseAssetClient::from_shared_cell(
            owner.clone(),
            repo.clone(),
            "",
            Arc::clone(&github_rest),
            Arc::clone(&github_graphql),
        );
        let release_asset_writer = pcu_release_assets::ReleaseAssetWriter::from_shared_cell(
            owner.clone(),
            repo.clone(),
            "",
            Arc::clone(&github_rest),
            Arc::clone(&github_graphql),
        );

        Ok(Self {
            git_repo,
            github_rest,
            github_graphql,
            github_token: String::new(),
            owner,
            repo,
            release_assets,
            release_asset_writer,
            default_branch: "main".to_string(),
            branch,
            pull_request: None,
            prlog,
            line_limit: 10,
            prlog_parse_options,
            prlog_update: None,
            commit_message: String::new(),
            gnupg_home: None,
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
    /// `Ok(None)` means no release exists for the tag — a legitimate answer
    /// when deciding whether to create one, and an error only at the call
    /// sites that require a release to already be there.
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
    /// Idempotent: if an asset with the same name already exists on the
    /// release it is deleted first (delete-then-replace), so retries don't
    /// fail with GitHub's 422 "Validation Failed".
    ///
    /// Delegates to [`pcu_release_assets::ReleaseAssetWriter`] — see
    /// jerus-org/pcu#1061 for why this is no longer its own copy.
    pub async fn upload_release_asset(
        &self,
        tag: &str,
        binary: &std::path::Path,
        asset_name: &str,
    ) -> Result<(), Error> {
        Ok(self
            .release_asset_writer
            .upload_release_asset(tag, binary, asset_name)
            .await?)
    }

    /// Download a named asset from an existing GitHub release, returning its
    /// raw bytes.
    ///
    /// Refuses a draft release unless `allow_draft` is `true` — a draft's
    /// assets can still be replaced, so callers that need a finalised
    /// artefact (e.g. an audit record) should read from a published release
    /// by default and opt in explicitly if a draft is acceptable.
    ///
    /// Delegates to [`pcu_release_assets::ReleaseAssetClient`]
    /// (jerus-org/pcu#1051). For a read-only, checkout-free,
    /// no-draft-access client see
    /// [`pcu_release_assets::ReleaseAssetClient::download_release_asset`]
    /// directly.
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

    /// Publish the release for `release_id` when it is still a draft.
    ///
    /// Unconditional by design — the release pipeline runs this as its last
    /// step with no condition attached — so it must be a
    /// no-op wherever there is nothing to publish. `make_latest` is set here
    /// rather than at creation because GitHub does not accept it on a draft.
    ///
    /// Delegates to [`pcu_release_assets::ReleaseAssetWriter`] — see
    /// jerus-org/pcu#1061 for why this is no longer its own copy.
    pub(crate) async fn publish_release(&self, release_id: i64) -> Result<(), Error> {
        Ok(self
            .release_asset_writer
            .publish_release_by_id(release_id)
            .await?)
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

    /// jerus-org/pcu#1085: `new_local`/`new_local_at` are documented as
    /// callable with no GitHub API access at all — the same sync-before-any-
    /// runtime contract as `pcu_release_assets::ReleaseAssetClient::new`.
    /// Deliberately plain `#[test]`, not `#[tokio::test]` — no runtime
    /// exists anywhere in this test at all.
    #[test]
    fn new_local_at_does_not_require_a_tokio_runtime() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let _client = Client::new_local_at(dir.path()).unwrap();
    }

    /// `release_assets`/`release_asset_writer` are built via
    /// `from_shared_cell`, sharing `Client`'s own deferred `github_rest`
    /// cell — so all three build (and cache) a single `Octocrab` instance on
    /// first real use, rather than three independent ones (jerus-org/pcu#1085
    /// code review follow-up). The count is exactly 3: `Client`'s own field,
    /// plus one clone handed to each of `release_assets` and
    /// `release_asset_writer`.
    #[test]
    fn new_local_at_shares_one_deferred_client_across_all_three_fields() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let client = Client::new_local_at(dir.path()).unwrap();
        assert_eq!(Arc::strong_count(&client.github_rest), 3);
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

    #[tokio::test]
    async fn upload_release_asset_returns_error_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let client = Client::new_local_at(dir.path()).unwrap();

        let result = client
            .upload_release_asset(
                "v1.0.0",
                std::path::Path::new("/nonexistent/binary"),
                "binary",
            )
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Asset file not found"), "unexpected: {msg}");
    }

    // --- GitHub App private key handling (#1136) ---------------------------

    use std::sync::OnceLock;

    use aws_lc_rs::{
        encoding::{AsDer, Pkcs8V1Der},
        rsa::{KeyPair, KeySize},
    };

    /// A throwaway RSA key as PKCS#8 PEM text, generated once per test run so
    /// no key material is committed to the repository.
    fn test_pem() -> &'static str {
        static PEM: OnceLock<String> = OnceLock::new();
        PEM.get_or_init(|| {
            let key = KeyPair::generate(KeySize::Rsa2048).unwrap();
            let der: Pkcs8V1Der<'static> = key.as_der().unwrap();
            let body = STANDARD.encode(der.as_ref());
            let wrapped: Vec<&str> = body
                .as_bytes()
                .chunks(64)
                .map(|c| std::str::from_utf8(c).unwrap())
                .collect();
            format!(
                "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
                wrapped.join("\n")
            )
        })
    }

    fn settings_with_key(value: &str) -> Config {
        Config::builder()
            .set_override("private_key", value)
            .unwrap()
            .build()
            .unwrap()
    }

    fn key_error(value: &str) -> Error {
        match app_encoding_key(&settings_with_key(value)) {
            Ok(_) => panic!("{value:?} should not be a usable key"),
            Err(e) => e,
        }
    }

    /// A key body line from the real key, used to check an error never echoes it.
    fn secret_fragment() -> String {
        test_pem().lines().nth(1).unwrap().to_string()
    }

    #[test]
    fn a_missing_private_key_is_reported_as_missing_and_names_the_variable() {
        let settings = Config::builder().build().unwrap();

        let err = app_encoding_key(&settings).map(|_| ()).unwrap_err();

        assert!(matches!(err, Error::NoGitHubAPIPrivateKey));
        let message = err.to_string();
        assert!(message.contains("PCU_PRIVATE_KEY"), "got: {message}");
    }

    #[test]
    fn a_plain_pem_is_accepted_without_comment() {
        let pem = test_pem();

        assert!(app_encoding_key(&settings_with_key(pem)).is_ok());
        let (normalised, note) = normalise_private_key(pem);
        assert!(note.is_none());
        assert!(matches!(normalised, Cow::Borrowed(_)), "untouched: no copy");
    }

    #[test]
    fn the_common_mangled_shapes_are_normalised_and_accepted() {
        let pem = test_pem();
        let cases = [
            ("base64 of the PEM", STANDARD.encode(pem), Some("as base64")),
            (
                "base64 split across lines",
                STANDARD
                    .encode(pem)
                    .as_bytes()
                    .chunks(60)
                    .map(|c| std::str::from_utf8(c).unwrap())
                    .collect::<Vec<_>>()
                    .join("\n"),
                Some("as base64"),
            ),
            (
                "literal \\n for line breaks",
                pem.trim_end().replace('\n', "\\n"),
                Some("with literal \\n instead of line breaks"),
            ),
        ];
        for (name, value, expected_note) in cases {
            let (normalised, note) = normalise_private_key(&value);

            assert_eq!(note, expected_note, "{name}");
            assert_eq!(
                normalised.trim_end(),
                pem.trim_end(),
                "{name} restores the PEM"
            );
            assert!(
                app_encoding_key(&settings_with_key(&value)).is_ok(),
                "{name}"
            );
        }
    }

    #[test]
    fn shapes_the_parser_already_accepts_are_left_alone() {
        let pem = test_pem();
        for value in [
            pem.replace('\n', "\r\n"),
            format!("\"{}\"", pem.trim_end()),
            pem.trim_end().replace('\n', " "),
        ] {
            let (_, note) = normalise_private_key(&value);

            assert!(note.is_none());
            assert!(app_encoding_key(&settings_with_key(&value)).is_ok());
        }
    }

    #[test]
    fn an_unusable_key_is_not_reported_as_missing() {
        for value in ["", "   ", "just some text", "AAAA"] {
            let err = key_error(value);

            assert!(
                matches!(err, Error::InvalidGitHubAPIPrivateKey(_)),
                "{value:?}: {err}"
            );
            assert!(!err.to_string().contains("No GitHub API private key found"));
        }
    }

    #[test]
    fn the_message_says_what_is_wrong_with_the_value() {
        let pem = test_pem();
        let body_only: String = pem
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect::<Vec<_>>()
            .join("\n");
        let truncated = format!("{}\n-----END PRIVATE KEY-----\n", &pem[..pem.len() / 2]);
        let cases = [
            ("", "empty"),
            (body_only.as_str(), "no `-----BEGIN"),
            (
                "-----BEGIN EC PRIVATE KEY-----\nAAAA\n-----END EC PRIVATE KEY-----",
                "`EC PRIVATE KEY`",
            ),
            (
                "-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----",
                "`PUBLIC KEY`",
            ),
            (
                "-----BEGIN SOMETHING ELSE-----\nAAAA\n-----END SOMETHING ELSE-----",
                "not an RSA private key",
            ),
            (truncated.as_str(), "truncated or corrupted"),
        ];
        for (value, expected) in cases {
            let message = key_error(value).to_string();

            assert!(
                message.contains(expected),
                "{expected:?} missing from: {message}"
            );
        }
    }

    /// The value is a secret: no error may contain any of it.
    #[test]
    fn errors_never_contain_the_key() {
        let fragment = secret_fragment();
        let pem = test_pem();
        let values = [
            pem[..pem.len() / 2].to_string(),
            pem.lines().skip(1).collect::<Vec<_>>().join("\n"),
            pem.replace("PRIVATE KEY", "WEIRD KEY"),
        ];
        for value in values {
            let message = key_error(&value).to_string();

            assert!(
                !message.contains(&fragment),
                "the message leaks part of the key: {message}"
            );
            assert!(message.len() < 400, "short and fixed: {message}");
        }
    }

    fn permissions(json: &str) -> serde_json::Value {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_read_only_installation_has_no_write_permissions() {
        let read_only =
            permissions(r#"{"contents":"read","metadata":"read","pull_requests":"read"}"#);
        assert!(write_permissions(&read_only).unwrap().is_empty());
    }

    #[test]
    fn write_and_admin_permissions_are_named_including_ones_octocrab_does_not_model() {
        let broad = permissions(
            r#"{"contents":"write","metadata":"read","administration":"admin","issues":"read"}"#,
        );
        assert_eq!(
            write_permissions(&broad).unwrap(),
            vec!["administration".to_string(), "contents".to_string()]
        );
    }

    #[test]
    fn an_empty_permissions_object_is_read_only() {
        assert!(write_permissions(&permissions("{}")).unwrap().is_empty());
    }

    #[test]
    fn a_missing_permissions_object_is_refused_not_assumed_read_only() {
        for bad in ["null", "[]", "\"read\""] {
            let err = write_permissions(&permissions(bad))
                .unwrap_err()
                .to_string();
            assert!(err.contains("reported no permissions"), "{bad}: {err}");
        }
    }
}
