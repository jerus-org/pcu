use std::{path::Path, sync::Arc};

use octocrate::{APIConfig, GitHubAPI, PersonalAccessToken};

use crate::{
    client::{build_authenticated_clients, release_not_found_error, ReleaseAssetClient},
    Error,
};

const UPLOADS_END_POINT: &str = "https://uploads.github.com";

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
    github_rest: Arc<GitHubAPI>,
    /// A second REST client pointed at `uploads.github.com` — binary uploads
    /// must go there, not `api.github.com`. Built once at construction and
    /// reused across calls, rather than per-upload, since a single writer is
    /// commonly used to attach several assets (binary, `.sig`, attestation
    /// bundle) to the same release.
    upload_rest: Arc<GitHubAPI>,
    reader: ReleaseAssetClient,
}

impl ReleaseAssetWriter {
    /// Construct a writer for `owner`/`repo`, authenticating with
    /// `github_token`. Does not touch the filesystem or git in any way.
    pub fn new(
        owner: impl Into<String>,
        repo: impl Into<String>,
        github_token: impl Into<String>,
    ) -> Self {
        let github_token = github_token.into();
        let (github_rest, github_graphql) = build_authenticated_clients(&github_token);

        Self::from_shared(
            owner,
            repo,
            github_token,
            Arc::new(github_rest),
            Arc::new(github_graphql),
        )
    }

    /// Construct a writer for `owner`/`repo` from an already-authenticated
    /// `github_rest`/`github_graphql` pair — e.g. the ones `pcu::Client`
    /// already built. The `Arc`s are cloned (refcount only), not rebuilt, and
    /// the same clones back an internal [`ReleaseAssetClient`] for shared
    /// read operations — no second, redundant auth object for the same
    /// token. `github_token` is only needed transiently here, to build the
    /// `uploads.github.com`-pointed client — it is not retained afterward.
    pub fn from_shared(
        owner: impl Into<String>,
        repo: impl Into<String>,
        github_token: impl Into<String>,
        github_rest: Arc<GitHubAPI>,
        github_graphql: Arc<gql_client::Client>,
    ) -> Self {
        let owner = owner.into();
        let repo = repo.into();
        let github_token = github_token.into();

        let reader = ReleaseAssetClient::from_shared(
            owner.clone(),
            repo.clone(),
            github_token.clone(),
            Arc::clone(&github_rest),
            github_graphql,
        );

        let upload_token = PersonalAccessToken::new(github_token);
        let upload_config = APIConfig::new(UPLOADS_END_POINT, upload_token);
        let upload_rest = Arc::new(GitHubAPI::new(&upload_config));

        Self {
            owner,
            repo,
            github_rest,
            upload_rest,
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
        // nothing to replace it. The `Metadata` is kept for `content_length`
        // below rather than stat-ing the file a second time after opening.
        let metadata = tokio::fs::metadata(binary)
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
        // `self.github_rest` already points at api.github.com, so it is
        // reused directly here rather than building a second client for the
        // same endpoint.
        if let Some(asset_id) = self
            .reader
            .find_asset_in_release(release_ref.id, asset_name)
            .await?
        {
            log::info!("Replacing existing asset '{asset_name}' (id={asset_id})");
            self.github_rest
                .repos
                .delete_release_asset(&self.owner, &self.repo, asset_id)
                .send()
                .await
                .map_err(|e| map_asset_upload_error(tag, &e.to_string()))?;
        }

        let file = tokio::fs::File::open(binary).await.map_err(|e| {
            Error::ReleaseAsset(format!(
                "failed to open asset file '{}': {e}",
                binary.display()
            ))
        })?;
        let content_length = metadata.len();

        let content_type = if asset_name.ends_with(".sig") {
            "text/plain"
        } else {
            "application/octet-stream"
        };

        let query = octocrate::repos::upload_release_asset::Query::builder()
            .name(asset_name)
            .build();

        self.upload_rest
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

    /// Un-draft the release for `tag`, headless.
    ///
    /// Errors if no release exists for `tag` — unlike `pcu::Client`'s
    /// internal, release-id-based `publish_release` (always called with a
    /// known id from the release pipeline), this tag-based entry point must
    /// look the release up first, so "no such release" is an error rather
    /// than a silent no-op.
    ///
    /// Uses `make_latest: legacy` (GitHub's own creation-date/semver
    /// heuristic), not an unconditional `true` — unlike `pcu::Client`'s
    /// internal, release-id-based `publish_release`, which is only ever
    /// called on the release the pipeline just created (so forcing `true`
    /// is always correct there). This entry point is public and tag-based;
    /// a caller could publish an **older** release (e.g. a backport), where
    /// forcing `true` would wrongly demote the actual newest release.
    pub async fn publish_release(&self, tag: &str) -> Result<(), Error> {
        let release_ref = self
            .reader
            .find_release_for_tag(tag)
            .await?
            .ok_or_else(|| release_not_found_error(tag))?;

        let request = octocrate::repos::update_release::Request {
            body: None,
            discussion_category_name: None,
            draft: Some(false),
            make_latest: Some(octocrate::repos::update_release::RequestMakeLatest::Legacy),
            name: None,
            prerelease: None,
            tag_name: None,
            target_commitish: None,
        };

        self.github_rest
            .repos
            .update_release(&self.owner, &self.repo, release_ref.id)
            .body(&request)
            .send()
            .await
            .map_err(|e| {
                Error::ReleaseAsset(format!("failed to publish release for tag '{tag}': {e}"))
            })?;

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

    #[test]
    fn release_asset_writer_builds_without_git_checkout() {
        let writer = ReleaseAssetWriter::new("test-org", "test-repo", "token");
        assert_eq!(writer.owner(), "test-org");
        assert_eq!(writer.repo(), "test-repo");
    }

    /// `from_shared` must reuse the given `Arc`s rather than building a new
    /// `GitHubAPI` for the same token. `ReleaseAssetWriter` holds the `Arc`
    /// twice — once in its own field (for write calls) and once inside the
    /// composed `reader: ReleaseAssetClient` (for read calls) — so the
    /// count rises by exactly 2 (the caller's own explicit clone passed in,
    /// then the writer's field, then the reader's field: 1 -> 2 -> 3). If
    /// `from_shared` ever started constructing a fresh `GitHubAPI` instead
    /// of cloning the given `Arc`, this given `Arc`'s count would stay at 2
    /// (bumped only by the caller's own clone) instead of reaching 3.
    #[test]
    fn writer_from_shared_reuses_the_given_clients() {
        let dummy_pat = PersonalAccessToken::new("token");
        let dummy_config = APIConfig::with_token(dummy_pat).shared();
        let github_rest = Arc::new(GitHubAPI::new(&dummy_config));
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
            3,
            "from_shared should hold the same GitHubAPI instance (in both its own field \
             and the composed reader's field), not build a new one"
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
}
