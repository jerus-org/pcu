#![allow(dead_code)]
use std::collections::HashMap;

use gql_client::Client;
use serde::{Deserialize, Serialize};

use crate::Error;

const END_POINT: &str = "https://api.github.com/graphql";

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct GetPullRequestTitle {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Repository {
    owner: String,
    name: String,
    #[serde(rename = "pullRequest")]
    pull_request: PullRequest,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct PullRequest {
    number: i64,
    title: String,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct Vars {
    owner: String,
    name: String,
    number: i64,
}

pub(crate) async fn get_pull_request_title(
    owner: &str,
    name: &str,
    number: i64,
) -> Result<String, Error> {
    let query = r#"
    query($owner:String!, $name:String!, $number:Int!){
        repository(owner: $owner, name: $name) {
            pullRequest(number: $number) {
                number
                title
            }
        }
    }
    "#;

    let headers = HashMap::from([("User-Agent", owner)]);

    let client = Client::new_with_headers(END_POINT, headers);
    let vars = Vars {
        owner: owner.to_string(),
        name: name.to_string(),
        number,
    };
    let data = client
        .query_with_vars_unwrap::<GetPullRequestTitle, Vars>(query, vars)
        .await
        .unwrap();

    log::trace!("data: {:?}", data);

    let title = data.repository.pull_request.title;

    Ok(title)
}
