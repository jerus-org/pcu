use std::{path::Path, str::FromStr};

use git2::Repository;
use pcu_lib::{Client, PrTitle};
use url::Url;

use eyre::Result;

const CHANGELOG_FILENAME: &str = "CHANGELOG.md";

#[tokio::main]
async fn main() {
    let client = Client::new();

    if client.branch() == "main" {
        println!("I am on the main branch, so nothing more to do!");
    } else {
        println!(
            "I am on the `{}` branch, so time to get to work!",
            client.branch()
        );

        match changelog_update(client).await {
            Ok(_) => println!("Changelog updated!"),
            Err(e) => println!("Error updating changelog: {e}"),
        }
    }
}

async fn changelog_update(client: Client) -> Result<()> {
    println!(
        "PR ID: {} - Owner: {} - Repo: {}",
        client.pr_number(),
        client.owner(),
        client.repo()
    );

    if let Ok(pr_number) = client.pr_number().parse::<u64>() {
        println!("Pr #: {pr_number}!");

        // let pulls = octocrab::instance()
        //     .pulls(client.owner(), client.repo())
        //     .list()
        //     .send()
        //     .await?;

        // let pull_release = pulls.into_iter().find(|pr| pr.number == pr_number).unwrap();
        let pull_release = octocrab::instance()
            .pulls(client.owner(), client.repo())
            .get(pr_number)
            .await?;

        if let Some(title) = pull_release.title {
            let mut pr_title = PrTitle::parse(&title);
            pr_title.pr_id = Some(pr_number);
            pr_title.pr_url = Some(Url::from_str(client.pull_release()).unwrap());

            let change_log = get_changelog_name();
            println!("Changelog file name: {change_log}");

            pr_title.update_change_log(&change_log);

            println!("Change entry:{:#?}", pr_title.entry);

            if let Err(e) = commit_changelog(&change_log) {
                eprintln!("Error committing changelog: {}", e);
                return Err(e.into());
            }

            println!("Changelog updated!");
        }
    };

    Ok(())
}

fn get_changelog_name() -> String {
    let files = std::fs::read_dir(".").unwrap();
    for file in files.into_iter().flatten() {
        println!("File: {:?}", file.path());

        if file.file_name().to_string_lossy().contains("change")
            && file.file_type().unwrap().is_file()
        {
            return file.file_name().to_string_lossy().into_owned();
        }
    }

    CHANGELOG_FILENAME.to_string()
}

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
