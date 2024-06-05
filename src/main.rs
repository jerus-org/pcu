use std::{fs, path::Path};

use git2::Repository;
use keep_a_changelog::ChangeKind;
use pcu_lib::{Client, Error};

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

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

    match changelog_update(client).await {
        Ok(_) => log::info!("Changelog updated!"),
        Err(e) => log::error!("Error updating changelog: {e}"),
    };

    Ok(())
}

async fn changelog_update(mut client: Client) -> Result<()> {
    println!(
        "PR ID: {} - Owner: {} - Repo: {}",
        client.pr_number(),
        client.owner(),
        client.repo()
    );

    let title = client.title();

    println!("Pull Request Title: {title}");

    client.create_entry()?;

    if let Some((section, entry)) = client.update_changelog()? {
        let section = match section {
            ChangeKind::Added => "Added",
            ChangeKind::Changed => "Changed",
            ChangeKind::Deprecated => "Deprecated",
            ChangeKind::Fixed => "Fixed",
            ChangeKind::Removed => "Removed",
            ChangeKind::Security => "Security",
        };
        println!("Proposed addition to change log unreleased changes: In Section: `{section}` add the following entry: `{entry}`");
    } else {
        println!("No update required");
        return Ok(());
    };

    println!("Changelog file name: {}", client.changelog());

    print_changelog(client.changelog());

    let report = client.repo_status()?;
    println!("Repo state:\n{report}");
    println!("Branch status: {}", client.branch_status()?);

    client.commit_changelog()?;
    println!("Repo state (after commit):\n{}", client.repo_status()?);
    println!("Branch status: {}", client.branch_status()?);

    client.push_changelog()?;
    println!("Branch status: {}", client.branch_status()?);

    Ok(())
}

#[allow(dead_code)]
fn commit_changelog(changelog_path: &str) -> Result<(), git2::Error> {
    println!("Committing changelog: {changelog_path}");
    let files = std::fs::read_dir(".").unwrap();
    println!("Files: ");
    for file in files.into_iter().flatten() {
        println!("\t{:?}", file.path());
    }

    let repo = Repository::open(".")?;

    println!("Repo state (before commit): {:?}", repo.state());

    let mut index = repo.index()?;
    index.add_path(Path::new(changelog_path))?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let head = repo.head()?;
    let parent = repo.find_commit(head.target().unwrap())?;
    let sig = repo.signature()?;

    println!("Ready to commit with tree id: {tree_id}, sig: {sig}");

    let _commit_id = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Update changelog",
        &repo.find_tree(tree_id)?,
        &[&parent],
    )?;

    println!("Repo state (after commit): {:?}", repo.state());

    // let mut remote = repo.remote("origin", "https://github.com/jerus-org/pcu.git")?;
    // remote.push(&["master"], None)?;
    // println!("Pushed to remote origin");

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
