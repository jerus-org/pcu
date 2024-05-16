use std::env;

fn main() {
    let branch = env::var("CIRCLE_BRANCH").unwrap_or("".to_string());

    println!("I am on the branch: {branch}!");
}
