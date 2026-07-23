use crate::utilities::linkedin_post::{build_release_text, compute_release_url};
use gen_linkedin::posts::{PostsClient, TextPost};
use gen_linkedin::{auth::StaticTokenProvider, client::Client as LiClient};

async fn share_release_to_linkedin(prefix: &str, version: &str) -> Result<(), Error> {
    let settings = super::Commands::Release(Release {
        update_prlog: false,
        prefix: prefix.to_string(),
        linkedin_share: true,
        skip_ci: false,
        no_skip_ci: false,
        mode: Mode::Version(crate::cli::release::mode::Version {
            version: version.to_string(),
        }),
    })
    .get_settings()?;

    let token = settings
        .get::<String>("linkedin_access_token")
        .or_else(|_| {
            std::env::var("LINKEDIN_ACCESS_TOKEN")
                .map_err(|_| config::ConfigError::NotFound("linkedin_access_token".into()))
        })?;
    let author_urn = settings.get::<String>("linkedin_author_urn").or_else(|_| {
        std::env::var("LINKEDIN_AUTHOR_URN")
            .map_err(|_| config::ConfigError::NotFound("linkedin_author_urn".into()))
    })?;

    let text = build_release_text(&settings, prefix, version)?;
    let link = compute_release_url(&settings, prefix, Some(version))?;

    let li = LiClient::new(StaticTokenProvider(token))?;
    let pc = PostsClient::new(li);
    let mut post = TextPost::new(author_urn, text);
    if let Some(u) = link {
        post = post.with_link(u);
    }
    let _ = pc.create_text_post(&post).await?;
    Ok(())
}

use std::{fs, io::Write, path::Path, process::Command};

use super::{CIExit, Commands};
use crate::{Client, Error, GitOps, MakeRelease, SignConfig, Workspace};
mod mode;

use clap::Parser;
use mode::Mode;
use octocrate::{APIConfig, GitHubAPI, PersonalAccessToken};
use owo_colors::{OwoColorize, Style};

/// Poll `probe` up to `max_attempts` times, sleeping `retry_delay` between
/// attempts, returning `true` as soon as it yields `true`.
///
/// GitHub's REST API lags behind a freshly-pushed git tag: a poll taken
/// immediately after the push can report the tag as absent for a few seconds.
/// Returning `false` only after exhausting every attempt (rather than on the
/// first poll) keeps release creation from being silently skipped during that
/// eventual-consistency window — mirroring the retry already used when looking
/// up the release for asset upload.
async fn tag_visible_with_retry<F, Fut>(
    tag: &str,
    max_attempts: u32,
    retry_delay: std::time::Duration,
    mut probe: F,
) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for attempt in 1..=max_attempts {
        if probe().await {
            return true;
        }
        log::warn!("tag '{tag}' not yet visible via API (attempt {attempt}/{max_attempts})");
        if attempt < max_attempts {
            tokio::time::sleep(retry_delay).await;
        }
    }
    false
}

/// Outcome of [`ensure_release_for_tag`], made explicit so "did nothing" can
/// never masquerade as success.
#[derive(Debug, PartialEq, Eq)]
enum EnsureOutcome {
    /// A GitHub release was created for the tag.
    Created,
    /// A GitHub release already existed for the tag.
    AlreadyPresent,
    /// The git tag was not present, so there was nothing to release.
    TagAbsent,
}

/// Ensure a GitHub release exists for `tag`, using injectable effects.
///
/// Never reports success without having ensured the release: an undetermined
/// release-existence check (`Err`) and a failed creation both propagate as
/// errors rather than a silent `Ok`. That silent `Ok` is the failure mode that
/// stranded gen-circleci-orb 0.0.53 — the "create release" step went green
/// having created nothing, then asset upload failed past the irreversible
/// pubkey-tag push, leaving a release with no binstall binary.
async fn ensure_release_for_tag<TV, TVF, RE, REF, MK, MKF>(
    tag: &str,
    mut tag_visible: TV,
    mut release_exists: RE,
    mut make_release: MK,
) -> Result<EnsureOutcome, Error>
where
    TV: FnMut() -> TVF,
    TVF: std::future::Future<Output = bool>,
    RE: FnMut() -> REF,
    REF: std::future::Future<Output = Result<bool, Error>>,
    MK: FnMut() -> MKF,
    MKF: std::future::Future<Output = Result<(), Error>>,
{
    if !tag_visible().await {
        // No tag for this package (e.g. a workspace member not released this
        // cycle) — there is genuinely nothing to release. This is the only
        // success-without-a-release outcome, and it is explicit.
        log::info!("No tag '{tag}' present — nothing to release");
        return Ok(EnsureOutcome::TagAbsent);
    }
    // `?` here is the discipline: an undetermined existence check propagates as
    // an error instead of being coerced (as a bare `.is_ok()` did) into a
    // "release exists" that silently skips creation.
    if release_exists().await? {
        log::info!("GitHub release for '{tag}' already exists — skipping creation");
        return Ok(EnsureOutcome::AlreadyPresent);
    }
    log::info!("Tag '{tag}' exists, no GitHub release yet — creating");
    make_release().await?;
    Ok(EnsureOutcome::Created)
}

/// Create a GitHub release for `<prefix><version>` when needed.
///
/// Idempotent: if the git tag is absent, logs an error and returns `Ok`.
/// If the GitHub release already exists, skips creation silently.
/// Only calls `make_release` when the tag is present but no release exists yet.
async fn ensure_github_release(client: &Client, prefix: &str, version: &str) -> Result<(), Error> {
    let tag = format!("{prefix}{version}");
    // Each effect is retried/loud in its own right (tag visibility, release
    // existence, creation); the orchestration guarantees we never report success
    // without an ensured release. See the 0.0.53 stranding in
    // ensure_release_for_tag's docs.
    let outcome = ensure_release_for_tag(
        &tag,
        || {
            tag_visible_with_retry(&tag, 5, std::time::Duration::from_secs(2), || {
                client.tag_exists(&tag)
            })
        },
        || client.github_release_exists(&tag),
        || client.make_release(prefix, version),
    )
    .await?;
    log::debug!("ensure_github_release outcome for '{tag}': {outcome:?}");
    Ok(())
}

/// Resolve a version from an optional CLI argument, falling back to $SEMVER or $NEXT_VERSION.
/// Returns "none" when no version is available.
fn resolve_version(version_opt: &Option<String>) -> String {
    if let Some(v) = version_opt {
        return v.clone();
    }
    std::env::var("SEMVER")
        .or_else(|_| std::env::var("NEXT_VERSION"))
        .unwrap_or_else(|_| "none".to_string())
}

/// Outcome of injecting a signing pubkey into a crate's `Cargo.toml`.
#[derive(Debug, PartialEq, Eq)]
enum PubkeyOutcome {
    /// The `pubkey = "..."` placeholder was found and replaced; carries the new
    /// file content to write.
    Injected(String),
    /// No placeholder present, and the scaffold was not required (binary not
    /// published as a signed release): nothing to write.
    SkippedNoScaffold,
}

/// Whether the crate at this manifest produces a binary target.
///
/// A crate is a binary if its manifest declares a `[[bin]]` target, or cargo
/// would auto-discover one from `src/main.rs` or a `src/bin/` directory. A
/// library-only crate produces no signable binary, so it needs no signing
/// pubkey. Decided independently of the signing scaffold so that a *binary*
/// crate missing its scaffold is caught (jerus-org/pcu#1012) rather than
/// silently mistaken for a library.
fn crate_is_binary(
    cargo_toml: &str,
    main_rs_exists: bool,
    bin_dir_exists: bool,
) -> Result<bool, Error> {
    let manifest: toml::Value = toml::from_str(cargo_toml)?;
    let declares_bin = manifest
        .get("bin")
        .and_then(|b| b.as_array())
        .is_some_and(|targets| !targets.is_empty());
    Ok(declares_bin || main_rs_exists || bin_dir_exists)
}

