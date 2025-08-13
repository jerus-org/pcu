use std::{fs, path::Path};

use super::{CIExit, Commands};
use crate::{Client, Error, GitOps, MakeRelease, Sign, Workspace};
mod mode;

use clap::Parser;
use mode::Mode;
use owo_colors::{OwoColorize, Style};

#[derive(Debug, Parser, Clone)]
pub struct Release {
    /// Update the changelog by renaming the unreleased section with the version number
    #[arg(short, long, default_value_t = false)]
    pub update_changelog: bool,
    /// Prefix for the version tag
    #[clap(short, long, default_value_t = String::from("v"))]
    pub prefix: String,
    #[command(subcommand)]
    pub mode: Mode,
}

impl Release {
    pub async fn run_release(self, sign: Sign) -> Result<CIExit, Error> {
        let client = Commands::Release(self.clone()).get_client().await?;

        match self.mode {
            Mode::Version(_) => self.release_version(client, sign).await,
            Mode::Package(_) => self.release_package(client).await,
            Mode::Workspace => self.release_workspace(client).await,
            Mode::Current(_) => self.release_current(client).await,
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
    async fn release_version(self, mut client: Client, sign: Sign) -> Result<CIExit, Error> {
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
        log::trace!("Signing: {sign:?}");
        log::trace!("Update changelog flag: {}", self.update_changelog);

        if self.update_changelog {
            client.release_unreleased(&version)?;
            log::debug!("Changelog file name: {}", client.changelog_as_str());

            log::trace!(
                "{}",
                print_changelog(client.changelog_as_str(), client.line_limit())
            );

            let commit_message = "chore: update changelog for pr";

            client
                .commit_changed_files(sign, commit_message, &self.prefix, Some(&version))
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

        Ok(CIExit::Released)
    }
}

fn print_changelog(changelog_path: &str, mut line_limit: usize) -> String {
    let mut output = String::new();

    if let Ok(change_log) = fs::read_to_string(changelog_path) {
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
