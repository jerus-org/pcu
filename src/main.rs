use std::env;

mod log_update;

#[tokio::main]
async fn main() {
    let branch = env::var("CIRCLE_BRANCH").unwrap_or("".to_string());

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
    println!("I am on the project: {owner}/{repo}!");

    let pr_list = octocrab::instance()
        .pulls(owner, repo)
        .list()
        .send()
        .await?;

    pr_list.items.iter().for_each(|pr| {
        println!("PR: {:?}", pr.title);
    });

    Ok(())
}