/// Replace the `pubkey = "..."` placeholder in a crate `Cargo.toml` with the
/// confirmed signing `pubkey`.
///
/// Called only for binary crates. When no placeholder is present:
/// - `require_scaffold` true → `Err(MissingSigningScaffold)` (a published,
///   signed binary must carry a verifiable key);
/// - `require_scaffold` false → `SkippedNoScaffold` (binary not published as a
///   signed release, via `--no-github-release`).
fn inject_pubkey_value(
    content: &str,
    pubkey: &str,
    require_scaffold: bool,
) -> Result<PubkeyOutcome, Error> {
    let replacement = format!(r#"pubkey = "{pubkey}""#);
    // `Regex::replace` returns `Cow::Borrowed` (the input, unchanged) only when
    // there is no match — the reliable signal that the scaffold is absent.
    match regex::Regex::new(r#"pubkey = ".*""#)?.replace(content, replacement.as_str()) {
        std::borrow::Cow::Owned(new_content) => Ok(PubkeyOutcome::Injected(new_content)),
        std::borrow::Cow::Borrowed(_) if require_scaffold => Err(Error::MissingSigningScaffold(
            "binary crate has no `pubkey = \"...\"` line under \
                 `[package.metadata.binstall.signing]` to inject the signing key into; \
                 add the signing scaffold, or pass --no-github-release if this binary \
                 is not published as a signed release"
                .to_string(),
        )),
        std::borrow::Cow::Borrowed(_) => Ok(PubkeyOutcome::SkippedNoScaffold),
    }
}

/// Append `export KEY=VALUE\n` to the file named by $BASH_ENV.
/// Logs a warning if $BASH_ENV is unset (e.g. running locally).
fn write_to_bash_env(key: &str, value: &str) -> Result<(), Error> {
    let bash_env = std::env::var("BASH_ENV").unwrap_or_default();
    if bash_env.is_empty() {
        log::warn!("$BASH_ENV not set — {key}={value} will not persist to subsequent CI steps");
        return Ok(());
    }
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&bash_env)?;
    writeln!(file, "export {key}={value}")?;
    log::debug!("Wrote {key}={value} to $BASH_ENV ({bash_env})");
    Ok(())
}

/// Commit subject for the prlog update that accompanies a release.
///
/// Distinct from the routine post-merge `chore: update prlog for pr`: this
/// commit also carries the version tag, so it must be identifiable as a release
/// on the default branch (and version-stamped for traceability) rather than
/// masquerading as a generic prlog update.
fn release_prlog_commit_message(version: &str) -> String {
    format!("chore: update prlog for release {version}")
}

#[derive(Debug, Parser, Clone)]
pub struct Release {
    /// Update the prlog by renaming the unreleased section with the version
    /// number
    #[arg(short, long, default_value_t = false)]
    pub update_prlog: bool,
    /// Prefix for the version tag
    #[clap(short, long, default_value_t = String::from("v"))]
    pub prefix: String,
    /// Also share this release to LinkedIn using configured credentials
    #[arg(long, default_value_t = false)]
    pub linkedin_share: bool,
    /// Skip CI on the prlog update commit by appending the ci-avoidance marker
    /// (only takes effect on the default branch). This is the DEFAULT for
    /// `release`: the release work itself was validated, so the one-line
    /// post-release prlog commit need not be revalidated. `--skip-ci` states it
    /// explicitly; pass `--no-skip-ci` to opt back into validation. The effective
    /// decision is computed by [`Release::should_skip_ci`].
    #[arg(long, overrides_with = "no_skip_ci", default_value_t = false)]
    pub skip_ci: bool,
    /// Opt back into CI validation on the prlog update commit (release skips by
    /// default). Needed by orb repos whose `v*` tag triggers the publish (the
    /// marker must not reach the tagged commit) and by the validating final
    /// prlog of a multi-crate release. If both flags are given, the last wins.
    #[arg(long = "no-skip-ci", action = clap::ArgAction::SetTrue, overrides_with = "skip_ci")]
    pub no_skip_ci: bool,
    #[command(subcommand)]
    pub mode: Mode,
}

impl Release {
    /// Effective ci-skip decision for the prlog update commit.
    ///
    /// `release` skips by default; `--no-skip-ci` opts back into validation.
    /// The `overrides_with` relationship between the two flags makes the last
    /// one on the command line win, so this need only consult `no_skip_ci`:
    /// when `--skip-ci` is given last it resets `no_skip_ci` to false (skip),
    /// and when `--no-skip-ci` is given last it sets it true (validate).
    pub fn should_skip_ci(&self) -> bool {
        !self.no_skip_ci
    }

    pub async fn run_release(self, sign_config: SignConfig) -> Result<CIExit, Error> {
        let client = Commands::Release(self.clone()).get_client().await?;

        match self.mode {
            Mode::Version(_) => self.release_version(client, sign_config).await,
            Mode::Package(_) => self.release_package(client).await,
            Mode::Workspace => self.release_workspace(client).await,
            Mode::Current(_) => self.release_current(client).await,
            Mode::CheckVersionPublished(_) => self.check_version_published().await,
            Mode::CheckTag(_) => self.check_tag(client).await,
            Mode::InjectPubkey(_) => self.inject_pubkey().await,
            Mode::UploadAsset(_) => self.upload_asset(client).await,
            Mode::Attest(_) => self.attest(client).await,
        }
    }

    async fn release_workspace(&self, client: Client) -> Result<CIExit, Error> {
        //     log::info!("Running release for workspace");
        let path = Path::new("./Cargo.toml");
        let workspace = Workspace::new(path).unwrap();

        let packages = workspace.packages();

        if let Some(packages) = packages {
            for package in packages {
                let prefix = format!("{}-{}", package.name, self.prefix);
                let version = package.version;
                ensure_github_release(&client, &prefix, &version).await?;
            }
        }
        Ok(CIExit::Released)
    }

    async fn release_package(&self, client: Client) -> Result<CIExit, Error> {
        log::info!("Running release for package");
        let Mode::Package(ref package) = self.mode else {
            log::error!("No package specified");
            return Err(Error::NoPackageSpecified);
        };

        let rel_package = package.package.to_string();
        log::info!("Running release for package: {rel_package}");

        let path = Path::new("./Cargo.toml");
        let workspace = Workspace::new(path).unwrap();

        let packages = workspace.packages();

        if let Some(packages) = packages {
            for package in packages {
                log::debug!("Found workspace package: {}", package.name);
                if package.name != *rel_package {
                    continue;
                }
                let prefix = format!("{}-{}", package.name, self.prefix);
                let version = package.version;
                ensure_github_release(&client, &prefix, &version).await?;
                break;
            }
        }
        Ok(CIExit::Released)
    }

    async fn release_current(self, client: Client) -> Result<CIExit, Error> {
        log::info!("Running release for package");

        let specific_package = {
            if let Mode::Current(ref current) = self.mode {
                if let Some(ref rel_package) = current.package {
                    log::info!("Running release for package: {rel_package}");
                    Some(rel_package.to_string())
                } else {
                    log::warn!("No package specified");
                    None
                }
            } else {
                log::warn!("No current configuration specified");
                None
            }
        };

        let path = Path::new("./Cargo.toml");
        let workspace = Workspace::new(path).unwrap();

        let packages = workspace.packages();

        if let Some(packages) = packages {
            for package in packages {
                log::debug!("Found workspace package: {}", package.name);
                if let Some(ref specific_pkg) = specific_package {
                    if package.name != *specific_pkg {
                        continue;
                    }
                }
                let prefix = format!("{}-{}", package.name, self.prefix);
                let version = package.version;
                ensure_github_release(&client, &prefix, &version).await?;
                break;
            }
        }
        Ok(CIExit::Released)
    }
    async fn release_version(
        self,
        mut client: Client,
        sign_config: SignConfig,
    ) -> Result<CIExit, Error> {
        let Mode::Version(ref version) = self.mode else {
            log::error!("Semver is required for release");
            return Err(Error::MissingSemver);
        };

        let version = version.version.to_string();
        log::info!("Running version release for release {version}");
        log::trace!(
            "PR ID: {} - Owner: {} - Repo: {}",
            client.pr_number(),
            client.owner(),
            client.repo()
        );
        log::trace!("Signing: {:?}", sign_config.sign);
        log::trace!("Update prlog flag: {}", self.update_prlog);

        if self.update_prlog {
            client.release_unreleased(&version)?;
            log::debug!("Changelog file name: {}", client.prlog_as_str());

            log::trace!(
                "{}",
                print_prlog(client.prlog_as_str(), client.line_limit())
            );

            let on_default_branch = client.branch_or_main() == client.default_branch.as_str();
            let commit_message = super::with_skip_ci(
                &release_prlog_commit_message(&version),
                self.should_skip_ci(),
                on_default_branch,
            );

            client
                .commit_changed_files(sign_config, &commit_message, &self.prefix, Some(&version))
                .await?;

            log::info!("Push the commit");
            log::trace!("tag_opt: {:?} and no_push: {:?}", Some(&version), false);

            let bot_user_name =
                std::env::var("BOT_USER_NAME").unwrap_or_else(|_| "bot".to_string());
            log::debug!("Using bot user name: {bot_user_name}");

            client.push_commit(&self.prefix, Some(&version), false, &bot_user_name)?;
            let hdr_style = Style::new().bold().underline();
            log::debug!("{}", "Check Push".style(hdr_style));
            log::debug!("Branch status: {}", client.branch_status()?);
        }

        client.make_release(&self.prefix, &version).await?;

        if self.linkedin_share {
            share_release_to_linkedin(&self.prefix, &version).await?;
        }

        Ok(CIExit::Released)
    }

