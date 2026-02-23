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
