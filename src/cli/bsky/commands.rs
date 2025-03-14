mod cmd_draft;
mod cmd_post;

use std::fmt::Display;

use clap::Subcommand;
use cmd_draft::CmdDraft;
use cmd_post::CmdPost;

#[derive(Debug, Subcommand, Clone)]
pub enum Cmd {
    Draft(CmdDraft),
    Post(CmdPost),
}

impl Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::Draft(_) => write!(f, "draft"),
            Cmd::Post(_) => write!(f, "post"),
        }
    }
}
