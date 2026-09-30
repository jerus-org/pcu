use std::{path::Path, sync::Arc};

use octocrab::{repos::releases::MakeLatest, Octocrab};
use secrecy::ExposeSecret;
use tokio::sync::OnceCell;

use crate::{
    client::{release_not_found_error, ReleaseAssetClient},
    Error,
};

/// A headless, write-capable client for a GitHub release's assets — upload
/// and publish, with no git checkout required.
///
/// Built for writers (e.g. `jci-audit publish-record`) that need to attach a
/// signed record to a release and optionally un-draft it, possibly from a
/// different, later job than whatever job created the draft. Sibling to
/// [`ReleaseAssetClient`], which stays read-only by design — see
/// jerus-org/pcu#1059. Shared read plumbing (release/asset lookup) is
/// composed via an internal `ReleaseAssetClient` rather than duplicated.
pub struct ReleaseAssetWriter {
    owner: String,
    repo: String,
    reader: ReleaseAssetClient,
}

impl ReleaseAssetWriter {
    /// Construct a writer for `owner`/`repo`, authenticating with
    /// `github_token`. Does not touch the filesystem or git in any way.
    ///
    /// Delegates entirely to [`ReleaseAssetClient::new`] for the actual
    /// client — including its runtime-deferred construction (see that
    /// type's `github_rest` field doc, jerus-org/pcu#1085) — rather than
    /// building its own, since a writer only ever needs the single
    /// `Octocrab` instance the composed `reader` already holds.
    pub fn new(
        owner: impl Into<String>,
        repo: impl Into<String>,
        github_token: impl Into<String>,
    ) -> Self {
        let owner = owner.into();
        let repo = repo.into();
        let reader = ReleaseAssetClient::new(owner.clone(), repo.clone(), github_token);
        Self {
            owner,
            repo,
            reader,
        }
    }

    /// Construct a writer for `owner`/`repo` from an already-authenticated
    /// `github_rest`/`github_graphql` pair — e.g. the ones `pcu::Client`
    /// already built. The `Arc`s are cloned (refcount only), not rebuilt, and
    /// back an internal [`ReleaseAssetClient`] for shared read operations —
    /// no second, redundant auth object for the same token, and no separate
    /// copy kept on `Self` either (unlike the octocrate-based client this
    /// replaced): every REST call goes through `self.reader.octocrab()`.
    /// `octocrab`'s `upload_asset` resolves the release's own upload URL
    /// itself, so there is no separate `uploads.github.com`-pointed client
    /// to build here any more.
    pub fn from_shared(
        owner: impl Into<String>,
        repo: impl Into<String>,
        github_token: impl Into<String>,
        github_rest: Arc<Octocrab>,
        github_graphql: Arc<gql_client::Client>,
    ) -> Self {
        let owner = owner.into();
        let repo = repo.into();

        let reader = ReleaseAssetClient::from_shared(
            owner.clone(),
            repo.clone(),
            github_token,
            github_rest,
            github_graphql,
        );

        Self {
            owner,
            repo,
            reader,
        }
    }

