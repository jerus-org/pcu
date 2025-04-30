use std::{fs, path::Path};

use crate::{Client, Error, GitOps, MakeRelease, Sign, Workspace};

use super::{CIExit, Commands};
mod mode;

use clap::Parser;
use mode::Mode;
use owo_colors::{OwoColorize, Style};

#[derive(Debug, Parser, Clone)]
pub struct Release {
    /// Semantic version number for the release
    #[arg(short, long)]
    pub semver: Option<String>,
    /// Update the changelog by renaming the unreleased section with the version number
    #[arg(short, long, default_value_t = false)]
    pub update_changelog: bool,
    /// Prefix for the version tag
    #[clap(short, long, default_value_t = String::from("v"))]
    pub prefix: String,
    /// Specific package to release
    #[clap(short = 'k', long)]
    pub package: Option<String>,
    #[command(subcommand)]
    pub mode: Mode,
}

impl Release {
    pub async fn run_release(self, sign: Sign) -> Result<CIExit, Error> {
        let client = Commands::Release(self.clone()).get_client().await?;

        match self.mode {
            Mode::Version => self.release_semver(client, sign).await,
            Mode::Package => self.release_package(client).await,
            Mode::Workspace => self.release_workspace(client).await,
            Mode::Current => self.release_current(client).await,
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

    async fn release_package(self, client: Client) -> Result<CIExit, Error> {
        log::info!("Running release for package");

        let Some(rel_package) = self.package else {
            log::error!("No package specified");
            return Err(Error::NoPackageSpecified);
        };
        log::info!("Running release for package: {}", rel_package);

        let path = Path::new("./Cargo.toml");
        let workspace = Workspace::new(path).unwrap();

        let packages = workspace.packages();

        if let Some(packages) = packages {
            for package in packages {
                log::debug!("Found workspace package: {}", package.name);
                if package.name != rel_package {
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

        let Some(rel_package) = self.package else {
            log::error!("No package specified");
            return Err(Error::NoPackageSpecified);
        };
        log::info!("Running release for package: {}", rel_package);

        let path = Path::new("./Cargo.toml");
        let workspace = Workspace::new(path).unwrap();

        let packages = workspace.packages();

        if let Some(packages) = packages {
            for package in packages {
                log::debug!("Found workspace package: {}", package.name);
                if package.name != rel_package {
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

    async fn release_semver(self, mut client: Client, sign: Sign) -> Result<CIExit, Error> {
        if self.semver.is_none() {
            log::error!("Semver is required for release");
            return Err(Error::MissingSemver);
        }
        log::info!("Running release for semver (requires semver to be set)");
        let Some(version) = self.semver else {
            log::error!("Semver required to update changelog");
            return Ok(CIExit::UnChanged);
        };

        log::trace!("Running release {version}");
        log::trace!(
            "PR ID: {} - Owner: {} - Repo: {}",
            client.pr_number(),
            client.owner(),
            client.repo()
        );
        log::trace!("Signing: {:?}", sign);
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
            log::debug!("Using bot user name: {}", bot_user_name);

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