    /// Check if a crate version is already published to crates.io.
    ///
    /// Uses the `kdeets_lib` library API to query the sparse registry cache,
    /// avoiding crates.io rate limiting.
    ///
    /// Writes `SKIP_PUBLISH=true/false` to `$BASH_ENV`.
    async fn check_version_published(self) -> Result<CIExit, Error> {
        let Mode::CheckVersionPublished(ref cmd) = self.mode else {
            return Err(Error::NoPackageSpecified);
        };

        let version = resolve_version(&cmd.version);

        if version == "none" {
            log::info!("No version to check — setting SKIP_PUBLISH=false");
            write_to_bash_env("SKIP_PUBLISH", "false")?;
            return Ok(CIExit::Released);
        }

        log::info!(
            "Checking if {package} {version} is on crates.io",
            package = cmd.package
        );

        let exists = map_kdeets_result(kdeets_lib::version_exists(&cmd.package, &version))?;

        if exists {
            log::info!("Version {version} already on crates.io — setting SKIP_PUBLISH=true");
            write_to_bash_env("SKIP_PUBLISH", "true")?;
        } else {
            log::info!("Version {version} not on crates.io — setting SKIP_PUBLISH=false");
            write_to_bash_env("SKIP_PUBLISH", "false")?;
        }

        Ok(CIExit::Released)
    }

    /// Check if the release tag already exists on the remote.
    ///
    /// Tag is constructed as `<package>-v<VERSION>`. Uses the GitHub API via
    /// the existing `client.tag_exists()` method.
    ///
    /// Writes `SKIP_RELEASE=true/false` to `$BASH_ENV`.
    async fn check_tag(self, client: Client) -> Result<CIExit, Error> {
        let Mode::CheckTag(ref cmd) = self.mode else {
            return Err(Error::NoPackageSpecified);
        };

        let version = resolve_version(&cmd.version);

        if version == "none" {
            log::info!("No version to check — setting SKIP_RELEASE=false");
            write_to_bash_env("SKIP_RELEASE", "false")?;
            return Ok(CIExit::Released);
        }

        let tag = format!("{}-v{}", cmd.package, version);
        log::info!("Checking if tag {tag} exists on remote");

        if client.tag_exists(&tag).await {
            log::info!("Tag {tag} already exists — setting SKIP_RELEASE=true");
            write_to_bash_env("SKIP_RELEASE", "true")?;
        } else {
            log::info!("Tag {tag} not found — setting SKIP_RELEASE=false");
            write_to_bash_env("SKIP_RELEASE", "false")?;
        }

        Ok(CIExit::Released)
    }

    /// Inject the confirmed signing pubkey into `Cargo.toml`, amend the release
    /// commit produced by `cargo release --no-push`, and move the signed tag to
    /// the amended commit.
    ///
    /// Reads pubkey from `--pubkey` flag or `$BINSTALL_SIGNING_PUBKEY`.
    /// Skips when version is "none", the crate is a library (no binary to
    /// sign), or no pubkey is available. A *binary* crate that is missing its
    /// signing scaffold errors instead of silently no-opping (pcu#1012), unless
    /// `--no-github-release`/`$PCU_NO_GITHUB_RELEASE` marks the binary as not
    /// published as a signed release.
    async fn inject_pubkey(self) -> Result<CIExit, Error> {
        let Mode::InjectPubkey(ref cmd) = self.mode else {
            return Err(Error::NoPackageSpecified);
        };

        let version = resolve_version(&cmd.version);

        if version == "none" {
            log::info!("No version set — skipping pubkey injection");
            return Ok(CIExit::Released);
        }

        let tag = format!("{}-v{}", cmd.package, version);
        let crate_dir = format!("crates/{}", cmd.package);
        let cargo_toml_path = format!("{crate_dir}/Cargo.toml");
        let content = fs::read_to_string(&cargo_toml_path)?;

        // Establish binary-vs-library independently of the signing scaffold:
        // a library has no binary to sign, so there is nothing to inject.
        let main_rs_exists = Path::new(&crate_dir).join("src/main.rs").exists();
        let bin_dir_exists = Path::new(&crate_dir).join("src/bin").is_dir();
        if !crate_is_binary(&content, main_rs_exists, bin_dir_exists)? {
            log::info!(
                "{} is a library crate — no binary to sign; skipping pubkey injection",
                cmd.package
            );
            return Ok(CIExit::Released);
        }

        let pubkey = cmd
            .pubkey
            .as_deref()
            .map(String::from)
            .or_else(|| std::env::var("BINSTALL_SIGNING_PUBKEY").ok());

        let Some(pubkey) = pubkey else {
            log::info!("No signing pubkey available — skipping Cargo.toml update");
            return Ok(CIExit::Released);
        };

        let no_github_release = cmd.no_github_release
            || std::env::var("PCU_NO_GITHUB_RELEASE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false);

        let updated = match inject_pubkey_value(&content, &pubkey, !no_github_release)? {
            PubkeyOutcome::Injected(new_content) => new_content,
            PubkeyOutcome::SkippedNoScaffold => {
                log::info!(
                    "{} has no binstall signing scaffold and --no-github-release is set — \
                     binary not published as a signed release; skipping pubkey injection",
                    cmd.package
                );
                return Ok(CIExit::Released);
            }
        };
        fs::write(&cargo_toml_path, &updated)?;
        log::info!("Updated {cargo_toml_path} with confirmed signing pubkey");

        // Stage the updated Cargo.toml
        let status = Command::new("git")
            .args(["add", &cargo_toml_path])
            .status()
            .map_err(|e| Error::GitError(format!("Failed to run git add: {e}")))?;
        if !status.success() {
            return Err(Error::GitError("git add failed".to_string()));
        }

        // Amend the release commit to include the pubkey
        let status = Command::new("git")
            .args(["commit", "--amend", "--no-edit", "-S"])
            .status()
            .map_err(|e| Error::GitError(format!("Failed to run git commit --amend: {e}")))?;
        if !status.success() {
            return Err(Error::GitError("git commit --amend failed".to_string()));
        }

        // Move the signed tag to the amended commit
        let status = Command::new("git")
            .args(["tag", "-f", "-s", &tag, "-m", &tag])
            .status()
            .map_err(|e| Error::GitError(format!("Failed to run git tag: {e}")))?;
        if !status.success() {
            return Err(Error::GitError(format!("git tag -f -s {tag} failed")));
        }

        log::info!("Release commit amended and tag {tag} moved to amended commit");
        Ok(CIExit::Released)
    }

    /// Attest a published crate with SLSA v0.2 provenance signed via Sigstore keyless.
    ///
    /// Steps:
    /// 1. Download the .crate from crates.io (with retry for indexing delay)
    /// 2. Compute SHA256 of the downloaded artifact
    /// 3. Generate SLSA v0.2 provenance JSON recording source, environment, and artifact
    /// 4. Sign the .crate with cosign-compatible keyless signing (CircleCI OIDC → Fulcio → Rekor)
    /// 5. Upload the .sigstore.json bundle and provenance.json to the GitHub release
    ///
    /// Requires CIRCLE_OIDC_TOKEN_V2 with audience "sigstore" in the environment.
    async fn attest(self, client: Client) -> Result<CIExit, Error> {
        let Mode::Attest(ref cmd) = self.mode else {
            return Err(Error::NoPackageSpecified);
        };

        let version = resolve_version(&cmd.version);
        if should_skip_attest(&version) {
            log::info!("No version to attest — skipping");
            return Ok(CIExit::Released);
        }

        let pkg = &cmd.package;
        let crate_filename = format!("{pkg}-{version}.crate");
        let crate_url = format!("https://static.crates.io/crates/{pkg}/{crate_filename}");
        let bundle_filename = format!("{crate_filename}.sigstore.json");
        let provenance_filename = format!("{pkg}-{version}.provenance.json");

        // Step 1: Check whether attestation assets already exist on the GitHub release.
        // If both assets are present the previous run completed successfully — skip all work.
        let release_tag = format!("{}{}", cmd.crate_tag_prefix, version);
        let release = client
            .github_rest
            .repos
            .get_release_by_tag(client.owner(), client.repo(), &release_tag)
            .send()
            .await?;
        let existing_assets: std::collections::HashSet<String> =
            release.assets.iter().map(|a| a.name.clone()).collect();
        if attestation_assets_already_uploaded(
            &existing_assets,
            &bundle_filename,
            &provenance_filename,
        ) {
            log::info!("Attestation assets already present on release {release_tag} — skipping");
            return Ok(CIExit::Released);
        }

        let attest_dir = std::path::Path::new("/tmp/attestation");
        std::fs::create_dir_all(attest_dir)?;
        let crate_path = attest_dir.join(&crate_filename);

        // Step 2: Download .crate from crates.io with retry
        log::info!(
            "Waiting {}s for crates.io indexing before download...",
            cmd.crates_io_delay
        );
        tokio::time::sleep(std::time::Duration::from_secs(cmd.crates_io_delay)).await;

        let http_client = reqwest::Client::new();
        let crate_bytes =
            download_with_retry(
                &crate_filename,
                cmd.max_attempts.into(),
                std::time::Duration::from_secs(30),
                || {
                    let client = http_client.clone();
                    let url = crate_url.clone();
                    async move {
                        let response =
                            client.get(&url).send().await.map_err(|e| {
                                Error::Attestation(format!("HTTP request failed: {e}"))
                            })?;
                        if !response.status().is_success() {
                            return Err(Error::Attestation(format!(
                                "HTTP {} for {url}",
                                response.status()
                            )));
                        }
                        response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
                            Error::Attestation(format!("Failed to read response: {e}"))
                        })
                    }
                },
            )
            .await?;
        std::fs::write(&crate_path, &crate_bytes)?;

