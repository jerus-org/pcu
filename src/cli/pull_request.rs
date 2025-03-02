use std::env;

use crate::{
    cli::{commit_changed_files, Commands},
    Sign, UpdateFromPr,
};

use super::{CIExit, Pr};

use color_eyre::Result;
use keep_a_changelog::ChangeKind;

const SIGNAL_HALT: &str = "halt";

pub async fn run_pull_request(sign: Sign, args: Pr) -> Result<CIExit> {
    let branch = env::var("CIRCLE_BRANCH");
    let branch = branch.unwrap_or("main".to_string());
    log::trace!("Branch: {branch:?}");

    if branch == "main" {
        log::info!("On the default branch, nothing to do here!");
        if args.early_exit {
            println!("{SIGNAL_HALT}");
        }

        return Ok(CIExit::UnChanged);
    }

    log::trace!("*** Get Client ***");
    let mut client = crate::cli::get_client(Commands::Pr(args.clone())).await?;

    log::info!(
        "On the `{}` branch, so time to get to work!",
        client.branch_or_main()
    );
    log::debug!(
        "PR ID: {} - Owner: {} - Repo: {}",
        client.pr_number(),
        client.owner(),
        client.repo()
    );

    log::trace!("Full client: {:#?}", client);
    let title = client.title();

    log::debug!("Pull Request Title: {title}");

    client.create_entry()?;

    log::debug!("Proposed entry: {:?}", client.entry());

    if log::log_enabled!(log::Level::Info) {
        if let Some((section, entry)) = client.update_changelog()? {
            let section = match section {
                ChangeKind::Added => "Added",
                ChangeKind::Changed => "Changed",
                ChangeKind::Deprecated => "Deprecated",
                ChangeKind::Fixed => "Fixed",
                ChangeKind::Removed => "Removed",
                ChangeKind::Security => "Security",
            };
            log::info!("Amendment: In section `{section}`, adding `{entry}`");
        } else {
            log::info!("No update required");
            if args.early_exit {
                println!("{SIGNAL_HALT}");
            }
            return Ok(CIExit::UnChanged);
        };
    } else if client.update_changelog()?.is_none() {
        return Ok(CIExit::UnChanged);
    }

    log::debug!("Changelog file name: {}", client.changelog_as_str());

    log::trace!(
        "{}",
        crate::cli::print_changelog(client.changelog_as_str(), client.line_limit())
    );

    let commit_message = "chore: update changelog for pr";

    commit_changed_files(&client, sign, commit_message, &args.prefix, None).await?;

    let res = crate::cli::push_committed(&client, &args.prefix, None, false).await;
    match res {
        Ok(()) => Ok(CIExit::Updated),
        Err(e) => {
            if args.allow_push_fail
                && e.to_string()
                    .contains("cannot push non-fastforwardable reference")
            {
                log::info!("Cannot psh non-fastforwardable reference, presuming change made already in parallel job.");
                Ok(CIExit::UnChanged)
            } else {
                Err(e)
            }
        }
    }
}
