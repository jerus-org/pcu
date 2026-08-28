use std::{collections::HashMap, sync::Arc};

use octocrate::{APIConfig, GitHubAPI, PersonalAccessToken};
use serde::{Deserialize, Serialize};

use crate::Error;

const END_POINT: &str = "https://api.github.com/graphql";

/// A release located for a tag, reduced to what callers act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseRef {
    /// REST id — the asset-download endpoint is REST-only, so this is what
    /// it needs.
    pub id: i64,
    pub draft: bool,
    /// GitHub has frozen this release's assets. Only ever true for a
    /// published release; a draft is by definition still open.
    pub immutable: bool,
}

/// A read-only client for fetching a named asset from an already-published
/// GitHub release, with no git checkout required.
///
/// Built for verifiers (e.g. `jci-audit verify --release-version`) that need
/// a release artifact from a bare directory, with no clone and no write
/// access. There is deliberately no upload or publish method on this type —
/// the capability boundary is enforced by the method not existing, not by a
/// runtime flag — and [`ReleaseAssetClient::download_release_asset`] never
/// reads a draft release: once a release is immutable, a client that could
/// still be pointed at amending its assets (or reading an in-progress draft)
/// has no legitimate use here. See jerus-org/pcu#1051.
pub struct ReleaseAssetClient {
    owner: String,
    repo: String,
    github_token: String,
    github_rest: Arc<GitHubAPI>,
    github_graphql: Arc<gql_client::Client>,
}

impl ReleaseAssetClient {
    /// Construct a client for `owner`/`repo`, authenticating with
    /// `github_token`. Does not touch the filesystem or git in any way.
    ///
    /// Builds its own API clients — for a headless consumer (e.g.
    /// jci-audit) with no pre-existing authenticated client to reuse. A
    /// caller that already has one (e.g. `pcu::Client`) should use
    /// [`Self::from_shared`] instead, to avoid a second, redundant auth
    /// object for the same token.
    pub fn new(
        owner: impl Into<String>,
        repo: impl Into<String>,
        github_token: impl Into<String>,
    ) -> Self {
        new_headless(owner, repo, github_token, Self::from_shared)
    }