        // Step 3: Read bytes and compute SHA256
        use sha2::Digest as _;
        let hash_hex = sha2::Sha256::digest(&crate_bytes).iter().fold(
            String::with_capacity(64),
            |mut s, b| {
                use std::fmt::Write as _;
                write!(s, "{b:02x}").unwrap();
                s
            },
        );
        log::info!("SHA256({crate_filename}) = {hash_hex}");

        // Step 4: Generate SLSA v0.2 provenance JSON
        let build_started = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let rust_version = std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let provenance = serde_json::json!({
            "builder": {
                "id": std::env::var("CIRCLE_BUILD_URL").unwrap_or_default()
            },
            "buildType": "https://github.com/jerus-org/circleci-toolkit",
            "invocation": {
                "configSource": {
                    "uri": std::env::var("CIRCLE_REPOSITORY_URL").unwrap_or_default(),
                    "digest": { "sha1": std::env::var("CIRCLE_SHA1").unwrap_or_default() },
                    "entryPoint": ".circleci/release.yml"
                },
                "parameters": {
                    "package": pkg,
                    "version": &version,
                    "rust_version": &rust_version
                },
                "environment": {
                    "CIRCLE_BUILD_URL": std::env::var("CIRCLE_BUILD_URL").unwrap_or_default(),
                    "CIRCLE_WORKFLOW_ID": std::env::var("CIRCLE_WORKFLOW_ID").unwrap_or_default(),
                    "CIRCLE_PROJECT_USERNAME": std::env::var("CIRCLE_PROJECT_USERNAME").unwrap_or_default(),
                    "CIRCLE_PROJECT_REPONAME": std::env::var("CIRCLE_PROJECT_REPONAME").unwrap_or_default()
                }
            },
            "metadata": {
                "buildStartedOn": build_started,
                "completeness": { "parameters": true, "environment": true, "materials": true },
                "reproducible": false
            },
            "materials": [
                {
                    "uri": std::env::var("CIRCLE_REPOSITORY_URL").unwrap_or_default(),
                    "digest": { "sha1": std::env::var("CIRCLE_SHA1").unwrap_or_default() }
                }
            ],
            "subject": [
                {
                    "name": &crate_filename,
                    "digest": { "sha256": &hash_hex }
                }
            ]
        });

        let provenance_path = attest_dir.join(&provenance_filename);
        std::fs::write(&provenance_path, serde_json::to_string_pretty(&provenance)?)?;
        log::info!("Generated provenance: {provenance_filename}");

        // Step 5: Sign with Sigstore keyless (CircleCI OIDC → Fulcio v1 → Rekor)
        let oidc_token_str = get_oidc_token()?;

        log::info!("Signing {crate_filename} via Fulcio v1 API...");
        let bundle_json = sign_artifact_fulcio_v1(&crate_bytes, &oidc_token_str).await?;

        let bundle_path = attest_dir.join(&bundle_filename);
        std::fs::write(&bundle_path, &bundle_json)?;
        log::info!("Bundle written: {bundle_filename}");

        // Step 6: Upload bundle and provenance to GitHub release
        log::info!("Uploading attestation assets to release {release_tag}...");

        let upload_token = PersonalAccessToken::new(client.github_token.clone());
        let upload_config = APIConfig::new("https://uploads.github.com", upload_token);
        let upload_api = octocrate::GitHubAPI::new(&upload_config);

        for (path, name) in [
            (&bundle_path, bundle_filename.as_str()),
            (&provenance_path, provenance_filename.as_str()),
        ] {
            let file = tokio::fs::File::open(path).await?;
            let content_length = file.metadata().await?.len();
            let query = octocrate::repos::upload_release_asset::Query::builder()
                .name(name)
                .build();
            upload_api
                .repos
                .upload_release_asset(client.owner(), client.repo(), release.id)
                .query(&query)
                .header("Content-Type", "application/octet-stream")
                .header("Content-Length", content_length.to_string())
                .file(file)
                .send()
                .await?;
            log::info!("Uploaded {name}");
        }

        log::info!("Attestation complete.");
        log::info!(
            "Verify with: cosign verify-blob \
            --certificate-oidc-issuer 'https://oidc.circleci.com/org/<ORG_ID>' \
            --certificate-identity-regexp 'https://circleci.com/gh/{}/.*' \
            --bundle '{bundle_filename}' '{crate_filename}'",
            std::env::var("CIRCLE_PROJECT_USERNAME").unwrap_or_default()
        );

