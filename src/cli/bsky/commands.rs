use std::fmt::Display;

use clap::Subcommand;

#[derive(Debug, Subcommand, Clone)]
pub enum Cmd {
    Draft,
    Post,
}

impl Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::Draft => write!(f, "draft"),
            Cmd::Post => write!(f, "post"),
        }
    }
}