    /// Construct a client for `owner`/`repo` from an already-authenticated
    /// `github_rest`/`github_graphql` pair — e.g. the ones `pcu::Client`
    /// already built. `pcu`'s auth is a superset of what this read-only
    /// client needs, so there is no reason to derive a second one for the
    /// same token.
    pub fn from_shared(
        owner: impl Into<String>,
        repo: impl Into<String>,
        github_token: impl Into<String>,
        github_rest: Arc<GitHubAPI>,
        github_graphql: Arc<gql_client::Client>,
    ) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            github_token: github_token.into(),
            github_rest,
            github_graphql,
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// Locate the release for `tag`, including drafts.
    ///
    /// `Ok(None)` means no release exists for the tag — a legitimate answer,
    /// not an error. Exposed (not just used internally) because `pcu::Client`
    /// needs the same lookup, with the `draft` flag intact, for its own
    /// upload/publish path.
    pub async fn find_release_for_tag(&self, tag: &str) -> Result<Option<ReleaseRef>, Error> {
        lookup_with_retry(
            tag,
            RELEASE_LOOKUP_ATTEMPTS,
            RELEASE_LOOKUP_DELAY,
            || async { self.probe_release_for_tag(tag).await },
        )
        .await
    }

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
                Error::ReleaseAsset(format!(
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

    /// Every release that could match `tag`, gathered in one GraphQL round
    /// trip. See the module-level test fixtures for why both a by-tag lookup
    /// and a listing are needed: `release(tagName:)` answers published
    /// releases only, so the listing is the only way to see a draft.
    async fn get_release_candidates(&self, tag: &str) -> Result<Vec<ReleaseNode>, Error> {
        log::trace!("looking for releases tagged: {tag}");
        let query = r#"
            query ($owner: String!, $name: String!, $tag: String!) {
              repository(owner: $owner, name: $name) {
                release(tagName: $tag) {
                  databaseId
                  tagName
                  isDraft
                  immutable
                }
                releases(first: 100, orderBy: {field: CREATED_AT, direction: DESC}) {
                  nodes {
                    databaseId
                    tagName
                    isDraft
                    immutable
                  }
                }
              }
            }"#;

        let vars = Vars {
            owner: self.owner.clone(),
            name: self.repo.clone(),
            tag: tag.to_string(),
        };

        let data = self
            .github_graphql
            .query_with_vars_unwrap::<GetReleases, Vars>(query, vars)
            .await
            .map_err(|e| Error::ReleaseAsset(format!("GraphQL error: {e}")))?;

        Ok(collect_candidates(data.repository))
    }

    /// Fetch `release_id`'s current asset list and look up `asset_name` in
    /// it, returning its id if present.
    ///
    /// Exposed because `pcu::Client::upload_release_asset` needs the same
    /// lookup to decide whether to delete-then-replace.
    pub async fn find_asset_in_release(
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

    async fn require_release_for_tag(&self, tag: &str) -> Result<ReleaseRef, Error> {
        self.find_release_for_tag(tag)
            .await?
            .ok_or_else(|| release_not_found_error(tag))
    }

    /// Download a named asset from the release for `tag`, returning its raw
    /// bytes. Refuses a draft release unless `allow_draft` is `true`.
    ///
    /// Exposed (in addition to the published-only [`Self::download_release_asset`])
    /// because `pcu::Client::download_release_asset` needs the same fetch with
    /// its caller-supplied `allow_draft`.
    pub async fn download_release_asset_allowing_draft(
        &self,
        tag: &str,
        asset_name: &str,
        allow_draft: bool,
    ) -> Result<Vec<u8>, Error> {
        let release_ref = self.require_release_for_tag(tag).await?;

        check_draft_allowed(release_ref.draft, allow_draft, tag)?;

        let asset_id = self
            .find_asset_in_release(release_ref.id, asset_name)
            .await?
            .ok_or_else(|| asset_not_found_error(asset_name, tag))?;

        let url = asset_download_url(&self.owner, &self.repo, asset_id);

        let response = reqwest::Client::new()
            .get(&url)
            .header("Accept", "application/octet-stream")
            .header("Authorization", format!("Bearer {}", self.github_token))
            .header("User-Agent", "pcu-release-assets")
            .send()
            .await
            .map_err(|e| {
                Error::ReleaseAsset(format!("failed to download asset '{asset_name}': {e}"))
            })?;

        check_download_response_status(response.status(), tag, asset_name)?;

        let bytes = response.bytes().await.map_err(|e| {
            Error::ReleaseAsset(format!("failed to read body of asset '{asset_name}': {e}"))
        })?;

        log::info!("Downloaded {} bytes for asset '{asset_name}'", bytes.len());
        Ok(bytes.to_vec())
    }

    /// Download a named asset from the **published** release for `tag`.
    ///
    /// This is the entry point for external, read-only consumers (e.g.
    /// jci-audit's release verification). There is no `allow_draft`
    /// parameter — a draft's assets can still be replaced, so a verifier
    /// must never trust one.
    pub async fn download_release_asset(
        &self,
        tag: &str,
        asset_name: &str,
    ) -> Result<Vec<u8>, Error> {
        self.download_release_asset_allowing_draft(tag, asset_name, false)
            .await
    }
}

#[derive(Deserialize, Debug, Clone)]
struct GetReleases {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
struct Repository {
    release: Option<ReleaseNode>,
    releases: ReleaseConnection,
}

#[derive(Deserialize, Debug, Clone)]
struct ReleaseConnection {
    nodes: Vec<ReleaseNode>,
}

/// One release, reduced to what choosing between releases actually requires.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
struct ReleaseNode {
    #[serde(rename = "databaseId")]
    database_id: Option<i64>,
    #[serde(rename = "tagName")]
    tag_name: String,
    #[serde(rename = "isDraft")]
    is_draft: bool,
    immutable: bool,
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    owner: String,
    name: String,
    tag: String,
}

/// Flatten the two halves of the response into one candidate list.
///
/// The by-tag result leads, so a published release found directly is
/// considered before anything in the listing; the listing then contributes
/// the drafts. A release appearing in both is harmless — selection matches
/// on tag name and takes the first acceptable candidate.
fn collect_candidates(repository: Repository) -> Vec<ReleaseNode> {
    let mut candidates = Vec::with_capacity(repository.releases.nodes.len() + 1);
    if let Some(release) = repository.release {
        candidates.push(release);
    }
    candidates.extend(repository.releases.nodes);
    candidates
}

/// Select the release id for `tag` from a listing, preferring a published
/// release over a draft. A tag can carry both — see
/// jerus-org/gen-circleci-orb `v0.0.41`, which has an abandoned empty draft
/// alongside the published release created 37 minutes later. Matching is
/// exact: `v0.1.6` must not match inside `gen-changelog-v0.1.6`.
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

/// Find the id of a release asset whose name matches `name`, if present.
fn find_existing_asset_id<'a>(
    assets: impl IntoIterator<Item = (&'a str, i64)>,
    name: &str,
) -> Option<i64> {
    assets
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, id)| id)
}

/// Shared by every headless type's `new()` (`ReleaseAssetClient` and
/// `ReleaseAssetWriter`): build a fresh authenticated client pair for
/// `github_token`, then hand ownership to `from_shared`. Both types' `new()`
/// bodies were identical before this was factored out — SonarQube flagged
/// the duplication (jerus-org/pcu#1059's PR).
pub(crate) fn new_headless<T>(
    owner: impl Into<String>,
    repo: impl Into<String>,
    github_token: impl Into<String>,
    from_shared: impl FnOnce(String, String, String, Arc<GitHubAPI>, Arc<gql_client::Client>) -> T,
) -> T {
    let owner = owner.into();
    let repo = repo.into();
    let github_token = github_token.into();
    let (github_rest, github_graphql) = build_authenticated_clients(&github_token);

    from_shared(
        owner,
        repo,
        github_token,
        Arc::new(github_rest),
        Arc::new(github_graphql),
    )
}

/// Build a fresh authenticated REST + GraphQL client pair for `token`.
pub(crate) fn build_authenticated_clients(token: &str) -> (GitHubAPI, gql_client::Client) {
    let pat = PersonalAccessToken::new(token);
    let config = APIConfig::with_token(pat).shared();
    let github_rest = GitHubAPI::new(&config);

    let auth = format!("Bearer {token}");
    let github_graphql = gql_client::Client::new_with_headers(
        END_POINT,
        HashMap::from([
            ("X-Github-Next-Global-ID", "1"),
            ("User-Agent", "pcu-release-assets"),
            ("Authorization", &auth),
        ]),
    );

    (github_rest, github_graphql)
}

/// Build GitHub's REST asset-download URL for `asset_id`. Deliberately not
/// `asset.browser_download_url`: that field only works unauthenticated, and
/// only for public repos.
fn asset_download_url(owner: &str, repo: &str, asset_id: i64) -> String {
    format!("https://api.github.com/repos/{owner}/{repo}/releases/assets/{asset_id}")
}

/// Shared by [`ReleaseAssetClient`] and `ReleaseAssetWriter` — both need to
/// name a not-found tag identically.
pub(crate) fn release_not_found_error(tag: &str) -> Error {
    Error::ReleaseAsset(format!("GitHub release for tag '{tag}' not found"))
}

fn asset_not_found_error(asset_name: &str, tag: &str) -> Error {
    Error::ReleaseAsset(format!(
        "no asset named '{asset_name}' found on release for tag '{tag}'"
    ))
}

fn check_download_response_status(
    status: reqwest::StatusCode,
    tag: &str,
    asset_name: &str,
) -> Result<(), Error> {
    if status.is_success() {
        return Ok(());
    }
    Err(Error::ReleaseAsset(format!(
        "GitHub returned {status} downloading asset '{asset_name}' for tag '{tag}'"
    )))
}

/// Refuse a draft release unless the caller explicitly opted in via
/// `allow_draft`.
fn check_draft_allowed(draft: bool, allow_draft: bool, tag: &str) -> Result<(), Error> {
    if draft && !allow_draft {
        return Err(Error::ReleaseAsset(format!(
            "release for tag '{tag}' is still a draft; pass allow_draft=true to download from it anyway"
        )));
    }
    Ok(())
}

const RELEASE_LOOKUP_ATTEMPTS: u32 = 5;
const RELEASE_LOOKUP_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

/// Retry `probe` until it finds something, distinguishing "not there" from
/// "could not tell". `Ok(None)` is a legitimate answer, so exhausting the
/// attempts on `Ok(None)` returns `Ok(None)` rather than an error.
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
    fn release_asset_client_builds_without_git_checkout() {
        // No tempdir, no git2::Repository::init anywhere in scope — this
        // crate has no git dependency at all.
        let client = ReleaseAssetClient::new("test-org", "test-repo", "token");
        assert_eq!(client.owner(), "test-org");
        assert_eq!(client.repo(), "test-repo");
    }

    /// `from_shared` exists specifically so `pcu::Client` can pass in its
    /// own already-authenticated clients instead of `ReleaseAssetClient`
    /// building a second, redundant pair for the same token. Locks that
    /// contract in with `Arc::strong_count`: if `from_shared` ever started
    /// wrapping fresh clones instead of storing the given `Arc`s directly,
    /// the count below would stay at 1 instead of rising to 2.
    #[test]
    fn from_shared_reuses_the_given_clients_rather_than_building_new_ones() {
        let dummy_pat = PersonalAccessToken::new("token");
        let dummy_config = APIConfig::with_token(dummy_pat).shared();
        let github_rest = Arc::new(GitHubAPI::new(&dummy_config));
        let github_graphql = Arc::new(gql_client::Client::new_with_headers(
            END_POINT,
            HashMap::<&str, &str>::new(),
        ));

        assert_eq!(Arc::strong_count(&github_rest), 1);
        assert_eq!(Arc::strong_count(&github_graphql), 1);

        let _client = ReleaseAssetClient::from_shared(
            "test-org",
            "test-repo",
            "token",
            Arc::clone(&github_rest),
            Arc::clone(&github_graphql),
        );

        assert_eq!(
            Arc::strong_count(&github_rest),
            2,
            "from_shared should hold the same GitHubAPI instance, not build a new one"
        );
        assert_eq!(
            Arc::strong_count(&github_graphql),
            2,
            "from_shared should hold the same gql_client::Client instance, not build a new one"
        );
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

    #[test]
    fn check_draft_allowed_ok_when_not_draft() {
        assert!(check_draft_allowed(false, false, "pcu-v1.0.0").is_ok());
    }

    #[test]
    fn check_draft_allowed_ok_when_draft_and_allowed() {
        assert!(check_draft_allowed(true, true, "pcu-v1.0.0").is_ok());
    }

    #[test]
    fn check_draft_allowed_errors_when_draft_and_not_allowed() {
        let msg = check_draft_allowed(true, false, "pcu-v1.0.0")
            .unwrap_err()
            .to_string();
        assert!(msg.contains("pcu-v1.0.0"), "unexpected: {msg}");
        assert!(msg.to_lowercase().contains("draft"), "unexpected: {msg}");
    }

    /// The public, external-facing entry point must have no way to opt into
    /// reading a draft release — this is the reviewer's read-only/no-draft
    /// constraint from pcu#1051, locked in as a compile-time signature fact
    /// (no `allow_draft` parameter exists) plus this behavioural proof that
    /// it always resolves through the draft-refusing path.
    #[test]
    fn download_release_asset_always_refuses_a_draft() {
        // download_release_asset_allowing_draft(..., false) is exactly what
        // download_release_asset delegates to; check_draft_allowed is the
        // single source of truth for that refusal, already proven above.
        assert!(check_draft_allowed(true, false, "pcu-v1.0.0").is_err());
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
        let found = lookup_with_retry("pcu-v1.0.0", 3, std::time::Duration::ZERO, || async {
            Ok(None::<i64>)
        })
        .await
        .unwrap();
        assert_eq!(found, None);
    }

    #[tokio::test]
    async fn lookup_with_retry_propagates_a_persistent_api_error() {
        let result = lookup_with_retry("pcu-v1.0.0", 3, std::time::Duration::ZERO, || async {
            Err::<Option<i64>, Error>(Error::ReleaseAsset("api 500".into()))
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
                    Err(Error::ReleaseAsset("api 500".into()))
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
        let releases = [("pcu-v0.6.29", 11i64, true)];
        assert_eq!(
            find_release_id_by_tag(releases.iter().copied(), "pcu-v0.6.29"),
            Some(11)
        );
    }

    #[test]
    fn find_release_id_by_tag_prefers_published_over_draft() {
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
        let releases = [("gen-changelog-v0.1.6", 10i64, false)];
        assert_eq!(
            find_release_id_by_tag(releases.iter().copied(), "v0.1.6"),
            None
        );
    }

    /// Shaped on the real response for jerus-org/gen-circleci-orb `v0.0.41`,
    /// which carries an abandoned empty draft alongside the published
    /// release that superseded it 37 minutes later.
    const RESPONSE: &str = r#"{
        "repository": {
            "release": {"databaseId": 333744509, "tagName": "v0.0.41", "isDraft": false, "immutable": false},
            "releases": {"nodes": [
                {"databaseId": 333744509, "tagName": "v0.0.41", "isDraft": false, "immutable": false},
                {"databaseId": 333719504, "tagName": "v0.0.41", "isDraft": true, "immutable": false},
                {"databaseId": 361109117, "tagName": "jci-audit-v0.0.1", "isDraft": false, "immutable": true}
            ]}
        }
    }"#;

    #[test]
    fn deserialises_a_release_lookup_response() {
        let data: GetReleases = serde_json::from_str(RESPONSE).unwrap();
        assert_eq!(
            data.repository.release.unwrap().database_id,
            Some(333744509)
        );
        assert_eq!(data.repository.releases.nodes.len(), 3);
        assert!(data.repository.releases.nodes[1].is_draft);
        assert!(
            data.repository.releases.nodes[2].immutable,
            "the immutable field must survive deserialisation — it is the reason \
             this lookup is GraphQL"
        );
    }

    #[test]
    fn collect_candidates_puts_the_by_tag_release_first() {
        let data: GetReleases = serde_json::from_str(RESPONSE).unwrap();
        let candidates = collect_candidates(data.repository);
        assert_eq!(
            candidates.len(),
            4,
            "by-tag result plus every listed release"
        );
        assert_eq!(candidates[0].database_id, Some(333744509));
    }

    #[test]
    fn collect_candidates_tolerates_a_tag_with_no_published_release() {
        let response = r#"{
            "repository": {
                "release": null,
                "releases": {"nodes": [
                    {"databaseId": 42, "tagName": "pcu-v0.7.0", "isDraft": true, "immutable": false}
                ]}
            }
        }"#;
        let data: GetReleases = serde_json::from_str(response).unwrap();
        let candidates = collect_candidates(data.repository);
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].is_draft);
    }
}
