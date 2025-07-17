use clap::Parser;
use env_logger::Env;

use color_eyre::Result;

const LOG_ENV_VAR: &str = "RUST_LOG";
const LOG_STYLE_ENV_VAR: &str = "RUST_LOG_STYLE";

use pcu::{CIExit, Cli, Commands};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let mut builder = get_logging(&args.logging.log_level_filter());
    builder.init();
    get_tracing(args.logging.log_level_filter());
    log::debug!("Args: {args:?}");
    let sign = args.sign.unwrap_or_default();

    let cmd = args.command.clone();

    let res = match cmd {
        Commands::Pr(pr_args) => pr_args.run_pull_request(sign).await,
        Commands::Commit(commit_args) => commit_args.run_commit(sign).await,
        Commands::Push(push_args) => push_args.run_push().await,
        Commands::Label(label_args) => label_args.run_label().await,
        Commands::Release(rel_args) => rel_args.run_release(sign).await,
        Commands::Bsky(bsky_args) => bsky_args.run().await,
    };

    match res {
        Ok(state) => {
            match state {
                CIExit::Updated => log::info!("Changelog updated!"),
                CIExit::UnChanged => log::info!("Changelog not changed!"),
                CIExit::Committed => log::info!("Changed files committed"),
                CIExit::Pushed(s) => log::info!("{s}"),
                CIExit::Released => log::info!("Created GitHub Release"),
                CIExit::Label(pr) => log::info!("Rebased PR request #{pr}"),
                CIExit::NoLabel => log::info!("No label required"),
                CIExit::DraftedForBluesky => log::info!("Drafted for Bluesky"),
                CIExit::PostedToBluesky => log::info!("Posted to Bluesky"),
                CIExit::NoFilesToProcess => log::info!("No files to process"),
            };
            Ok(())
        }
        Err(e) => {
            log::error!("Error: {e}");
            Err(e.into())
        }
    }
}
fn get_logging(level: &log::LevelFilter) -> env_logger::Builder {
    let env = Env::new()
        .filter_or(LOG_ENV_VAR, "off")
        .write_style_or(LOG_STYLE_ENV_VAR, "auto");

    let mut builder = env_logger::Builder::from_env(env);

    builder.filter_module("pcu::cli", *level);
    builder.filter_module("pcu::client", *level);
    builder.filter_module("pcu::client::graphql", *level);
    builder.filter_module("pcu::client::graphql::get_tag", *level);
    builder.filter_module("pcu::ops", *level);
    builder.filter_module("pcu::utilities", *level);
    builder.filter_module("pcu", *level);
    builder.format_timestamp_secs();

    builder
}

fn get_tracing(level: log::LevelFilter) {
    let filter_pcu = EnvFilter::from(format!("pcu={level}"));
    let filter_pcu_lib = EnvFilter::from(format!("pcu_lib={level}"));

    let log_subscriber = tracing_subscriber::FmtSubscriber::builder()
        .pretty()
        .with_env_filter(filter_pcu)
        .with_env_filter(filter_pcu_lib)
        .finish();

    let _ = tracing::subscriber::set_global_default(log_subscriber)
        .map_err(|_| eprintln!("Unable to set global default subscriber!"));

    log::info!("Initialised logging to console at {level}");
}