    /// Construct a writer for `owner`/`repo` sharing a caller's own
    /// still-possibly-empty, lazily-built `Octocrab` cell — the writer-side
    /// counterpart to [`ReleaseAssetClient::from_shared_cell`], for the same
    /// reason (jerus-org/pcu#1085): `pcu::Client::new_local_at`'s `Client`,
    /// `ReleaseAssetClient`, and this writer all build (and cache) the same
    /// single instance on first real use.
    pub fn from_shared_cell(
        owner: impl Into<String>,
        repo: impl Into<String>,
        github_token: impl Into<String>,
        github_rest: Arc<OnceCell<Arc<Octocrab>>>,
        github_graphql: Arc<gql_client::Client>,
    ) -> Self {
        let owner = owner.into();
        let repo = repo.into();

        let reader = ReleaseAssetClient::from_shared_cell(
            owner.clone(),
            repo.clone(),
            github_token,
            github_rest,
            github_graphql,
        );

        Self {
            owner,
            repo,
            reader,
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// Upload `binary` as `asset_name` to the release for `tag`.
    ///
    /// Idempotent: if an asset with the same name already exists on the
    /// release it is deleted first (delete-then-replace). Works against a
    /// draft or an already-published release — refuses only when the
    /// release is immutable (published assets frozen).
    pub async fn upload_release_asset(
        &self,
        tag: &str,
        binary: &Path,
        asset_name: &str,
    ) -> Result<(), Error> {
        // A non-blocking stat (vs. `Path::exists`, which would block the
        // async executor's thread) so a missing or inaccessible file fails
        // fast before the release lookup and, more importantly, before the
        // delete-then-replace below could remove an existing asset with
        // nothing to replace it. Only existence/access matters here — the
        // length is read fresh, right before the upload, below.
        tokio::fs::metadata(binary)
            .await
            .map_err(|e| binary_access_error(binary, &e))?;

        let release_ref = self
            .reader
            .find_release_for_tag(tag)
            .await?
            .ok_or_else(|| release_not_found_error(tag))?;

        // Assets are frozen at publication, so neither the upload nor the
        // delete-then-replace below can succeed. Refusing here names the
        // cause before anything is attempted, rather than translating the
        // API's rejection after the fact.
        if release_ref.immutable {
            return Err(Error::ImmutableRelease(
                tag.to_string(),
                "the release is published with immutable assets".to_string(),
            ));
        }

        // Delete-then-replace: if an asset of the same name already exists
        // on the release, GitHub rejects a fresh upload with HTTP 422.
        if let Some(asset_id) = self
            .reader
            .find_asset_in_release(release_ref.id, asset_name)
            .await?
        {
            log::info!("Replacing existing asset '{asset_name}' (id={asset_id})");
            // octocrab's `ReleasesHandler` has no dedicated delete-asset
            // method, so this goes through the generic route — see
            // jerus-org/pcu#1070.
            let delete_route = format!(
                "repos/{}/{}/releases/assets/{asset_id}",
                self.owner, self.repo
            );
            if let Err(e) = self
                .reader
                .octocrab()
                .await?
                .delete::<(), _, ()>(delete_route, None)
                .await
            {
                // A concurrent or previous partial run may have already
                // deleted this asset between the listing above and this
                // call. Re-check the actual end state: if the asset is
                // genuinely gone, the desired outcome — no conflicting
                // asset — already holds, and this isn't a real failure.
                if self
                    .reader
                    .find_asset_in_release(release_ref.id, asset_name)
                    .await?
                    .is_some()
                {
                    return Err(map_asset_upload_error(tag, &describe_api_error(&e)));
                }
                log::info!("Asset '{asset_name}' was already gone; treating delete as a no-op");
            }
        }

        // octocrab's `upload_asset` takes fully-buffered `Bytes` (it has no
        // streamed-file variant, unlike the octocrate client this replaced —
        // see jerus-org/pcu#1070), so the file is read in full here rather
        // than opened and streamed. Read fresh right before the upload
        // (rather than reusing the existence check's `metadata` above) to
        // stay immune to a TOCTOU: several await points (release lookup,
        // asset lookup, delete-asset round-trip) separate that earlier stat
        // from this read.
        let content = tokio::fs::read(binary).await.map_err(|e| {
            Error::ReleaseAsset(format!(
                "failed to read asset file '{}': {e}",
                binary.display()
            ))
        })?;

        let octocrab = self.reader.octocrab().await?;
        send_asset(
            octocrab,
            &self.owner,
            &self.repo,
            release_ref.id as u64,
            asset_name,
            content,
        )
        .await
        .map_err(|e| map_asset_upload_error(tag, &e))?;

        log::info!("Successfully uploaded {asset_name}");
        Ok(())
    }

    /// Un-draft the release for `tag`, headless.
    ///
    /// Errors if no release exists for `tag` — unlike [`Self::publish_release_by_id`]
    /// (always called with a known id from the release pipeline), this
    /// tag-based entry point must look the release up first, so "no such
    /// release" is an error rather than a silent no-op.
    ///
    /// Uses `make_latest: legacy` (GitHub's own creation-date/semver
    /// heuristic), not an unconditional `true` — this entry point is public
    /// and tag-based, so a caller could publish an **older** release (e.g.
    /// a backport), where forcing `true` would wrongly demote the actual
    /// newest release. See [`Self::publish_release_by_id`] for the
    /// force-`true` counterpart.
    pub async fn publish_release(&self, tag: &str) -> Result<(), Error> {
        let release_ref = self
            .reader
            .find_release_for_tag(tag)
            .await?
            .ok_or_else(|| release_not_found_error(tag))?;

        self.publish_release_ref(release_ref.id, MakeLatest::Legacy)
            .await
            .map_err(|e| {
                Error::ReleaseAsset(format!("failed to publish release for tag '{tag}': {e}"))
            })
    }

    /// Un-draft `release_id`, forcing `make_latest: true` unconditionally.
    ///
    /// Exposed (like [`ReleaseAssetClient::find_release_for_tag`] and
    /// [`ReleaseAssetClient::find_asset_in_release`]) because `pcu::Client`
    /// needs it: its release pipeline calls this immediately after creating
    /// the release it already has the id for, where forcing `true` is
    /// always correct — unlike the public, tag-based [`Self::publish_release`]
    /// (see its own doc comment), which a caller could point at an older
    /// release. Consolidates what was previously a separate copy of this
    /// same PATCH in `pcu::Client::publish_release` — see jerus-org/pcu#1061.
    pub async fn publish_release_by_id(&self, release_id: i64) -> Result<(), Error> {
        self.publish_release_ref(release_id, MakeLatest::True)
            .await
            .map_err(|e| {
                Error::ReleaseAsset(format!("failed to publish release {release_id}: {e}"))
            })
    }

    async fn publish_release_ref(
        &self,
        release_id: i64,
        make_latest: MakeLatest,
    ) -> Result<(), Error> {
        self.reader
            .octocrab()
            .await?
            .repos(&self.owner, &self.repo)
            .releases()
            .update(release_id as u64)
            .draft(false)
            .make_latest(make_latest)
            .send()
            .await?;

        Ok(())
    }
}

/// POST `content` as `asset_name` to release `release_id`, returning what
/// GitHub said on failure.
///
/// This builds the upload request itself rather than using octocrab's
/// `upload_asset`, for two reasons:
///
/// - A GitHub App installation client (how `pcu::Client` authenticates in
///   CI) must carry its token on the upload. `Octocrab::execute` only
///   attaches an installation token to requests for `api.github.com`, and
///   the upload goes to the release's `upload_url` on `uploads.github.com`,
///   so octocrab would send it with no credentials and GitHub would reject
///   it. The installation token is set on the request here. A token client
///   needs nothing extra: octocrab's auth layer already sends its token to
///   the upload host.
/// - `upload_asset` puts the asset name into the query string unencoded,
///   so a name with a space or `&` breaks the URL.
///
/// The body is sent as `application/octet-stream`: the `.sig` vs binary
/// distinction the octocrate client made (`.sig` as `text/plain`) is not
/// kept. Nothing reads that header to decide validity (`cosign verify-blob`
/// doesn't consult it); it only affected how a browser would render the
/// asset if opened directly. See jerus-org/pcu#1070.
async fn send_asset(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
    release_id: u64,
    asset_name: &str,
    content: Vec<u8>,
) -> Result<(), String> {
    let installation_token = match octocrab.installation_token().await {
        Ok(token) => Some(token),
        Err(octocrab::Error::InstallationTokenInvalidAuth { .. }) => None,
        Err(e) => return Err(describe_api_error(&e)),
    };

    // The upload URL comes from the release itself, as octocrab's own
    // `upload_asset` does, rather than being assembled from a fixed host.
    let release = octocrab
        .repos(owner, repo)
        .releases()
        .get(release_id)
        .await
        .map_err(|e| describe_api_error(&e))?;
    let mut url = reqwest::Url::parse(&release.upload_url.replace("{?name,label}", ""))
        .map_err(|e| format!("invalid upload URL '{}': {e}", release.upload_url))?;
    url.query_pairs_mut().append_pair("name", asset_name);

    let mut request = http::Request::builder()
        .method(http::Method::POST)
        .uri(url.as_str());
    if let Some(token) = installation_token {
        let mut authorization =
            http::HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))
                .map_err(|e| format!("invalid installation token header: {e}"))?;
        authorization.set_sensitive(true);
        request = request.header(http::header::AUTHORIZATION, authorization);
    }
    let request = request
        .header(http::header::CONTENT_TYPE, "application/octet-stream")
        .header(http::header::CONTENT_LENGTH, content.len())
        .body(content)
        .map_err(|e| format!("failed to build upload request: {e}"))?;

