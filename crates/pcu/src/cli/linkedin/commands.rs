mod cmd_draft;
mod cmd_post;
mod cmd_share;

use std::fmt::Display;

use clap::Subcommand;
pub(super) use cmd_draft::CmdDraft;
pub(super) use cmd_post::CmdPost;
use cmd_share::CmdShare;

#[derive(Debug, Subcommand, Clone)]
pub enum Cmd {
    /// Draft LinkedIn posts from blog post frontmatter
    Draft(CmdDraft),
    /// Publish staged LinkedIn draft files
    Post(CmdPost),
    /// Share a LinkedIn post
    Share(CmdShare),
}

impl Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::Draft(_) => write!(f, "draft"),
            Cmd::Post(_) => write!(f, "post"),
            Cmd::Share(_) => write!(f, "share"),
        }
    }
}
