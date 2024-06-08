use std::fs;

use clap::{Parser, ValueEnum};
use env_logger::Env;
use keep_a_changelog::ChangeKind;
use pcu_lib::{Client, Error};

use eyre::Result;

const LOG_ENV_VAR: &str = "PCU_LOG";
const LOG_STYLE_ENV_VAR: &str = "PCU_LOG_STYLE";

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
    sign: Option<Sign>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let mut builder = get_logging(args.logging.log_level_filter());
    builder.init();

    let client = match Client::new().await {
        Ok(client) => client,
        Err(e) => match e {
            Error::EnvVarPullRequestNotFound => {
                log::info!("I am on the main branch, so nothing more to do!");
                return Ok(());
            }
            _ => return Err(e.into()),
        },
    };

    log::info!(
        "On the `{}` branch, so time to get to work!",
        client.branch()
    );

    let sign = if let Some(sign) = args.sign {
        sign
    } else {
        Sign::default()
    };

    match run_update(client, sign).await {
        Ok(_) => log::info!("Changelog updated!"),
        Err(e) => log::error!("Error updating changelog: {e}"),
    };

    Ok(())
}

async fn run_update(mut client: Client, sign: Sign) -> Result<()> {
    log::debug!(
        "PR ID: {} - Owner: {} - Repo: {}",
        client.pr_number(),
        client.owner(),
        client.repo()
    );

    let title = client.title();

    log::debug!("Pull Request Title: {title}");

    client.create_entry()?;

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
            log::info!("Proposed addition to change log unreleased changes: In Section: `{section}` add the following entry: `{entry}`");
        } else {
            log::info!("No update required");
            return Ok(());
        };
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

    Ok(())
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