    let response = octocrab
        .execute(request)
        .await
        .map_err(|e| describe_api_error(&e))?;
    octocrab::map_github_error(response)
        .await
        .map_err(|e| describe_api_error(&e))?;
    Ok(())
}

fn binary_not_found_error(binary: &Path) -> Error {
    Error::ReleaseAsset(format!("Asset file not found: {}", binary.display()))
}

/// Translate a failed stat of the asset file into an [`Error`], keeping
/// "not found" specific to that cause rather than reporting every
/// inaccessible-file case (e.g. a permission error) the same way.
fn binary_access_error(binary: &Path, source: &std::io::Error) -> Error {
    if source.kind() == std::io::ErrorKind::NotFound {
        return binary_not_found_error(binary);
    }
    Error::ReleaseAsset(format!(
        "cannot access asset file '{}': {source}",
        binary.display()
    ))
}

/// What GitHub actually said, for an error message. octocrab's `Display`
/// for an API rejection is just "GitHub": the status, message, field
/// errors and documentation link all live in the error's source, so
/// `to_string()` drops exactly the part that explains the failure. Other
/// errors (transport, parsing) are rendered with their full source chain.
fn describe_api_error(e: &octocrab::Error) -> String {
    if let octocrab::Error::GitHub { source, .. } = e {
        let mut described = format!("{} {}", source.status_code, source.message);
        if let Some(errors) = source.errors.as_ref().filter(|errors| !errors.is_empty()) {
            let details: Vec<String> = errors.iter().map(ToString::to_string).collect();
            described.push_str(&format!(" [{}]", details.join(", ")));
        }
        if let Some(url) = &source.documentation_url {
            described.push_str(&format!(" (see {url})"));
        }
        return described;
    }
    let mut described = e.to_string();
    let mut cause = std::error::Error::source(e);
    while let Some(c) = cause {
        described.push_str(&format!(": {c}"));
        cause = c.source();
    }
    described
}