        Ok(CIExit::Released)
    }

    /// Upload a binary asset to an existing GitHub release.
    ///
    /// Looks up the release ID via `get_release_by_tag`, then uploads to
    /// `uploads.github.com` using a dedicated `APIConfig` (octocrate's
    /// `upload_release_asset` requires this separate base URL).
    async fn upload_asset(self, client: Client) -> Result<CIExit, Error> {
        let Mode::UploadAsset(ref cmd) = self.mode else {
            return Err(Error::NoPackageSpecified);
        };

        let asset_name = cmd
            .asset_name
            .as_deref()
            .map(String::from)
            .or_else(|| {
                cmd.asset_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .ok_or_else(|| Error::GitError("Could not determine asset name".to_string()))?;

        if !cmd.asset_path.exists() {
            return Err(Error::GitError(format!(
                "Asset file not found: {}",
                cmd.asset_path.display()
            )));
        }

        log::info!("Looking up GitHub release for tag {}", cmd.tag);

        // Use a fresh API client per attempt so the closure owns all its data.
        // GitHub's REST API has a brief eventual-consistency window after
        // create_release: get_release_by_tag can return 404 for a few seconds.
        let token = client.github_token.clone();
        let owner = client.owner().to_string();
        let repo = client.repo().to_string();
        let tag = cmd.tag.clone();
        let release = get_release_with_retry(&tag, 5, std::time::Duration::from_secs(2), || {
            let tok = PersonalAccessToken::new(token.clone());
            let cfg = APIConfig::with_token(tok).shared();
            let api = GitHubAPI::new(&cfg);
            let o = owner.clone();
            let r = repo.clone();
            let t = tag.clone();
            async move {
                api.repos
                    .get_release_by_tag(&o, &r, &t)
                    .send()
                    .await
                    .map_err(Error::from)
            }
        })
        .await?;

        log::info!("Found release {} (id={})", release.tag_name, release.id);

        // GitHub binary uploads must go to uploads.github.com, not api.github.com.
        // A dedicated APIConfig with the upload base URL is required.
        let upload_token = PersonalAccessToken::new(client.github_token.clone());
        let upload_config = APIConfig::new("https://uploads.github.com", upload_token);
        let upload_api = octocrate::GitHubAPI::new(&upload_config);

        let file = tokio::fs::File::open(&cmd.asset_path).await?;
        let content_length = file.metadata().await?.len();

        // Minisign signatures are text; binaries use octet-stream
        let content_type = if asset_name.ends_with(".sig") {
            "text/plain"
        } else {
            "application/octet-stream"
        };

        let query = octocrate::repos::upload_release_asset::Query::builder()
            .name(asset_name.clone())
            .build();

        upload_api
            .repos
            .upload_release_asset(client.owner(), client.repo(), release.id)
            .query(&query)
            .header("Content-Type", content_type)
            .header("Content-Length", content_length.to_string())
            .file(file)
            .send()
            .await?;

        log::info!("Successfully uploaded {asset_name}");
        Ok(CIExit::Released)
    }
}

/// Returns true if the version string indicates no release is needed.
fn should_skip_attest(version: &str) -> bool {
    version == "none"
}

/// Returns true if all expected attestation assets are already present on the
/// GitHub release, indicating the upload was completed in a previous run.
///
/// When true, the entire attest operation (download, sign, upload) can be
/// skipped, making `pcu release attest` idempotent on re-runs.
fn attestation_assets_already_uploaded(
    existing_asset_names: &std::collections::HashSet<String>,
    bundle_filename: &str,
    provenance_filename: &str,
) -> bool {
    existing_asset_names.contains(bundle_filename)
        && existing_asset_names.contains(provenance_filename)
}

/// Read the CircleCI OIDC token (v2) from the environment.
///
/// Returns `Error::Attestation` with a clear message if the variable is unset.
fn get_oidc_token() -> Result<String, Error> {
    std::env::var("CIRCLE_OIDC_TOKEN_V2").map_err(|_| {
        Error::Attestation(
            "CIRCLE_OIDC_TOKEN_V2 is not set. \
            Set it to a CircleCI OIDC token with audience 'sigstore'. \
            Use `circleci run oidc get --claims '{\"aud\":\"sigstore\"}'` to obtain one."
                .to_string(),
        )
    })
}

/// Extract the `sub` claim from a raw JWT string without requiring an `email` claim.
///
/// CircleCI machine OIDC tokens do not include an `email` field; only `sub` is needed
/// as the challenge value for the Fulcio v1 signing endpoint.
fn extract_sub_from_jwt(raw_jwt: &str) -> Result<String, Error> {
    use base64::Engine as _;
    let parts: Vec<&str> = raw_jwt.split('.').collect();
    if parts.len() < 2 {
        return Err(Error::Attestation(
            "Invalid JWT format: expected at least 2 dot-separated parts".to_string(),
        ));
    }
    // JWT uses base64url (URL_SAFE_NO_PAD); fall back to STANDARD_NO_PAD for test tokens.
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(parts[1]))
        .map_err(|e| Error::Attestation(format!("JWT payload base64 decode failed: {e}")))?;
    let claims: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|e| Error::Attestation(format!("JWT payload JSON parse failed: {e}")))?;
    claims["sub"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Attestation("JWT missing 'sub' claim".to_string()))
}

/// Decode the first PEM certificate block to its raw DER bytes.
///
/// Fulcio v1 returns a certificate chain (leaf + intermediates) as multiple
/// PEM blocks.  Only the leaf (first block) is needed for the Sigstore bundle.
/// Joining all blocks before decoding would embed `=` padding mid-string,
/// causing base64 decode to fail with "Invalid symbol 61".
fn pem_to_der(pem_str: &str) -> Result<Vec<u8>, Error> {
    use base64::Engine as _;
    let b64: String = pem_str
        .lines()
        .skip_while(|l| !l.starts_with("-----BEGIN"))
        .skip(1)
        .take_while(|l| !l.starts_with("-----END"))
        .collect::<Vec<_>>()
        .join("");
    base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .map_err(|e| Error::Attestation(format!("PEM to DER conversion failed: {e}")))
}

/// Sign `artifact` bytes using the Fulcio v1 API with a CircleCI OIDC token.
///
/// The v1 path (`FulcioClient::request_cert`) uses `TokenProvider::Static` and signs
/// the challenge (= `sub` claim) to prove key possession.  It does NOT require an
/// `email` claim — making it compatible with CircleCI machine OIDC tokens.
///
/// Returns the Sigstore bundle JSON string.
async fn sign_artifact_fulcio_v1(artifact: &[u8], oidc_token_str: &str) -> Result<String, Error> {
    use base64::Engine as _;
    use sha2::Digest as _;
    use sigstore::crypto::SigningScheme;
    use sigstore::fulcio::{FulcioClient, TokenProvider, FULCIO_ROOT};
    use sigstore::rekor::apis::configuration::Configuration as RekorConfiguration;
    use sigstore::rekor::apis::entries_api::create_log_entry;
    use sigstore::rekor::models::hashedrekord;
    use sigstore::rekor::models::proposed_entry::ProposedEntry as ProposedLogEntry;
    use sigstore_protobuf_specs::dev::sigstore::bundle::v1::bundle;
    use sigstore_protobuf_specs::dev::sigstore::bundle::v1::verification_material;
    use sigstore_protobuf_specs::dev::sigstore::bundle::v1::{Bundle, VerificationMaterial};
    use sigstore_protobuf_specs::dev::sigstore::common::v1::{
        HashAlgorithm, HashOutput, MessageSignature, X509Certificate, X509CertificateChain,
    };
    use sigstore_protobuf_specs::dev::sigstore::rekor::v1::TransparencyLogEntry;
    use url::Url;

    // Extract sub claim (challenge for Fulcio)
    let sub = extract_sub_from_jwt(oidc_token_str)?;

    // Build CoreIdToken from raw JWT string
    let core_token: openidconnect::core::CoreIdToken =
        serde_json::from_value(serde_json::Value::String(oidc_token_str.to_string()))
            .map_err(|e| Error::Attestation(format!("Failed to parse OIDC token: {e}")))?;

    // Create Fulcio client with v1 Static provider
    let fulcio_url = Url::parse(FULCIO_ROOT)
        .map_err(|e| Error::Attestation(format!("Invalid Fulcio URL: {e}")))?;
    let fulcio = FulcioClient::new(fulcio_url, TokenProvider::Static((core_token, sub)));

    // Request Fulcio certificate via v1 endpoint
    log::info!("Requesting Fulcio signing certificate via v1 API...");
    let (signer, cert_pem) = fulcio
        .request_cert(SigningScheme::ECDSA_P256_SHA256_ASN1)
        .await
        .map_err(|e| Error::Attestation(format!("Fulcio certificate request failed: {e}")))?;

    // Compute SHA256 of artifact
    let sha256_hash = sha2::Sha256::digest(artifact);
    let sha256_hex = sha256_hash
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write as _;
            write!(s, "{b:02x}").unwrap();
            s
        });

    // Sign artifact bytes
    let signature_bytes = signer
        .sign(artifact)
        .map_err(|e| Error::Attestation(format!("Artifact signing failed: {e}")))?;

    // Convert cert PEM to DER for the bundle
    let cert_der = pem_to_der(&cert_pem.to_string())?;

    // Submit to Rekor transparency log
    let proposed_entry = ProposedLogEntry::Hashedrekord {
        api_version: "0.0.1".to_owned(),
        spec: hashedrekord::Spec {
            signature: hashedrekord::Signature {
                content: base64::engine::general_purpose::STANDARD.encode(&signature_bytes),
                public_key: hashedrekord::PublicKey::new(
                    base64::engine::general_purpose::STANDARD.encode(cert_pem.as_ref()),
                ),
            },
            data: hashedrekord::Data {
                hash: hashedrekord::Hash {
                    algorithm: hashedrekord::AlgorithmKind::sha256,
                    value: sha256_hex,
                },
            },
        },
    };

    log::info!("Submitting to Rekor transparency log...");
    let log_entry = create_log_entry(&RekorConfiguration::default(), proposed_entry)
        .await
        .map_err(|e| Error::Attestation(format!("Rekor submission failed: {e}")))?;
    let tlog_entry: TransparencyLogEntry = log_entry
        .try_into()
        .map_err(|_| Error::Attestation("Rekor returned malformed log entry".to_string()))?;

    // Build Sigstore bundle
    let x509_chain = X509CertificateChain {
        certificates: vec![X509Certificate {
            raw_bytes: cert_der,
        }],
    };
    let verification_material = Some(VerificationMaterial {
        timestamp_verification_data: None,
        tlog_entries: vec![tlog_entry],
        content: Some(verification_material::Content::X509CertificateChain(
            x509_chain,
        )),
    });
    let message_signature = MessageSignature {
        message_digest: Some(HashOutput {
            algorithm: HashAlgorithm::Sha2256.into(),
            digest: sha256_hash.to_vec(),
        }),
        signature: signature_bytes,
    };
    let bundle = Bundle {
        media_type: "application/vnd.dev.sigstore.bundle+json;version=0.2".to_string(),
        verification_material,
        content: Some(bundle::Content::MessageSignature(message_signature)),
    };

    serde_json::to_string_pretty(&bundle)
        .map_err(|e| Error::Attestation(format!("Bundle serialisation failed: {e}")))
}

