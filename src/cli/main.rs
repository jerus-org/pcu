use std::{env, fs, path::Path};

use clap::Parser;
use config::Config;
use env_logger::Env;
use keep_a_changelog::ChangeKind;
use owo_colors::{OwoColorize, Style};
use pcu_lib::{Client, Error, GitOps, MakeRelease, Sign, UpdateFromPr, Workspace};

use color_eyre::Result;

const LOG_ENV_VAR: &str = "RUST_LOG";
const LOG_STYLE_ENV_VAR: &str = "RUST_LOG_STYLE";
const SIGNAL_HALT: &str = "halt";
const GITHUB_PAT: &str = "GITHUB_TOKEN";

mod cli;

use cli::{Bsky, CIExit, Cli, Commands, Commit, Label, Pr, Push, Release};
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
        Commands::Pr(pr_args) => run_pull_request(sign, pr_args).await,
        Commands::Commit(commit_args) => run_commit(sign, commit_args).await,
        Commands::Push(push_args) => run_push(push_args).await,
        Commands::Label(label_args) => run_label(label_args).await,
        Commands::Release(rel_args) => run_release(sign, rel_args).await,
        Commands::Bsky(rel_args) => run_bsky(rel_args).await,
    };

    match res {
        Ok(state) => {
            match state {
                CIExit::Updated => log::info!("Changelog updated!"),
                CIExit::UnChanged => log::info!("Changelog not changed!"),
                CIExit::Committed => log::info!("Changed files committed"),
                CIExit::Pushed(s) => log::info!("{s}"),
                CIExit::Released => log::info!("Created GitHub Release"),
                CIExit::Label(pr) => log::info!("Rebased PR request #{}", pr),
                CIExit::NoLabel => log::info!("No label required"),
                CIExit::PostedToBluesky => log::info!("Posted to Bluesky"),
            };
        }
        Err(e) => {
            return Err(e);
        }
    };

    Ok(())
}

async fn run_pull_request(sign: Sign, args: Pr) -> Result<CIExit> {
    let branch = env::var("CIRCLE_BRANCH");
    let branch = branch.unwrap_or("main".to_string());
    log::trace!("Branch: {branch:?}");

    if branch == "main" {
        log::info!("On the default branch, nothing to do here!");
        if args.early_exit {
            println!("{SIGNAL_HALT}");
        }

        return Ok(CIExit::UnChanged);
    }

    log::trace!("*** Get Client ***");
    let mut client = get_client(Commands::Pr(args.clone())).await?;

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

    log::trace!("Full client: {:#?}", client);
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
            return Ok(CIExit::UnChanged);
        };
    } else if client.update_changelog()?.is_none() {
        return Ok(CIExit::UnChanged);
    }

    log::debug!("Changelog file name: {}", client.changelog_as_str());

    log::trace!(
        "{}",
        print_changelog(client.changelog_as_str(), client.line_limit())
    );

    let commit_message = "chore: update changelog for pr";

    commit_changed_files(&client, sign, commit_message, &args.prefix, None).await?;

    let res = push_committed(&client, &args.prefix, None, false).await;
    match res {
        Ok(()) => Ok(CIExit::Updated),
        Err(e) => {
            if args.allow_push_fail
                && e.to_string()
                    .contains("cannot push non-fastforwardable reference")
            {
                log::info!("Cannot psh non-fastforwardable reference, presuming change made already in parallel job.");
                Ok(CIExit::UnChanged)
            } else {
                Err(e)
            }
        }
    }
}

async fn run_commit(sign: Sign, args: Commit) -> Result<CIExit> {
    let client = get_client(Commands::Commit(args.clone())).await?;

    commit_changed_files(
        &client,
        sign,
        args.commit_message(),
        &args.prefix,
        args.tag_opt(),
    )
    .await?;

    Ok(CIExit::Committed)
}

async fn commit_changed_files(
    client: &Client,
    sign: Sign,
    commit_message: &str,
    prefix: &str,
    tag_opt: Option<&str>,
) -> Result<()> {
    let hdr_style = Style::new().bold().underline();
    log::debug!("{}", "Check WorkDir".style(hdr_style));

    let files_in_workdir = client.repo_files_not_staged()?;

    log::debug!("WorkDir files:\n\t{:?}", files_in_workdir);
    log::debug!("Staged files:\n\t{:?}", client.repo_files_staged()?);
    log::debug!("Branch status: {}", client.branch_status()?);

    log::info!("Stage the changes for commit");

    client.stage_files(files_in_workdir)?;

    log::debug!("{}", "Check Staged".style(hdr_style));
    log::debug!("WorkDir files:\n\t{:?}", client.repo_files_not_staged()?);

    let files_staged_for_commit = client.repo_files_staged()?;

    log::debug!("Staged files:\n\t{:?}", files_staged_for_commit);
    log::debug!("Branch status: {}", client.branch_status()?);

    log::info!("Commit the staged changes");

    client.commit_staged(sign, commit_message, prefix, tag_opt)?;

    log::debug!("{}", "Check Committed".style(hdr_style));
    log::debug!("WorkDir files:\n\t{:?}", client.repo_files_not_staged()?);

    let files_staged_for_commit = client.repo_files_staged()?;

    log::debug!("Staged files:\n\t{:?}", files_staged_for_commit);
    log::debug!("Branch status: {}", client.branch_status()?);

    Ok(())
}

async fn run_push(args: Push) -> Result<CIExit> {
    let client = get_client(Commands::Push(args.clone())).await?;

    push_committed(&client, &args.prefix, args.tag_opt(), args.no_push).await?;

    if !args.no_push {
        Ok(CIExit::Pushed(
            "Changed files committed and pushed to remote repository.".to_string(),
        ))
    } else {
        Ok(CIExit::Pushed(
            "Changed files committed and push dry run completed for logging.".to_string(),
        ))
    }
}

