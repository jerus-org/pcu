use std::env;

use pcu_lib::PrTitle;

const CHANGELOG_FILENAME: &str = "CHANGELOG.md";

#[tokio::main]
async fn main() {
    let branch = env::var("PCU_BRANCH").unwrap_or("".to_string());

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
    let owner = env::var("CIRCLE_PROJECT_USERNAME").unwrap_or("".to_string());
    let repo = env::var("CIRCLE_PROJECT_REPONAME").unwrap_or("".to_string());
    let pr_number = env::var("CIRCLE_PULL_REQUEST").unwrap_or("".to_string());

    println!("I am in pr: {pr_number}!");
    println!("I am on the project: {owner}/{repo}!");

    let last_slash = pr_number.rfind('/').unwrap_or(0);
    println!("Last slash: {last_slash}");
    println!("Length of pr number: {}", pr_number.len());

    let pr_id = &pr_number[last_slash + 1..];
    if let Ok(pr_id) = pr_id.parse::<u64>() {
        println!("I am in pr: {pr_id}!");

        let pull_release = octocrab::instance().pulls(owner, repo).get(pr_id).await?;

        if let Some(title) = pull_release.title {
            let mut pr_title = PrTitle::parse(&title);
            println!("PR: {:#?}", pr_title);

            let change_log = get_changelog_name();
            println!("Changelog file name: {change_log}");

            pr_title.update_change_log(&change_log);
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
    let pcu = env::var("PCU_BRANCH").unwrap_or("".to_string());
    let circle = env::var(keys::CIRCLE_BRANCH).unwrap_or("".to_string());

    println!("Are they the same? {pcu} vs {circle}");
    if pcu == circle {
        println!("They are the same!");
    } else {
        println!("They are not the same!");
    }
}