fn print_prlog(prlog_path: &str, mut line_limit: usize) -> String {
    let mut output = String::new();

    if let Ok(change_log) = fs::read_to_string(prlog_path) {
        let mut line_count = 0;
        if line_limit == 0 {
            line_limit = change_log.lines().count();
        };

        output.push_str("\n*****Changelog*****:\n----------------------------");
        for line in change_log.lines() {
            output.push_str(format!("{line}\n").as_str());
            line_count += 1;
            if line_count >= line_limit {
                break;
            }
        }
        output.push_str("----------------------------\n");
    };

    output
}

#[cfg(test)]
mod attest_tests {
    use super::*;

    #[tokio::test]
    async fn download_with_retry_succeeds_on_first_attempt() {
        let result =
            download_with_retry("test-1.0.0.crate", 3, std::time::Duration::ZERO, || async {
                Ok(b"crate-data".to_vec())
            })
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"crate-data");
    }

    #[tokio::test]
    async fn download_with_retry_returns_err_after_all_attempts_exhausted() {
        let result =
            download_with_retry("test-1.0.0.crate", 3, std::time::Duration::ZERO, || async {
                Err(Error::Attestation("HTTP 503".to_string()))
            })
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("test-1.0.0.crate"),
            "error should name the file: {msg}"
        );
        assert!(
            msg.contains('3'),
            "error should mention attempt count: {msg}"
        );
    }

    #[tokio::test]
    async fn download_with_retry_succeeds_on_second_attempt() {
        let attempt = std::sync::atomic::AtomicU32::new(0);
        let result =
            download_with_retry("test-1.0.0.crate", 3, std::time::Duration::ZERO, || async {
                let n = attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n == 0 {
                    Err(Error::Attestation("first attempt failed".to_string()))
                } else {
                    Ok(b"crate-data".to_vec())
                }
            })
            .await;
        assert!(result.is_ok());
    }

    /// Build a minimal fake JWT string.
    ///
    /// Uses URL_SAFE_NO_PAD base64 (standard JWT encoding).
    /// The signature is fake — we only parse the payload claims.
    fn fake_jwt(sub: &str) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(format!(
            r#"{{"aud":"sigstore","exp":9999999999,"sub":"{sub}"}}"#
        ));
        format!("{header}.{payload}.fakesig")
    }

    #[test]
    fn attest_skips_when_version_is_none() {
        assert!(
            should_skip_attest("none"),
            "version 'none' should trigger skip"
        );
    }

    #[test]
    fn attest_does_not_skip_when_version_is_present() {
        assert!(
            !should_skip_attest("1.2.3"),
            "a real version should not trigger skip"
        );
    }

    #[test]
    fn get_oidc_token_errors_when_env_var_missing() {
        let saved = std::env::var("CIRCLE_OIDC_TOKEN_V2").ok();
        unsafe { std::env::remove_var("CIRCLE_OIDC_TOKEN_V2") };

        let result = get_oidc_token();

        if let Some(v) = saved {
            unsafe { std::env::set_var("CIRCLE_OIDC_TOKEN_V2", v) };
        }

        assert!(result.is_err(), "should error when env var is absent");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("CIRCLE_OIDC_TOKEN_V2"),
            "error should name the missing env var: {msg}"
        );
    }

    #[test]
    fn get_oidc_token_returns_value_when_env_var_set() {
        let saved = std::env::var("CIRCLE_OIDC_TOKEN_V2").ok();
        unsafe { std::env::set_var("CIRCLE_OIDC_TOKEN_V2", "some-token") };

        let result = get_oidc_token();

        unsafe { std::env::remove_var("CIRCLE_OIDC_TOKEN_V2") };
        if let Some(v) = saved {
            unsafe { std::env::set_var("CIRCLE_OIDC_TOKEN_V2", v) };
        }

        assert_eq!(result.unwrap(), "some-token");
    }

    #[test]
    fn extract_sub_from_jwt_returns_sub_claim() {
        let jwt = fake_jwt("https://circleci.com/org/abc/project/xyz/user/u");
        let result = extract_sub_from_jwt(&jwt);
        assert!(result.is_ok(), "should extract sub: {result:?}");
        assert_eq!(
            result.unwrap(),
            "https://circleci.com/org/abc/project/xyz/user/u"
        );
    }

    #[test]
    fn extract_sub_from_jwt_errors_on_malformed_jwt() {
        let result = extract_sub_from_jwt("not-a-jwt");
        assert!(result.is_err(), "malformed JWT should fail");
    }

    #[test]
    fn extract_sub_from_jwt_errors_when_sub_missing() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(r#"{"aud":"sigstore","exp":9999999999}"#);
        let jwt = format!("{header}.{payload}.fakesig");
        let result = extract_sub_from_jwt(&jwt);
        assert!(result.is_err(), "missing sub should fail");
        assert!(
            result.unwrap_err().to_string().contains("sub"),
            "error should mention 'sub'"
        );
    }

    #[test]
    fn pem_to_der_roundtrips_certificate_bytes() {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        // Fabricate a fake "certificate" (just some bytes)
        let fake_der = b"FAKE_DER_BYTES_0123456789";
        let b64 = STANDARD.encode(fake_der);
        let pem = format!("-----BEGIN CERTIFICATE-----\n{b64}\n-----END CERTIFICATE-----\n");
        let result = pem_to_der(&pem);
        assert!(result.is_ok(), "pem_to_der should succeed: {result:?}");
        assert_eq!(result.unwrap(), fake_der);
    }

    #[test]
    fn pem_to_der_returns_only_first_cert_from_chain() {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        // Fulcio v1 returns a chain: leaf cert + intermediate(s).
        // Only the leaf (first block) should be decoded; the intermediate's
        // base64 padding ('=') must not contaminate the leaf decode.
        let leaf_der = b"LEAF_CERT_BYTES";
        let intermediate_der = b"INTERMEDIATE_CERT_BYTES_LONGER";
        let leaf_b64 = STANDARD.encode(leaf_der);
        let intermediate_b64 = STANDARD.encode(intermediate_der);
        let chain_pem = format!(
            "-----BEGIN CERTIFICATE-----\n{leaf_b64}\n-----END CERTIFICATE-----\n\
             -----BEGIN CERTIFICATE-----\n{intermediate_b64}\n-----END CERTIFICATE-----\n"
        );
        let result = pem_to_der(&chain_pem);
        assert!(
            result.is_ok(),
            "pem_to_der should handle a cert chain: {result:?}"
        );
        assert_eq!(
            result.unwrap(),
            leaf_der,
            "should return only the leaf (first) certificate"
        );
    }
}

