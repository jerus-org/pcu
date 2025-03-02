use crate::Client;

use super::{CIExit, Commands, GitOps, Push};

use color_eyre::Result;
use owo_colors::{OwoColorize, Style};

pub async fn run_push(args: Push) -> Result<CIExit> {
    let client = super::get_client(Commands::Push(args.clone())).await?;

    push_committed(&client, &args.prefix, args.tag_opt(), args.no_push).await?;

    if !args.no_push {
        Ok(CIExit::Pushed(
            "Changed files committed and pushed to remote repository.".to_string(),
        ))
    } else {
        Ok(CIExit::Pushed(
            "Changed files committed and push dry run completed for logging.".to_string(),
        ))
    }
}

async fn push_committed(
    client: &Client,
    prefix: &str,
    tag_opt: Option<&str>,
    no_push: bool,
) -> Result<()> {
    log::info!("Push the commit");
    log::trace!("tag_opt: {tag_opt:?} and no_push: {no_push}");

    client.push_commit(prefix, tag_opt, no_push)?;
    let hdr_style = Style::new().bold().underline();
    log::debug!("{}", "Check Push".style(hdr_style));
    log::debug!("Branch status: {}", client.branch_status()?);

    Ok(())
}
