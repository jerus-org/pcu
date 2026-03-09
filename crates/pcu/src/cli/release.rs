use crate::utilities::linkedin_post::{build_release_text, compute_release_url};
use gen_linkedin::posts::{PostsClient, TextPost};
use gen_linkedin::{auth::StaticTokenProvider, client::Client as LiClient};

async fn share_release_to_linkedin(prefix: &str, version: &str) -> Result<(), Error> {
    let settings = super::Commands::Release(Release {
        update_prlog: false,
        prefix: prefix.to_string(),
        linkedin_share: true,
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
use octocrate::{APIConfig, PersonalAccessToken};
use owo_colors::{OwoColorize, Style};

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
    #[command(subcommand)]
    pub mode: Mode,
}

impl Release {
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
                let tag = format!("{prefix}{version}");
                if !client.tag_exists(&tag).await {
                    log::error!("Tag does not exist: {tag}");
                } else {
                    log::info!("Tag already exists: {tag}, attempt to make release");
                    client.make_release(&prefix, &version).await?;
                }
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
                let tag = format!("{prefix}{version}");
                log::trace!("Checking for tag `{tag}` to make release against.");
                if !client.tag_exists(&tag).await {
                    log::error!("Tag does not exist: {tag}");
                } else {
                    log::info!("Tag already exists: {tag}, attempt to make release");
                    client.make_release(&prefix, &version).await?;
                }
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
                let tag = format!("{prefix}{version}");
                log::trace!("Checking for tag `{tag}` to make release against.");
                if !client.tag_exists(&tag).await {
                    log::error!("Tag does not exist: {tag}");
                } else {
                    log::info!("Tag already exists: {tag}, attempt to make release");
                    client.make_release(&prefix, &version).await?;
                }
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

            let commit_message = "chore: update prlog for pr";

            client
                .commit_changed_files(sign_config, commit_message, &self.prefix, Some(&version))
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
    /// Uses `kdeets` (pre-installed in ci-container) to query the sparse registry
    /// cache, avoiding crates.io rate limiting. See jerus-org/kdeets#170 for a
    /// future library API that would replace this subprocess call.
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

        let output = Command::new("kdeets")
            .args(["--no-colour", "crate", &cmd.package, "-l"])
            .output()
            .map_err(|e| Error::GitError(format!("Failed to run kdeets: {e}")))?;

        let kdeets_output = String::from_utf8_lossy(&output.stdout);
        let version_exists = kdeets_output
            .lines()
            .any(|line| line.split_whitespace().last() == Some(version.as_str()));

        if version_exists {
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
    /// Silently skips when version is "none" or pubkey is unavailable.
    async fn inject_pubkey(self) -> Result<CIExit, Error> {
        let Mode::InjectPubkey(ref cmd) = self.mode else {
            return Err(Error::NoPackageSpecified);
        };

        let version = resolve_version(&cmd.version);

        if version == "none" {
            log::info!("No version set — skipping pubkey injection");
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

        let tag = format!("{}-v{}", cmd.package, version);
        let cargo_toml_path = format!("crates/{}/Cargo.toml", cmd.package);

        // Replace the pubkey value in Cargo.toml (uses | as delimiter to avoid
        // conflicts with the pubkey's base62 characters)
        let content = fs::read_to_string(&cargo_toml_path)?;
        let updated = regex::Regex::new(r#"pubkey = ".*""#)?
            .replace(&content, format!(r#"pubkey = "{pubkey}""#).as_str())
            .into_owned();
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
        let attest_dir = std::path::Path::new("/tmp/attestation");
        std::fs::create_dir_all(attest_dir)?;
        let crate_path = attest_dir.join(&crate_filename);

        // Step 1: Download .crate from crates.io with retry
        log::info!(
            "Waiting {}s for crates.io indexing before download...",
            cmd.crates_io_delay
        );
        tokio::time::sleep(std::time::Duration::from_secs(cmd.crates_io_delay)).await;

        let mut downloaded = false;
        for attempt in 1..=cmd.max_attempts {
            let crate_path_str = crate_path
                .to_str()
                .ok_or_else(|| Error::Attestation("Invalid crate path".to_string()))?;
            let status = std::process::Command::new("curl")
                .args(["-sSfL", &crate_url, "-o", crate_path_str])
                .status()?;
            if status.success() {
                log::info!("Downloaded {crate_filename} (attempt {attempt})");
                downloaded = true;
                break;
            }
            log::warn!("Download attempt {attempt} failed ({status})");
            if attempt < cmd.max_attempts {
                log::info!("Retrying in 30s...");
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        }
        if !downloaded {
            return Err(Error::Attestation(format!(
                "Failed to download {crate_filename} after {} attempts",
                cmd.max_attempts
            )));
        }

        // Step 2: Read bytes and compute SHA256
        let crate_bytes = std::fs::read(&crate_path)?;
        use sha2::Digest as _;
        let hash_hex = format!("{:x}", sha2::Sha256::digest(&crate_bytes));
        log::info!("SHA256({crate_filename}) = {hash_hex}");

        // Step 3: Generate SLSA v0.2 provenance JSON
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

        // Step 4: Sign with Sigstore keyless (CircleCI OIDC → Fulcio v1 → Rekor)
        let oidc_token_str = get_oidc_token()?;

        log::info!("Signing {crate_filename} via Fulcio v1 API...");
        let bundle_json = sign_artifact_fulcio_v1(&crate_bytes, &oidc_token_str).await?;

        let bundle_path = attest_dir.join(&bundle_filename);
        std::fs::write(&bundle_path, &bundle_json)?;
        log::info!("Bundle written: {bundle_filename}");

        // Step 5: Upload bundle and provenance to GitHub release
        let release_tag = format!("{}{}", cmd.crate_tag_prefix, version);
        log::info!("Uploading attestation assets to release {release_tag}...");

        let release = client
            .github_rest
            .repos
            .get_release_by_tag(client.owner(), client.repo(), &release_tag)
            .send()
            .await?;

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

        let release = client
            .github_rest
            .repos
            .get_release_by_tag(client.owner(), client.repo(), &cmd.tag)
            .send()
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

/// Decode a PEM certificate string to its raw DER bytes.
///
/// Strips the `-----BEGIN ...-----` / `-----END ...-----` header lines and
/// base64-decodes the remaining content.
fn pem_to_der(pem_str: &str) -> Result<Vec<u8>, Error> {
    use base64::Engine as _;
    let b64: String = pem_str
        .lines()
        .filter(|l| !l.starts_with("-----"))
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
    let sha256_hex = format!("{:x}", &sha256_hash);

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
}
