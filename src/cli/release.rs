use std::{fs, path::Path};

use crate::{Client, GitOps, MakeRelease, Sign, Workspace};

use super::{CIExit, Commands, Release};

use color_eyre::Result;

pub async fn run_release(sign: Sign, args: Release) -> Result<CIExit> {
    let client = super::get_client(Commands::Release(args.clone())).await?;

    if args.workspace {
        log::info!("Running release for workspace");
        return release_workspace(client, args).await;
    };

    if args.package.is_some() {
        return release_package(client, args).await;
    }

    release_semver(client, args, sign).await
}

async fn release_workspace(client: Client, args: Release) -> Result<CIExit> {
    let path = Path::new("./Cargo.toml");
    let workspace = Workspace::new(path).unwrap();

    let packages = workspace.packages();

    if let Some(packages) = packages {
        for package in packages {
            let prefix = format!("{}-{}", package.name, args.prefix);
            let version = package.version;
            let tag = format!("{prefix}{version}");
            if !client.tag_exists(&tag) {
                log::error!("Tag does not exist: {tag}");
            } else {
                log::info!("Tag already exists: {tag}, attempt to make release");
                client.make_release(&prefix, &version).await?;
            }
        }
    }
    Ok(CIExit::Released)
}

async fn release_package(client: Client, args: Release) -> Result<CIExit> {
    let rel_package = args.package.unwrap();
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
            let prefix = format!("{}-{}", package.name, args.prefix);
            let version = package.version;
            let tag = format!("{prefix}{version}");
            if !client.tag_exists(&tag) {
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

async fn release_semver(mut client: Client, args: Release, sign: Sign) -> Result<CIExit> {
    let Some(version) = args.semver else {
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
    log::trace!("Update changelog flag: {}", args.update_changelog);

    if args.update_changelog {
        client.release_unreleased(&version)?;
        log::debug!("Changelog file name: {}", client.changelog_as_str());

        log::trace!(
            "{}",
            print_changelog(client.changelog_as_str(), client.line_limit())
        );

        let commit_message = "chore: update changelog for pr";

        crate::cli::commit_changed_files(
            &client,
            sign,
            commit_message,
            &args.prefix,
            Some(&version),
        )
        .await?;

        crate::cli::push_committed(&client, &args.prefix, Some(&version), false).await?;
    }

    client.make_release(&args.prefix, &version).await?;

    Ok(CIExit::Released)
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
