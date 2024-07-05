use std::{collections::HashMap, fs};

use clap::{Parser, ValueEnum};
use config::Config;
use env_logger::Env;
use keep_a_changelog::ChangeKind;
use pcu_lib::{Client, Error};

use color_eyre::Result;

const LOG_ENV_VAR: &str = "RUST_LOG";
const LOG_STYLE_ENV_VAR: &str = "RUST_LOG_STYLE";
const SIGNAL_HALT: &str = "halt";

#[derive(ValueEnum, Debug, Default, Clone)]
enum Sign {
    #[default]
    Gpg,
    None,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(flatten)]
    logging: clap_verbosity_flag::Verbosity,
    #[clap(short, long)]
    /// Require the user to sign the update commit with their GPG key
    sign: Option<Sign>,
    /// Signal an early exit as the changelog is already updated
    #[clap(short, long, default_value_t = false)]
    early_exit: bool,
}

enum ClState {
    Updated,
    UnChanged,
}

#[tokio::main]
async fn main() -> Result<()> {
    let settings = get_settings()?;
    let args = Cli::parse();
    log::debug!("Args: {args:?}");
    let mut builder = get_logging(args.logging.log_level_filter());
    builder.init();

    log::trace!("Settings for github client: {settings:?}");
    let client = match Client::new_with(settings).await {
        Ok(client) => client,
        Err(e) => match e {
            Error::EnvVarPullRequestNotFound => {
                log::info!("On the main branch, so nothing more to do!");
                if args.early_exit {
                    println!("{SIGNAL_HALT}");
                }
                return Ok(());
            }
            _ => return Err(e.into()),
        },
    };

    log::info!(
        "On the `{}` branch, so time to get to work!",
        client.branch()
    );

    let sign = args.sign.unwrap_or_default();

    match run_update(client, sign).await {
        Ok(state) => {
            log::info!("Changelog updated!");
            if let ClState::UnChanged = state {
                if args.early_exit {
                    println!("{SIGNAL_HALT}");
                }
            }
        }
        Err(e) => {
            log::error!("Error updating changelog: {e}");
            return Err(e);
        }
    }

    Ok(())
}

async fn run_update(mut client: Client, sign: Sign) -> Result<ClState> {
    log::debug!(
        "PR ID: {} - Owner: {} - Repo: {}",
        client.pr_number(),
        client.owner(),
        client.repo()
    );

    let title = client.title();

    log::debug!("Pull Request Title: {title}");

    client.create_entry()?;

    log::debug!("Proposed entry: {:?}", client.entry());

    if log::log_enabled!(log::Level::Info) {
        if let Some((section, entry)) = client.update_changelog()? {
            let section = match section {
                ChangeKind::Added => "Added",
                ChangeKind::Changed => "Changed",
                ChangeKind::Deprecated => "Deprecated",
                ChangeKind::Fixed => "Fixed",
                ChangeKind::Removed => "Removed",
                ChangeKind::Security => "Security",
            };
            log::info!("Amendment: In section `{section}`, adding `{entry}`");
        } else {
            log::info!("No update required");
            return Ok(ClState::UnChanged);
        };
    } else if client.update_changelog()?.is_none() {
        return Ok(ClState::UnChanged);
    }

    log::debug!("Changelog file name: {}", client.changelog());

    if log::log_enabled!(log::Level::Trace) {
        print_changelog(client.changelog());
    };

    let report = client.repo_status()?;
    log::debug!("Before commit:Repo state: {report}");
    log::debug!("before commit:Branch status: {}", client.branch_status()?);

    match sign {
        Sign::Gpg => {
            client.commit_changelog_gpg()?;
        }
        Sign::None => {
            client.commit_changelog()?;
        }
    }

    log::debug!("After commit: Repo state: {}", client.repo_status()?);
    log::debug!("After commit: Branch status: {}", client.branch_status()?);

    client.push_changelog()?;
    log::debug!("After push: Branch status: {}", client.branch_status()?);

    Ok(ClState::Updated)
}

fn print_changelog(changelog_path: &str) {
    if let Ok(change_log) = fs::read_to_string(changelog_path) {
        println!("\nChangelog:\n");
        println!("----------------------------",);
        for line in change_log.lines() {
            println!("{line}");
        }
        println!("----------------------------\n",);
    };
}

fn get_logging(level: log::LevelFilter) -> env_logger::Builder {
    let env = Env::new()
        .filter_or(LOG_ENV_VAR, "off")
        .write_style_or(LOG_STYLE_ENV_VAR, "auto");

    let mut builder = env_logger::Builder::from_env(env);

    builder.filter_module("pcu", level);
    builder.filter_module("pcu_lib", level);
    builder.format_timestamp_secs();

    builder
}

fn get_settings() -> Result<Config, Error> {
    let settings = Config::builder()
        // Set defaults for CircleCI
        .set_default("log", "CHANGELOG.md")?
        .set_default("branch", "CIRCLE_BRANCH")?
        .set_default("pull_request", "CIRCLE_PULL_REQUEST")?
        .set_default("username", "CIRCLE_PROJECT_USERNAME")?
        .set_default("reponame", "CIRCLE_PROJECT_REPONAME")?
        .set_default("commit_message", "chore: update changelog")?
        // Add in settings from pcu.toml if it exists
        .add_source(config::File::with_name("pcu.toml").required(false))
        // Add in settings from the environment (with a prefix of PCU)
        .add_source(config::Environment::with_prefix("PCU"));

    match settings.build() {
        Ok(settings) => {
            // Print out our settings (as a HashMap)
            log::trace!(
                "{:?}",
                settings
                    .clone()
                    .try_deserialize::<HashMap<String, String>>()
                    .unwrap()
            );
            Ok(settings)
        }
        Err(e) => {
            println!("Error: {e}");
            Err(e.into())
        }
    }
}