async fn push_committed(
    client: &Client,
    prefix: &str,
    tag_opt: Option<&str>,
    no_push: bool,
) -> Result<()> {
    log::info!("Push the commit");
    log::trace!("tag_opt: {tag_opt:?} and no_push: {no_push}");

    client.push_commit(prefix, tag_opt, no_push)?;
    let hdr_style = Style::new().bold().underline();
    log::debug!("{}", "Check Push".style(hdr_style));
    log::debug!("Branch status: {}", client.branch_status()?);

    Ok(())
}

async fn run_label(args: Label) -> Result<CIExit> {
    let client = get_client(Commands::Label(args.clone())).await?;

    let pr_number = client
        .label_next_pr(args.author(), args.label(), args.desc(), args.colour())
        .await?;

    if let Some(pr_number) = pr_number {
        Ok(CIExit::Label(pr_number))
    } else {
        Ok(CIExit::NoLabel)
    }
}

async fn run_release(sign: Sign, args: Release) -> Result<CIExit> {
    let client = get_client(Commands::Release(args.clone())).await?;

    if args.workspace {
        log::info!("Running release for workspace");
        return release_workspace(client, args).await;
    };

    if args.package.is_some() {
        return release_package(client, args).await;
    }

    release_semver(client, args, sign).await
}

async fn run_bsky(args: Bsky) -> Result<CIExit> {
    // TODO: Identify blogs that have changed
    // TODO: For each blog, extract the title, description, and tags
    // TODO: For each blog, create a Bluesky post
    let _client = get_client(Commands::Bsky(args.clone())).await?;

    Ok(CIExit::PostedToBluesky)
}

async fn release_semver(mut client: Client, args: Release, sign: Sign) -> Result<CIExit> {
    let Some(version) = args.semver else {
        log::error!("Semver required to update changelog");
        return Ok(CIExit::UnChanged);
    };

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

        commit_changed_files(&client, sign, commit_message, &args.prefix, Some(&version)).await?;

        push_committed(&client, &args.prefix, Some(&version), false).await?;
    }

    client.make_release(&args.prefix, &version).await?;

    Ok(CIExit::Released)
}

async fn release_workspace(client: Client, args: Release) -> Result<CIExit> {
    let path = Path::new("./Cargo.toml");
    let workspace = Workspace::new(path).unwrap();

    let packages = workspace.packages();

    if let Some(packages) = packages {
        for package in packages {
            let prefix = format!("{}-{}", package.name, args.prefix);
            let version = package.version;
            let tag = format!("{prefix}{version}");
            if !client.tag_exists(&tag) {
                log::error!("Tag does not exist: {tag}");
            } else {
                log::info!("Tag already exists: {tag}, attempt to make release");
                client.make_release(&prefix, &version).await?;
            }
        }
    }
    Ok(CIExit::Released)
}

async fn release_package(client: Client, args: Release) -> Result<CIExit> {
    let rel_package = args.package.unwrap();
    log::info!("Running release for package: {}", rel_package);

    let path = Path::new("./Cargo.toml");
    let workspace = Workspace::new(path).unwrap();

    let packages = workspace.packages();

    if let Some(packages) = packages {
        for package in packages {
            log::debug!("Found workspace package: {}", package.name);
            if package.name != rel_package {
                continue;
            }
            let prefix = format!("{}-{}", package.name, args.prefix);
            let version = package.version;
            let tag = format!("{prefix}{version}");
            if !client.tag_exists(&tag) {
                log::error!("Tag does not exist: {tag}");
            } else {
                log::info!("Tag already exists: {tag}, attempt to make release");
                client.make_release(&prefix, &version).await?;
            }
            break;
        }
    }
    Ok(CIExit::Released)
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

fn get_logging(level: &log::LevelFilter) -> env_logger::Builder {
    let env = Env::new()
        .filter_or(LOG_ENV_VAR, "off")
        .write_style_or(LOG_STYLE_ENV_VAR, "auto");

    let mut builder = env_logger::Builder::from_env(env);

    builder.filter_module("pcu", *level);
    builder.filter_module("pcu_lib", *level);
    builder.format_timestamp_secs();

    builder
}

async fn get_client(cmd: Commands) -> Result<Client, Error> {
    let settings = get_settings(cmd)?;
    let client = Client::new_with(settings).await?;

    Ok(client)
}

fn get_tracing(level: log::LevelFilter) {
    let filter_pcu = EnvFilter::from(format!("pcu={}", level));
    let filter_pcu_lib = EnvFilter::from(format!("pcu_lib={}", level));

    let log_subscriber = tracing_subscriber::FmtSubscriber::builder()
        .pretty()
        .with_env_filter(filter_pcu)
        .with_env_filter(filter_pcu_lib)
        .finish();

    let _ = tracing::subscriber::set_global_default(log_subscriber)
        .map_err(|_| eprintln!("Unable to set global default subscriber!"));

    tracing::info!("Initialised logging to console at {level}");
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
        Commands::Pr(_) => settings
            .set_override("commit_message", "chore: update changelog for pr")?
            .set_override("command", "pr")?,
        Commands::Release(_) => settings
            .set_override("commit_message", "chore: update changelog for release")?
            .set_override("command", "release")?,
        Commands::Commit(_) => settings
            .set_override("commit_message", "chore: adding changed files")?
            .set_override("command", "commit")?,
        Commands::Push(_) => settings
            .set_override("commit_message", "chore: update changelog for release")?
            .set_override("command", "push")?,
        Commands::Label(_) => settings
            .set_override("commit_message", "chore: update changelog for release")?
            .set_override("command", "label")?,
        Commands::Bsky(_) => settings.set_override("command", "bsky")?,
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