#[cfg(test)]
mod release_package_tests {
    use super::*;

    #[tokio::test]
    async fn tag_visible_with_retry_true_on_first_attempt() {
        let attempt = std::sync::atomic::AtomicU32::new(0);
        let visible = tag_visible_with_retry("crate-v1.0.0", 5, std::time::Duration::ZERO, || {
            attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async { true }
        })
        .await;
        assert!(visible);
        assert_eq!(
            attempt.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "stop polling as soon as the tag is visible"
        );
    }

    #[tokio::test]
    async fn tag_visible_with_retry_true_when_visible_after_api_lag() {
        // GitHub's REST API lags behind the git tag push: the first poll(s)
        // return false, then the tag becomes visible.
        let attempt = std::sync::atomic::AtomicU32::new(0);
        let visible = tag_visible_with_retry("crate-v1.0.0", 5, std::time::Duration::ZERO, || {
            let n = attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move { n >= 2 }
        })
        .await;
        assert!(visible, "tag must be detected once the API catches up");
        assert_eq!(attempt.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn tag_visible_with_retry_false_after_all_attempts() {
        let attempt = std::sync::atomic::AtomicU32::new(0);
        let visible = tag_visible_with_retry("crate-v1.0.0", 3, std::time::Duration::ZERO, || {
            attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async { false }
        })
        .await;
        assert!(
            !visible,
            "a genuinely-absent tag returns false (not an error)"
        );
        assert_eq!(
            attempt.load(std::sync::atomic::Ordering::SeqCst),
            3,
            "every attempt is exhausted before giving up"
        );
    }
}

use crate::client::get_release_with_retry;

/// Downloads a URL with retry, using a caller-supplied async attempt function.
///
/// `attempt_fn` is called up to `max_attempts` times. On success it returns
/// `Ok(Vec<u8>)` containing the downloaded bytes. On failure the error is
/// logged and (if attempts remain) the retry delay is observed before the next
/// attempt. After all attempts are exhausted an `Error::Attestation` is
/// returned naming the file and the attempt count.
async fn download_with_retry<F, Fut>(
    crate_filename: &str,
    max_attempts: u64,
    retry_delay: std::time::Duration,
    mut attempt_fn: F,
) -> Result<Vec<u8>, Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<u8>, Error>>,
{
    for attempt in 1..=max_attempts {
        match attempt_fn().await {
            Ok(bytes) => {
                log::info!("Downloaded {crate_filename} (attempt {attempt})");
                return Ok(bytes);
            }
            Err(e) => {
                log::warn!("Download attempt {attempt} failed: {e}");
                if attempt < max_attempts {
                    log::info!("Retrying in {}s...", retry_delay.as_secs());
                    tokio::time::sleep(retry_delay).await;
                }
            }
        }
    }
    Err(Error::Attestation(format!(
        "Failed to download {crate_filename} after {max_attempts} attempts"
    )))
}

/// Maps a `kdeets_lib::version_exists` result to `Result<bool, Error>`.
///
/// `CrateNotFoundOnIndex` is treated as "not published" (`Ok(false)`) so that
/// a brand-new crate that has never appeared on the index does not block publishing.
/// All other kdeets errors propagate as `Error::GitError`.
fn map_kdeets_result(result: Result<bool, kdeets_lib::Error>) -> Result<bool, Error> {
    match result {
        Ok(exists) => Ok(exists),
        Err(kdeets_lib::Error::CrateNotFoundOnIndex) => Ok(false),
        Err(e) => Err(Error::GitError(format!("kdeets error: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_resolve_version_returns_explicit() {
        assert_eq!(resolve_version(&Some("1.2.3".to_string())), "1.2.3");
    }

    // Env-var-dependent cases run sequentially within one function to avoid
    // races with other tests that might concurrently read SEMVER / NEXT_VERSION.
    #[test]
    fn test_resolve_version_env_fallbacks() {
        // Explicit arg overrides any env var.
        unsafe { std::env::set_var("SEMVER", "9.9.9") };
        assert_eq!(resolve_version(&Some("1.0.0".to_string())), "1.0.0");

        // SEMVER is used when arg is None.
        unsafe { std::env::remove_var("NEXT_VERSION") };
        assert_eq!(resolve_version(&None), "9.9.9");

        // NEXT_VERSION is used when SEMVER is absent.
        unsafe {
            std::env::remove_var("SEMVER");
            std::env::set_var("NEXT_VERSION", "3.1.0");
        }
        assert_eq!(resolve_version(&None), "3.1.0");

        // Returns "none" when neither env var is set.
        unsafe { std::env::remove_var("NEXT_VERSION") };
        assert_eq!(resolve_version(&None), "none");
    }

    // BASH_ENV tests run sequentially within one function to avoid races.
    #[test]
    fn test_write_to_bash_env() {
        let saved_bash_env = std::env::var("BASH_ENV").ok();

        // Returns Ok(()) with no side-effects when BASH_ENV is not set.
        unsafe { std::env::remove_var("BASH_ENV") };
        assert!(write_to_bash_env("KEY", "val").is_ok());

        // Writes a single `export KEY=VALUE` line.
        let mut tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        unsafe { std::env::set_var("BASH_ENV", &path) };
        write_to_bash_env("SKIP_PUBLISH", "true").unwrap();
        unsafe { std::env::remove_var("BASH_ENV") };
        let mut contents = String::new();
        tmp.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "export SKIP_PUBLISH=true\n");

        // Successive calls append separate lines.
        let mut tmp2 = NamedTempFile::new().unwrap();
        let path2 = tmp2.path().to_str().unwrap().to_string();
        unsafe { std::env::set_var("BASH_ENV", &path2) };
        write_to_bash_env("SKIP_RELEASE", "false").unwrap();
        write_to_bash_env("SKIP_PUBLISH", "false").unwrap();
        unsafe { std::env::remove_var("BASH_ENV") };
        let mut contents2 = String::new();
        tmp2.read_to_string(&mut contents2).unwrap();
        assert_eq!(
            contents2,
            "export SKIP_RELEASE=false\nexport SKIP_PUBLISH=false\n"
        );

        // Restore BASH_ENV if it was set before the test.
        if let Some(v) = saved_bash_env {
            unsafe { std::env::set_var("BASH_ENV", v) };
        }
    }

    #[test]
    fn attestation_assets_already_uploaded_true_when_both_present() {
        let existing = std::collections::HashSet::from([
            "my-crate-1.2.3.crate.sigstore.json".to_string(),
            "my-crate-1.2.3.provenance.json".to_string(),
        ]);
        assert!(attestation_assets_already_uploaded(
            &existing,
            "my-crate-1.2.3.crate.sigstore.json",
            "my-crate-1.2.3.provenance.json",
        ));
    }

    #[test]
    fn attestation_assets_already_uploaded_false_when_both_absent() {
        let existing = std::collections::HashSet::new();
        assert!(!attestation_assets_already_uploaded(
            &existing,
            "my-crate-1.2.3.crate.sigstore.json",
            "my-crate-1.2.3.provenance.json",
        ));
    }

    #[test]
    fn attestation_assets_already_uploaded_false_when_only_bundle_present() {
        let existing =
            std::collections::HashSet::from(["my-crate-1.2.3.crate.sigstore.json".to_string()]);
        assert!(!attestation_assets_already_uploaded(
            &existing,
            "my-crate-1.2.3.crate.sigstore.json",
            "my-crate-1.2.3.provenance.json",
        ));
    }

    #[test]
    fn attestation_assets_already_uploaded_false_when_only_provenance_present() {
        let existing =
            std::collections::HashSet::from(["my-crate-1.2.3.provenance.json".to_string()]);
        assert!(!attestation_assets_already_uploaded(
            &existing,
            "my-crate-1.2.3.crate.sigstore.json",
            "my-crate-1.2.3.provenance.json",
        ));
    }

    #[test]
    fn map_kdeets_result_ok_true_returns_ok_true() {
        let result = map_kdeets_result(Ok(true));
        assert!(matches!(result, Ok(true)));
    }

    #[test]
    fn map_kdeets_result_ok_false_returns_ok_false() {
        let result = map_kdeets_result(Ok(false));
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn map_kdeets_result_crate_not_found_returns_ok_false() {
        let result = map_kdeets_result(Err(kdeets_lib::Error::CrateNotFoundOnIndex));
        assert!(
            matches!(result, Ok(false)),
            "CrateNotFoundOnIndex should be treated as version not published"
        );
    }
}

#[cfg(test)]
mod upload_retry_tests {
    use super::*;

    #[tokio::test]
    async fn get_release_with_retry_succeeds_on_first_attempt() {
        let result = get_release_with_retry("pcu-v1.0.0", 3, std::time::Duration::ZERO, || async {
            Ok::<_, Error>("release")
        })
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "release");
    }

    #[tokio::test]
    async fn get_release_with_retry_returns_err_after_all_attempts_exhausted() {
        let result = get_release_with_retry("pcu-v1.0.0", 3, std::time::Duration::ZERO, || async {
            Err::<String, Error>(Error::GitError("Not Found".to_string()))
        })
        .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("pcu-v1.0.0"),
            "error should name the tag: {msg}"
        );
        assert!(
            msg.contains('3'),
            "error should mention attempt count: {msg}"
        );
    }

    #[tokio::test]
    async fn get_release_with_retry_succeeds_on_second_attempt() {
        let attempt = std::sync::atomic::AtomicU32::new(0);
        let result = get_release_with_retry("pcu-v1.0.0", 3, std::time::Duration::ZERO, || async {
            let n = attempt.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                Err::<String, Error>(Error::GitError("Not Found".to_string()))
            } else {
                Ok("release".to_string())
            }
        })
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "release");
    }
}

#[cfg(test)]
mod ensure_release_tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn creates_release_when_tag_present_and_release_absent() {
        let made = AtomicU32::new(0);
        let outcome = ensure_release_for_tag(
            "crate-v1.0.0",
            || async { true },
            || async { Ok(false) },
            || async {
                made.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        )
        .await;
        assert_eq!(outcome.unwrap(), EnsureOutcome::Created);
        assert_eq!(
            made.load(Ordering::SeqCst),
            1,
            "make_release must be called exactly once"
        );
    }

