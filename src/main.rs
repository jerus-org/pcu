use std::env;

fn main() {
    let branch = env::var("CIRCLE_BRANCH").unwrap_or("".to_string());

    let owner = env::var("CIRCLE_PROJECT_USERNAME").unwrap_or("".to_string());
    let repo = env::var("CIRCLE_PROJECT_REPONAME").unwrap_or("".to_string());

    // let pr = octocrab::instance()
    //     .pulls(owner, repo)
    //     .get(pr_number)
    //     .await?;

    println!("I am on the project: {owner}/{repo}!");

    println!("I am on the branch: {branch}!");
}
