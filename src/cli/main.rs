use std::{env, fs};

use ansi_term::Style;
use clap::Parser;
use config::Config;
use env_logger::Env;
use keep_a_changelog::ChangeKind;
use pcu_lib::{Client, Error, GitOps, MakeRelease, Sign, UpdateFromPr};

use color_eyre::Result;

const LOG_ENV_VAR: &str = "RUST_LOG";
const LOG_STYLE_ENV_VAR: &str = "RUST_LOG_STYLE";
const SIGNAL_HALT: &str = "halt";
const GITHUB_PAT: &str = "GITHUB_TOKEN";

mod cli;

use cli::{ClState, Cli, Commands, Commit, PullRequest, Push, Release};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let mut builder = get_logging(args.logging.log_level_filter());
    builder.init();
    log::debug!("Args: {args:?}");
    let sign = args.sign.unwrap_or_default();

    let cmd = args.command.clone();

    let res = match cmd {
        Commands::PullRequest(pr_args) => run_pull_request(sign, pr_args).await,
        Commands::Commit(commit_args) => run_commit(sign, commit_args).await,
        Commands::Push(push_args) => run_push(push_args).await,
        Commands::Release(rel_args) => run_release(sign, rel_args).await,
    };

    match res {
        Ok(state) => {
            match state {
                ClState::Updated => log::info!("Changelog updated!"),
                ClState::UnChanged => log::info!("Changelog not changed!"),
                ClState::Committed => log::info!("Changed files committed"),
                ClState::Pushed(s) => log::info!("{s}"),
                ClState::Released => log::info!("Created GitHub Release"),
            };
        }
        Err(e) => {
            return Err(e);
        }
    };

    Ok(())
}

async fn run_pull_request(sign: Sign, args: PullRequest) -> Result<ClState> {
    let branch = env::var("CIRCLE_BRANCH");
    log::trace!("Branch: {branch:?}");

    let branch = branch.unwrap_or("main".to_string());
    log::trace!("Branch: {branch:?}");

    if branch == "main" {
        log::info!("On the default branch, nothing to do here!");
        if args.early_exit {
            println!("{SIGNAL_HALT}");
        }

        return Ok(ClState::UnChanged);
    }

    let mut client = get_client(Commands::PullRequest(args.clone())).await?;

    log::info!(
        "On the `{}` branch, so time to get to work!",
        client.branch_or_main()
    );
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
            if args.early_exit {
                println!("{SIGNAL_HALT}");
            }
            return Ok(ClState::UnChanged);
        };
    } else if client.update_changelog()?.is_none() {
        return Ok(ClState::UnChanged);
    }

    log::debug!("Changelog file name: {}", client.changelog_as_str());

    log::trace!(
        "{}",
        print_changelog(client.changelog_as_str(), client.line_limit())
    );

    let commit_message = "chore: update changelog for pr";

    commit_changed_files(&client, sign, commit_message, None).await?;

    push_commited(&client, None, false).await?;

    Ok(ClState::Updated)
}

async fn run_commit(sign: Sign, args: Commit) -> Result<ClState> {
    let client = get_client(Commands::Commit(args.clone())).await?;

    commit_changed_files(&client, sign, args.commit_message(), args.tag_opt()).await?;

    Ok(ClState::Committed)
}

async fn commit_changed_files(
    client: &Client,
    sign: Sign,
    commit_message: &str,
    tag_opt: Option<&str>,
) -> Result<()> {
    log::debug!("{}", Style::new().bold().underline().paint("Check WorkDir"));

    let files_in_workdir = client.repo_files_not_staged()?;

    log::debug!("WorkDir files:\n\t{:?}", files_in_workdir);
    log::debug!("Staged files:\n\t{:?}", client.repo_files_staged()?);
    log::debug!("Branch status: {}", client.branch_status()?);

    log::info!("Stage the changes for commit");

    client.stage_files(files_in_workdir)?;

    log::debug!("{}", Style::new().bold().underline().paint("Check Staged"));
    log::debug!("WorkDir files:\n\t{:?}", client.repo_files_not_staged()?);

    let files_staged_for_commit = client.repo_files_staged()?;

    log::debug!("Staged files:\n\t{:?}", files_staged_for_commit);
    log::debug!("Branch status: {}", client.branch_status()?);

    log::info!("Commit the staged changes");

    client.commit_staged(sign, commit_message, tag_opt)?;

    log::debug!(
        "{}",
        Style::new().bold().underline().paint("Check Committed")
    );
    log::debug!("WorkDir files:\n\t{:?}", client.repo_files_not_staged()?);

    let files_staged_for_commit = client.repo_files_staged()?;

    log::debug!("Staged files:\n\t{:?}", files_staged_for_commit);
    log::debug!("Branch status: {}", client.branch_status()?);

    Ok(())
}

