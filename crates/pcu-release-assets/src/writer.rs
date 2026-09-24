use std::{path::Path, sync::Arc};

use octocrab::{repos::releases::MakeLatest, Octocrab};

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
                    return Err(map_asset_upload_error(tag, &e.to_string()));
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

        // octocrab's `UploadAssetBuilder` always sends
        // `Content-Type: application/octet-stream` with no override hook, so
        // the `.sig`-vs-binary distinction the octocrate client made (`.sig`
        // as `text/plain`) is lost here. Accepted: nothing reads that header
        // to decide validity (`cosign verify-blob` doesn't consult it) — it
        // only affected how a browser would render the asset if opened
        // directly. See jerus-org/pcu#1070.
        self.reader
            .octocrab()
            .await?
            .repos(&self.owner, &self.repo)
            .releases()
            .upload_asset(release_ref.id as u64, asset_name, content.into())
            .send()
            .await
            .map_err(|e| map_asset_upload_error(tag, &e.to_string()))?;

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
}
