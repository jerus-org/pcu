mod commands;

use std::fs;

use clap::Parser;
use commands::Cmd;
use config::Config;

use super::Commands;
use crate::{CIExit, Client, Error};

/// Configuration for the LinkedIn command
#[derive(Debug, Parser, Clone)]
pub struct Linkedin {
    /// LinkedIn author URN (e.g. urn:li:organization:...)
    #[arg(long)]
    pub author_urn: Option<String>,
    /// application id for access to the repository
    #[arg(short, long)]
    pub id: Option<String>,
    /// file with application private key for access to the repository
    #[arg(short, long)]
    pub pk: Option<String>,
    /// Command to execute
    #[command(subcommand)]
    pub cmd: Cmd,
}

impl Linkedin {
    pub async fn run(&self) -> Result<CIExit, Error> {
        match self.cmd.clone() {
            Cmd::Draft(mut draft) => {
                let (client, settings) = self.setup_client().await?;
                draft.run(&client, &settings).await
            }
            Cmd::Post(post) => {
                let (client, settings) = self.setup_client().await?;
                post.run(&client, &settings).await
            }
            Cmd::Share(mut share) => {
                let settings = self.setup_settings()?;
                share.run(&settings, self.author_urn.clone()).await
            }
        }
    }

    async fn setup_client(&self) -> Result<(Client, Config), Error> {
        let settings = Commands::Linkedin(self.clone()).get_settings()?;
        let mut builder = Config::builder();
        builder = builder.add_source(settings);

        if let Some(appid) = self.id.as_deref() {
            builder = builder.set_override("PCU_APP_ID", appid)?;
        }
        if let Some(app_private_key) = &self.pk {
            let app_private_key = fs::read_to_string(app_private_key)?;
            builder = builder.set_override("PCU_PRIVATE_KEY", app_private_key)?;
        }
        let settings = builder.build()?;
        let client = Client::new_with(&settings).await?;
        Ok((client, settings))
    }

    fn setup_settings(&self) -> Result<Config, Error> {
        let settings = Commands::Linkedin(self.clone()).get_settings()?;
        Ok(settings)
    }
}
