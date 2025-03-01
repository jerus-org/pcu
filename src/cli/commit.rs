use super::Commit;
use super::{CIExit, Commands};
use crate::{Client, GitOps, Sign};
use owo_colors::{OwoColorize, Style};

use color_eyre::Result;

pub async fn run_commit(sign: Sign, args: Commit) -> Result<CIExit> {
    let client = super::get_client(Commands::Commit(args.clone())).await?;

    commit_changed_files(
        &client,
        sign,
        args.commit_message(),
        &args.prefix,
        args.tag_opt(),
    )
    .await?;

    Ok(CIExit::Committed)
}

async fn commit_changed_files(
    client: &Client,
    sign: Sign,
    commit_message: &str,
    prefix: &str,
    tag_opt: Option<&str>,
) -> Result<()> {
    let hdr_style = Style::new().bold().underline();
    log::debug!("{}", "Check WorkDir".style(hdr_style));

    let files_in_workdir = client.repo_files_not_staged()?;

    log::debug!("WorkDir files:\n\t{:?}", files_in_workdir);
    log::debug!("Staged files:\n\t{:?}", client.repo_files_staged()?);
    log::debug!("Branch status: {}", client.branch_status()?);

    log::info!("Stage the changes for commit");

    client.stage_files(files_in_workdir)?;

    log::debug!("{}", "Check Staged".style(hdr_style));
    log::debug!("WorkDir files:\n\t{:?}", client.repo_files_not_staged()?);

    let files_staged_for_commit = client.repo_files_staged()?;

    log::debug!("Staged files:\n\t{:?}", files_staged_for_commit);
    log::debug!("Branch status: {}", client.branch_status()?);

    log::info!("Commit the staged changes");

    client.commit_staged(sign, commit_message, prefix, tag_opt)?;

    log::debug!("{}", "Check Committed".style(hdr_style));
    log::debug!("WorkDir files:\n\t{:?}", client.repo_files_not_staged()?);

    let files_staged_for_commit = client.repo_files_staged()?;

    log::debug!("Staged files:\n\t{:?}", files_staged_for_commit);
    log::debug!("Branch status: {}", client.branch_status()?);

    Ok(())
}
