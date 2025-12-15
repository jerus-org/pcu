mod cmd_share;

use std::fmt::Display;

use clap::Subcommand;
use cmd_share::CmdShare;

#[derive(Debug, Subcommand, Clone)]
pub enum Cmd {
    /// Share a LinkedIn post
    Share(CmdShare),
}

impl Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::Share(_) => write!(f, "share"),
        }
    }
}