    #[tokio::test]
    async fn errors_when_make_release_fails() {
        let r = ensure_release_for_tag(
            "crate-v1.0.0",
            || async { true },
            || async { Ok(false) },
            || async { Err(Error::GitError("boom".into())) },
        )
        .await;
        assert!(r.is_err(), "a failed creation must surface as an error");
    }

    #[tokio::test]
    async fn errors_when_release_existence_undetermined() {
        // The core fix: a non-conclusive existence check must NOT become a
        // silent Ok-skip (the 0.0.53 stranding). It must error and not create.
        let made = AtomicU32::new(0);
        let r = ensure_release_for_tag(
            "crate-v1.0.0",
            || async { true },
            || async { Err(Error::GitError("api 500".into())) },
            || async {
                made.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        )
        .await;
        assert!(r.is_err(), "undetermined existence must error, not skip");
        assert_eq!(
            made.load(Ordering::SeqCst),
            0,
            "must not blindly create when existence is undetermined"
        );
    }

    #[tokio::test]
    async fn skips_when_release_already_present() {
        let made = AtomicU32::new(0);
        let outcome = ensure_release_for_tag(
            "crate-v1.0.0",
            || async { true },
            || async { Ok(true) },
            || async {
                made.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        )
        .await;
        assert_eq!(outcome.unwrap(), EnsureOutcome::AlreadyPresent);
        assert_eq!(
            made.load(Ordering::SeqCst),
            0,
            "must not re-create an existing release"
        );
    }

    #[tokio::test]
    async fn reports_tag_absent_without_error() {
        let made = AtomicU32::new(0);
        let outcome = ensure_release_for_tag(
            "crate-v1.0.0",
            || async { false },
            || async { Ok(false) },
            || async {
                made.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        )
        .await;
        assert_eq!(outcome.unwrap(), EnsureOutcome::TagAbsent);
        assert_eq!(made.load(Ordering::SeqCst), 0);
    }
}

#[cfg(test)]
mod release_message_tests {
    use super::*;

    #[test]
    fn release_prlog_message_names_the_release_not_a_pr() {
        let m = release_prlog_commit_message("1.2.3");
        assert!(
            m.contains("release") && m.contains("1.2.3"),
            "release prlog commit must name the release + version: {m}"
        );
        assert!(
            !m.contains("for pr"),
            "must not masquerade as a routine post-merge pr prlog update: {m}"
        );
    }
}

#[cfg(test)]
mod inject_pubkey_tests {
    use super::*;

    const BIN_MANIFEST: &str = r#"
[package]
name = "demo"
version = "0.1.0"

[[bin]]
name = "demo"
path = "src/main.rs"

[package.metadata.binstall.signing]
algorithm = "minisign"
pubkey = "RWPLACEHOLDER0000000000000000000000000000000"
"#;

    const BIN_MANIFEST_NO_SCAFFOLD: &str = r#"
[package]
name = "demo"
version = "0.1.0"

[[bin]]
name = "demo"
path = "src/main.rs"
"#;

    const LIB_MANIFEST: &str = r#"
[package]
name = "demo"
version = "0.1.0"

[lib]
name = "demo"
path = "src/lib.rs"
"#;

    // ---- crate_is_binary ----

    #[test]
    fn manifest_with_bin_target_is_binary() {
        assert!(crate_is_binary(BIN_MANIFEST, false, false).unwrap());
    }

    #[test]
    fn library_manifest_without_bin_or_files_is_not_binary() {
        assert!(!crate_is_binary(LIB_MANIFEST, false, false).unwrap());
    }

    #[test]
    fn auto_detected_main_rs_makes_it_binary() {
        // No [[bin]] in the manifest, but src/main.rs on disk → cargo builds a binary.
        assert!(crate_is_binary(LIB_MANIFEST, true, false).unwrap());
    }

    #[test]
    fn auto_detected_bin_dir_makes_it_binary() {
        assert!(crate_is_binary(LIB_MANIFEST, false, true).unwrap());
    }

    // ---- inject_pubkey_value ----

    #[test]
    fn injects_key_when_placeholder_present() {
        let outcome = inject_pubkey_value(BIN_MANIFEST, "RWNEWKEY111", true).unwrap();
        match outcome {
            PubkeyOutcome::Injected(new) => {
                assert!(
                    new.contains(r#"pubkey = "RWNEWKEY111""#),
                    "new key must be written: {new}"
                );
                assert!(
                    !new.contains("RWPLACEHOLDER"),
                    "placeholder must be replaced: {new}"
                );
            }
            other => panic!("expected Injected, got {other:?}"),
        }
    }

    #[test]
    fn binary_missing_scaffold_is_an_error_when_required() {
        // The jerus-org/pcu#1012 bug: a binary crate whose Cargo.toml has no
        // pubkey line must NOT silently no-op.
        let err = inject_pubkey_value(BIN_MANIFEST_NO_SCAFFOLD, "RWNEWKEY111", true).unwrap_err();
        assert!(
            matches!(err, Error::MissingSigningScaffold(_)),
            "expected MissingSigningScaffold, got {err:?}"
        );
    }

    #[test]
    fn binary_missing_scaffold_is_skipped_when_not_required() {
        // --no-github-release: an unpublished binary may legitimately lack scaffold.
        let outcome = inject_pubkey_value(BIN_MANIFEST_NO_SCAFFOLD, "RWNEWKEY111", false).unwrap();
        assert_eq!(outcome, PubkeyOutcome::SkippedNoScaffold);
    }

    #[test]
    fn existing_real_key_is_replaced_idempotently() {
        let first = inject_pubkey_value(BIN_MANIFEST, "RWNEWKEY111", true).unwrap();
        let content = match first {
            PubkeyOutcome::Injected(c) => c,
            other => panic!("expected Injected, got {other:?}"),
        };
        // Re-running with a different key replaces again (no stacking).
        let second = inject_pubkey_value(&content, "RWNEWKEY222", true).unwrap();
        match second {
            PubkeyOutcome::Injected(new) => {
                assert!(new.contains(r#"pubkey = "RWNEWKEY222""#));
                assert!(!new.contains("RWNEWKEY111"));
                assert_eq!(
                    new.matches("pubkey = ").count(),
                    1,
                    "exactly one pubkey line"
                );
            }
            other => panic!("expected Injected, got {other:?}"),
        }
    }
}
