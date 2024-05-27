use std::{env, path::Path};

use git2::Repository;
use pcu_lib::PrTitle;

const CHANGELOG_FILENAME: &str = "CHANGELOG.md";

#[tokio::main]
async fn main() {
    let pcu_branch = env::var("PCU_BRANCH").unwrap_or("".to_string());

    let pcu_pull_request = env::var("PCU_PULL_REQUEST").unwrap_or("".to_string());
    let pr = env::var(pcu_pull_request).unwrap_or("".to_string());

    let parts = pr.splitn(7, '/');
    println!("Parts: {parts:?}");

    let branch = env::var(pcu_branch).unwrap_or("".to_string());

    sameness_check();

    if branch == "main" {
        println!("I am on the main branch, so nothing more to do!");
    } else {
        println!("I am on the {branch}  branch, so time to get to work!");

        match changelog_update().await {
            Ok(_) => println!("Changelog updated!"),
            Err(e) => println!("Error updating changelog: {e}"),
        }
    }
}

async fn changelog_update() -> Result<(), octocrab::Error> {
    let pcu_reponame = env::var("PCU_REPONAME").unwrap_or("".to_string());
    let pcu_username = env::var("PCU_USERNAME").unwrap_or("".to_string());
    let pcu_pull_request = env::var("PCU_PULL_REQUEST").unwrap_or("".to_string());

    let owner = env::var(pcu_reponame).unwrap_or("".to_string());
    let repo = env::var(pcu_username).unwrap_or("".to_string());
    let pr = env::var(pcu_pull_request).unwrap_or("".to_string());

    println!("I am in pr: {pr}!");
    println!("I am on the project: {owner}/{repo}!");

    let last_slash = pr.rfind('/').unwrap_or(0);
    println!("Last slash: {last_slash}");
    println!("Length of pr: {}", pr.len());

    let pr_id = &pr[last_slash + 1..];
    if let Ok(pr_id) = pr_id.parse::<u64>() {
        println!("Pr #: {pr_id}!");

        let pull_release = octocrab::instance().pulls(owner, repo).get(pr_id).await?;

        if let Some(title) = pull_release.title {
            let mut pr_title = PrTitle::parse(&title);
            println!("PR: {:#?}", pr_title);

            let change_log = get_changelog_name();
            println!("Changelog file name: {change_log}");

            pr_title.update_change_log(&change_log);

            if let Err(e) = commit_changelog(&change_log) {
                eprintln!("Error committing changelog: {}", e);
            }
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

fn sameness_check() {
    let pcu_branch = env::var("PCU_BRANCH").unwrap_or("".to_string());

    let pcu = env::var(pcu_branch).unwrap_or("".to_string());
    let circle = env::var("CIRCLE_BRANCH").unwrap_or("".to_string());

    println!("Are they the same? {pcu} vs {circle}");
    if pcu == circle {
        println!("They are the same!");
    } else {
        println!("They are not the same!");
    }
}

fn commit_changelog(changelog_path: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    let mut index = repo.index()?;
    index.add_path(Path::new(changelog_path))?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let head = repo.head()?;
    let parent = repo.find_commit(head.target().unwrap())?;
    let sig = repo.signature()?;
    let _commit_id = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Update changelog",
        &repo.find_tree(tree_id)?,
        &[&parent],
    )?;
    Ok(())
}