async fn run_push(args: Push) -> Result<ClState> {
    let client = get_client(Commands::Push(args.clone())).await?;

    push_commited(&client, args.tag_opt(), args.no_push).await?;

    if !args.no_push {
        Ok(ClState::Pushed(
            "Changed files committed and pushed to remote repsitory.".to_string(),
        ))
    } else {
        Ok(ClState::Pushed(
            "Changed files committed and push dry run completed for logging.".to_string(),
        ))
    }
}

async fn push_commited(client: &Client, tag_opt: Option<&str>, no_push: bool) -> Result<()> {
    log::info!("Push the commit");

    client.push_commit(tag_opt, no_push)?;
    log::debug!("{}", Style::new().bold().underline().paint("Check Push"));
    log::debug!("Branch status: {}", client.branch_status()?);

    Ok(())
}

async fn run_release(sign: Sign, args: Release) -> Result<ClState> {
    let mut client = get_client(Commands::Release(args.clone())).await?;

    let version = args.semver;

    log::trace!("Running release {version}");
    log::trace!(
        "PR ID: {} - Owner: {} - Repo: {}",
        client.pr_number(),
        client.owner(),
        client.repo()
    );
    log::trace!("Signing: {:?}", sign);
    log::trace!("Update changelog flag: {}", args.update_changelog);

    if args.update_changelog {
        client.release_unreleased(&version)?;
        log::debug!("Changelog file name: {}", client.changelog_as_str());

        log::trace!(
            "{}",
            print_changelog(client.changelog_as_str(), client.line_limit())
        );

        let commit_message = "chore: update changelog for pr";

        commit_changed_files(&client, sign, commit_message, None).await?;

        push_commited(&client, None, false).await?;
    }

    client.make_release(&version).await?;

    Ok(ClState::Released)
}

fn print_changelog(changelog_path: &str, mut line_limit: usize) -> String {
    let mut output = String::new();

    if let Ok(change_log) = fs::read_to_string(changelog_path) {
        let mut line_count = 0;
        if line_limit == 0 {
            line_limit = change_log.lines().count();
        };

        output.push_str("\n*****Changelog*****:\n----------------------------");
        for line in change_log.lines() {
            output.push_str(format!("{line}\n").as_str());
            line_count += 1;
            if line_count >= line_limit {
                break;
            }
        }
        output.push_str("----------------------------\n");
    };

    output
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

async fn get_client(cmd: Commands) -> Result<Client, Error> {
    let settings = get_settings(cmd)?;
    let client = Client::new_with(settings).await?;

    Ok(client)
}

fn get_settings(cmd: Commands) -> Result<Config, Error> {
    let mut settings = Config::builder()
        // Set defaults for CircleCI
        .set_default("log", "CHANGELOG.md")?
        .set_default("branch", "CIRCLE_BRANCH")?
        .set_default("default_branch", "main")?
        .set_default("pull_request", "CIRCLE_PULL_REQUEST")?
        .set_default("username", "CIRCLE_PROJECT_USERNAME")?
        .set_default("reponame", "CIRCLE_PROJECT_REPONAME")?
        .set_default("commit_message", "chore: update changelog")?
        .set_default("dev_platform", "https://github.com/")?
        .set_default("version_prefix", "v")?
        // Add in settings from pcu.toml if it exists
        .add_source(config::File::with_name("pcu.toml").required(false))
        // Add in settings from the environment (with a prefix of PCU)
        .add_source(config::Environment::with_prefix("PCU"));

    settings = match cmd {
        Commands::PullRequest(_) => settings
            .set_override("commit_message", "chore: update changelog for pr")?
            .set_override("command", "pull-request")?,
        Commands::Release(_) => settings
            .set_override("commit_message", "chore: update changelog for release")?
            .set_override("command", "release")?,
        Commands::Commit(_) => settings
            .set_override("commit_message", "chore: adding changed files")?
            .set_override("command", "commit")?,
        Commands::Push(_) => settings
            .set_override("commit_message", "chore: update changelog for release")?
            .set_override("command", "push")?,
    };

    settings = if let Ok(pat) = env::var(GITHUB_PAT) {
        settings.set_override("pat", pat.to_string())?
    } else {
        settings
    };

    match settings.build() {
        Ok(settings) => Ok(settings),
        Err(e) => {
            log::error!("Error: {e}");
            Err(e.into())
        }
    }
}
