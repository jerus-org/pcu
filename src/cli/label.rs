use super::{CIExit, Commands, GitOps, Label};

use color_eyre::Result;

pub async fn run_label(args: Label) -> Result<CIExit> {
    let client = super::get_client(Commands::Label(args.clone())).await?;

    let pr_number = client
        .label_next_pr(args.author(), args.label(), args.desc(), args.colour())
        .await?;

    if let Some(pr_number) = pr_number {
        Ok(CIExit::Label(pr_number))
    } else {
        Ok(CIExit::NoLabel)
    }
}
