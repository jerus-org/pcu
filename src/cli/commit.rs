use crate::Sign;

use color_eyre::Result;

use super::{CIExit, Commands, Commit};

pub async fn run_commit(sign: Sign, args: Commit) -> Result<CIExit> {
    let client = super::get_client(Commands::Commit(args.clone())).await?;

    super::commit_changed_files(
        &client,
        sign,
        args.commit_message(),
        &args.prefix,
        args.tag_opt(),
    )
    .await?;

    Ok(CIExit::Committed)
}
