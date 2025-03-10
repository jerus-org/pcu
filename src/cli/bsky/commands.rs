use std::fmt::Display;

use clap::Subcommand;

#[derive(Debug, Subcommand, Clone)]
pub enum Cmd {
    Build,
    Post,
}

impl Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::Build => write!(f, "build"),
            Cmd::Post => write!(f, "post"),
        }
    }
}
