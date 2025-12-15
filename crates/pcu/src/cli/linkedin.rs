mod commands;

use clap::Parser;
use config::Config;

use crate::{CIExit, Error};
use commands::Cmd;

/// Configuration for the LinkedIn command
#[derive(Debug, Parser, Clone)]
pub struct Linkedin {
    /// LinkedIn author URN (e.g. urn:li:organization:...)
    #[arg(long)]
    pub author_urn: Option<String>,
    /// Command to execute
    #[command(subcommand)]
    pub cmd: Cmd,
}

impl Linkedin {
    pub async fn run(&self) -> Result<CIExit, Error> {
        let settings = self.setup_settings()?;
        match self.cmd.clone() {
            Cmd::Share(mut share) => share.run(&settings, self.author_urn.clone()).await,
        }
    }

    fn setup_settings(&self) -> Result<Config, Error> {
        let settings = super::Commands::Linkedin(self.clone()).get_settings()?;
        Ok(settings)
    }
}