/// Translate a GitHub API error message into a typed [`Error`], recognising
/// the immutable-release rejection so callers get an actionable message
/// instead of a raw API string.
fn map_asset_upload_error(tag: &str, api_message: &str) -> Error {
    if api_message.to_lowercase().contains("immutable release") {
        return Error::ImmutableRelease(tag.to_string(), api_message.to_string());
    }
    Error::ReleaseAsset(api_message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises `upload_release_asset` end-to-end (not just its pure
    /// helpers): a missing source file must fail fast, before any network
    /// call — mirrors `pcu::Client`'s own
    /// `upload_release_asset_returns_error_for_missing_file` test.
    #[tokio::test]
    async fn upload_release_asset_returns_error_for_missing_file() {
        let writer = ReleaseAssetWriter::new("test-org", "test-repo", "token");
        let result = writer
            .upload_release_asset("v1.0.0", Path::new("/nonexistent/binary"), "binary")
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Asset file not found"), "unexpected: {msg}");
    }

    /// jerus-org/pcu#1085: mirrors jci-audit's `publish_record.rs`, which
    /// builds a `ReleaseAssetWriter` in a field-init statement before
    /// building the runtime it will later `block_on` with. Deliberately
    /// plain `#[test]`, not `#[tokio::test]` — no runtime exists at all.
    #[test]
    fn new_does_not_require_a_tokio_runtime() {
        let _writer = ReleaseAssetWriter::new("test-org", "test-repo", "token");
    }

    #[test]
    fn release_asset_writer_builds_without_git_checkout() {
        let writer = ReleaseAssetWriter::new("test-org", "test-repo", "token");
        assert_eq!(writer.owner(), "test-org");
        assert_eq!(writer.repo(), "test-repo");
    }

    /// `from_shared` must reuse the given `Arc`s rather than building a new
    /// `Octocrab` for the same token. `ReleaseAssetWriter` keeps no field of
    /// its own for it (jerus-org/pcu#1085 removed that duplicate copy) —
    /// every REST call goes through the composed `reader: ReleaseAssetClient`
    /// — so the count rises by exactly 1: the caller's own explicit clone
    /// passed in, then the reader's `OnceCell`-wrapped copy: 1 -> 2. If
    /// `from_shared` ever started constructing a fresh `Octocrab` instead of
    /// wrapping the given `Arc`, this given `Arc`'s count would stay at 1.
    #[tokio::test]
    async fn writer_from_shared_reuses_the_given_clients() {
        let github_rest = Arc::new(
            Octocrab::builder()
                .personal_token("token".to_string())
                .build()
                .unwrap(),
        );
        let github_graphql = Arc::new(gql_client::Client::new_with_headers(
            "https://api.github.com/graphql",
            std::collections::HashMap::<&str, &str>::new(),
        ));

        assert_eq!(Arc::strong_count(&github_rest), 1);

        let _writer = ReleaseAssetWriter::from_shared(
            "test-org",
            "test-repo",
            "token",
            Arc::clone(&github_rest),
            Arc::clone(&github_graphql),
        );

        assert_eq!(
            Arc::strong_count(&github_rest),
            2,
            "from_shared should hold the same Octocrab instance (via the composed reader), \
             not build a new one"
        );
    }

    // release_not_found_error is now shared with client.rs, which already
    // covers its message-formatting behaviour in its own test module.

    #[test]
    fn binary_not_found_error_names_the_path() {
        let msg = binary_not_found_error(Path::new("/tmp/does-not-exist.tar.gz")).to_string();
        assert!(
            msg.contains("/tmp/does-not-exist.tar.gz"),
            "unexpected: {msg}"
        );
    }

    /// A permission error must not be reported as "not found" — the two
    /// causes call for different fixes (the file exists but access is
    /// denied, vs. the path is simply wrong). Denying access via the
    /// *parent directory* (rather than the file itself) reliably fails the
    /// `metadata` stat call: a file's own permission bits don't gate
    /// `stat`, only `open`/`read` — only the containing directory's
    /// execute/search bit does.
    #[cfg(unix)]
    #[tokio::test]
    async fn upload_release_asset_distinguishes_permission_errors_from_not_found() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!(
            "pcu-release-assets-test-perm-dir-{}",
            std::process::id()
        ));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("binary");
        tokio::fs::write(&path, b"data").await.unwrap();
        tokio::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000))
            .await
            .unwrap();

        if tokio::fs::metadata(&path).await.is_ok() {
            // Running as root (or similar) — directory permission bits
            // don't block access here, so this environment can't exercise
            // the distinction under test.
            let _ = tokio::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).await;
            let _ = tokio::fs::remove_dir_all(&dir).await;
            return;
        }

        let writer = ReleaseAssetWriter::new("test-org", "test-repo", "token");
        let result = writer.upload_release_asset("v1.0.0", &path, "binary").await;

        let _ = tokio::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).await;
        let _ = tokio::fs::remove_dir_all(&dir).await;

        let msg = result.unwrap_err().to_string();
        assert!(
            !msg.contains("Asset file not found"),
            "a permission error must not be misreported as not-found: {msg}"
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
        assert!(matches!(err, Error::ImmutableRelease(_, _)));
    }

    #[test]
    fn map_asset_upload_error_is_case_insensitive() {
        let err = map_asset_upload_error(
            "pcu-v0.6.29",
            "cannot upload assets to an IMMUTABLE release",
        );
        assert!(matches!(err, Error::ImmutableRelease(_, _)));
    }

    /// A real octocrab API error: GitHub's error body served by a mock
    /// server, since `GitHubError` is non-exhaustive and can't be built
    /// directly.
    async fn github_api_error(status: u16, body: serde_json::Value) -> octocrab::Error {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(status).set_body_json(body))
            .mount(&server)
            .await;
        let octocrab = Octocrab::builder()
            .base_uri(server.uri())
            .unwrap()
            .personal_token("token".to_string())
            .build()
            .unwrap();
        octocrab
            .get::<serde_json::Value, _, ()>("/repos/test-org/test-repo/releases/1", None)
            .await
            .expect_err("the mock rejects every request")
    }

    /// octocrab renders an API rejection as just "GitHub"; the description
    /// must carry the status, message and field errors instead, so an
    /// upload failure says why.
    #[tokio::test]
    async fn describe_api_error_keeps_githubs_status_message_and_errors() {
        let e = github_api_error(
            422,
            serde_json::json!({
                "message": "Validation Failed",
                "errors": [{"resource": "ReleaseAsset", "code": "already_exists", "field": "name"}],
                "documentation_url": "https://docs.github.com/rest"
            }),
        )
        .await;
        assert_eq!(
            e.to_string(),
            "GitHub",
            "precondition: octocrab's Display hides the detail"
        );
        let described = describe_api_error(&e);
        assert!(described.contains("422"), "{described}");
        assert!(described.contains("Validation Failed"), "{described}");
        assert!(described.contains("already_exists"), "{described}");
        assert!(
            described.contains("https://docs.github.com/rest"),
            "{described}"
        );
    }

    /// The immutable-release translation matches on GitHub's message, so it
    /// can only fire once that message survives.
    #[tokio::test]
    async fn describe_api_error_lets_an_immutable_rejection_be_recognised() {
        let e = github_api_error(
            422,
            serde_json::json!({"message": "Cannot upload assets to an immutable release."}),
        )
        .await;
        let err = map_asset_upload_error("pcu-v0.6.38", &describe_api_error(&e));
        assert!(matches!(err, Error::ImmutableRelease(_, _)), "{err}");
    }

    #[test]
    fn map_asset_upload_error_falls_back_to_release_asset_for_other_failures() {
        let err = map_asset_upload_error("pcu-v0.6.29", "422 Validation Failed");
        assert!(matches!(err, Error::ReleaseAsset(_)));
    }

    /// A writer whose `github_rest` points at a mock server, for asserting
    /// on the actual outgoing request rather than an octocrate-era
    /// inspectable `Request` struct — octocrab's `UpdateReleaseBuilder` is a
    /// fluent builder with no such struct to unit-test in isolation.
    async fn writer_against(server: &wiremock::MockServer) -> ReleaseAssetWriter {
        let github_rest = Arc::new(
            Octocrab::builder()
                .base_uri(server.uri())
                .unwrap()
                .personal_token("token".to_string())
                .build()
                .unwrap(),
        );
        let github_graphql = Arc::new(gql_client::Client::new_with_headers(
            "https://api.github.com/graphql",
            std::collections::HashMap::<&str, &str>::new(),
        ));
        ReleaseAssetWriter::from_shared(
            "test-org",
            "test-repo",
            "token",
            github_rest,
            github_graphql,
        )
    }

    /// `pcu::Client`'s release pipeline calls the by-id publish path
    /// immediately after creating the release it already has the id for —
    /// forcing `true` is always correct there (jerus-org/pcu#1061).
    #[tokio::test]
    async fn publish_release_by_id_forces_make_latest_true() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PATCH"))
            .and(wiremock::matchers::path(
                "/repos/test-org/test-repo/releases/42",
            ))
            .and(wiremock::matchers::body_partial_json(
                serde_json::json!({"draft": false, "make_latest": "true"}),
            ))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42, "tag_name": "pcu-v1.0.0", "draft": false, "prerelease": false,
                "assets": [], "target_commitish": "main", "name": null, "body": null,
                "created_at": null, "published_at": null, "author": null,
                "url": "https://api.github.com/repos/test-org/test-repo/releases/42",
                "html_url": "https://github.com/test-org/test-repo/releases/tag/pcu-v1.0.0",
                "assets_url": "https://api.github.com/repos/test-org/test-repo/releases/42/assets",
                "upload_url": "https://uploads.github.com/repos/test-org/test-repo/releases/42/assets",
                "tarball_url": null, "zipball_url": null, "node_id": "R_1"
            })))
            .mount(&server)
            .await;

        let writer = writer_against(&server).await;
        writer.publish_release_by_id(42).await.unwrap();
    }

    /// `publish_release`'s tag-based path passes `MakeLatest::Legacy` (not
    /// `True`) to `publish_release_ref` — a caller could target an older
    /// release (e.g. a backport), where forcing `true` would wrongly demote
    /// the actual newest release. Exercised here directly against
    /// `publish_release_ref`, since going through the public `publish_release`
    /// would also require mocking the GraphQL release lookup it does first.
    #[tokio::test]
    async fn publish_release_ref_sends_make_latest_legacy() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("PATCH"))
            .and(wiremock::matchers::path(
                "/repos/test-org/test-repo/releases/42",
            ))
            .and(wiremock::matchers::body_partial_json(
                serde_json::json!({"draft": false, "make_latest": "legacy"}),
            ))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 42, "tag_name": "pcu-v1.0.0", "draft": false, "prerelease": false,
                "assets": [], "target_commitish": "main", "name": null, "body": null,
                "created_at": null, "published_at": null, "author": null,
                "url": "https://api.github.com/repos/test-org/test-repo/releases/42",
                "html_url": "https://github.com/test-org/test-repo/releases/tag/pcu-v1.0.0",
                "assets_url": "https://api.github.com/repos/test-org/test-repo/releases/42/assets",
                "upload_url": "https://uploads.github.com/repos/test-org/test-repo/releases/42/assets",
                "tarball_url": null, "zipball_url": null, "node_id": "R_1"
            })))
            .mount(&server)
            .await;

        let writer = writer_against(&server).await;
        writer
            .publish_release_ref(42, MakeLatest::Legacy)
            .await
            .unwrap();
    }

    /// A GitHub release body whose `upload_url` points back at `server`.
    fn release_json(server: &wiremock::MockServer) -> serde_json::Value {
        serde_json::json!({
            "id": 42, "tag_name": "pcu-v1.0.0", "draft": true, "prerelease": false,
            "assets": [], "target_commitish": "main", "name": null, "body": null,
            "created_at": null, "published_at": null, "author": null,
            "url": format!("{}/repos/test-org/test-repo/releases/42", server.uri()),
            "html_url": "https://github.com/test-org/test-repo/releases/tag/pcu-v1.0.0",
            "assets_url": format!("{}/repos/test-org/test-repo/releases/42/assets", server.uri()),
            "upload_url": format!(
                "{}/upload/repos/test-org/test-repo/releases/42/assets{{?name,label}}",
                server.uri()
            ),
            "tarball_url": null, "zipball_url": null, "node_id": "R_1"
        })
    }

    /// Mount the upload endpoint, answering only a request that carries
    /// `authorization`; anything else gets wiremock's default 404.
    async fn mount_upload(server: &wiremock::MockServer, authorization: &str) {
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/upload/repos/test-org/test-repo/releases/42/assets",
            ))
            .and(wiremock::matchers::query_param("name", "bin v1.tar.gz"))
            .and(wiremock::matchers::header("authorization", authorization))
            .and(wiremock::matchers::header(
                "content-type",
                "application/octet-stream",
            ))
            .and(wiremock::matchers::body_bytes(b"payload".to_vec()))
            .respond_with(
                wiremock::ResponseTemplate::new(201)
                    .set_body_json(serde_json::json!({"id": 7, "name": "bin v1.tar.gz"})),
            )
            .expect(1)
            .mount(server)
            .await;
    }

    /// A throwaway GitHub App key: generated per run, so no private key is
    /// kept in the repository.
    fn app_key() -> jsonwebtoken::EncodingKey {
        use aws_lc_rs::encoding::AsDer;
        use base64::Engine;

        let key = aws_lc_rs::rsa::KeyPair::generate(aws_lc_rs::rsa::KeySize::Rsa2048).unwrap();
        let der: aws_lc_rs::encoding::Pkcs8V1Der = key.as_der().unwrap();
        let pem = format!(
            "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
            base64::engine::general_purpose::STANDARD.encode(der.as_ref())
        );
        jsonwebtoken::EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap()
    }

    /// A GitHub App installation client — how `pcu::Client` authenticates in
    /// CI — must send its installation token with the upload. octocrab's
    /// own `upload_asset` leaves it off (it only authenticates requests to
    /// `api.github.com`, and uploads go to `uploads.github.com`), which is
    /// what failed the gen-circleci-orb 0.2.0 release with a bare "GitHub".
    #[tokio::test]
    async fn send_asset_authenticates_installation_upload() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/app/installations/99/access_tokens",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(201).set_body_json(serde_json::json!({
                    "token": "ghs_installation",
                    "expires_at": "2099-01-01T00:00:00Z",
                    "permissions": {}
                })),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(
                "/repos/test-org/test-repo/releases/42",
            ))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(release_json(&server)))
            .mount(&server)
            .await;
        mount_upload(&server, "Bearer ghs_installation").await;

        let app = Octocrab::builder()
            .base_uri(server.uri())
            .unwrap()
            .app(octocrab::models::AppId(1), app_key())
            .build()
            .unwrap();
        let (installation, _token) = app
            .installation_and_token(octocrab::models::InstallationId(99))
            .await
            .unwrap();

        send_asset(
            &installation,
            "test-org",
            "test-repo",
            42,
            "bin v1.tar.gz",
            b"payload".to_vec(),
        )
        .await
        .unwrap();
    }

    /// A token client's upload is authenticated by octocrab's auth layer,
    /// which sends the token to the configured upload host.
    #[tokio::test]
    async fn send_asset_authenticates_token_upload() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(
                "/repos/test-org/test-repo/releases/42",
            ))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(release_json(&server)))
            .mount(&server)
            .await;
        mount_upload(&server, "Bearer token").await;

        let octocrab = Octocrab::builder()
            .base_uri(server.uri())
            .unwrap()
            .upload_uri(server.uri())
            .unwrap()
            .personal_token("token".to_string())
            .build()
            .unwrap();

        send_asset(
            &octocrab,
            "test-org",
            "test-repo",
            42,
            "bin v1.tar.gz",
            b"payload".to_vec(),
        )
        .await
        .unwrap();
    }

    /// A rejected upload reports GitHub's status and message, not octocrab's
    /// bare "GitHub".
    #[tokio::test]
    async fn send_asset_reports_github_rejection() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/app/installations/99/access_tokens",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(201).set_body_json(serde_json::json!({
                    "token": "ghs_installation",
                    "expires_at": "2099-01-01T00:00:00Z",
                    "permissions": {}
                })),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(
                "/repos/test-org/test-repo/releases/42",
            ))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(release_json(&server)))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path(
                "/upload/repos/test-org/test-repo/releases/42/assets",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(422)
                    .set_body_json(serde_json::json!({"message": "Validation Failed"})),
            )
            .mount(&server)
            .await;

        let app = Octocrab::builder()
            .base_uri(server.uri())
            .unwrap()
            .app(octocrab::models::AppId(1), app_key())
            .build()
            .unwrap();
        let (installation, _token) = app
            .installation_and_token(octocrab::models::InstallationId(99))
            .await
            .unwrap();

        let err = send_asset(
            &installation,
            "test-org",
            "test-repo",
            42,
            "bin v1.tar.gz",
            b"payload".to_vec(),
        )
        .await
        .unwrap_err();
        assert!(err.contains("422"), "status missing: {err}");
        assert!(err.contains("Validation Failed"), "message missing: {err}");
    }
}
