#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use crate::{Error, GraphQLWrapper};

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct GetPullRequestTitle {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Repository {
    #[serde(skip_deserializing)]
    owner: String,
    #[serde(skip_deserializing)]
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

#[allow(async_fn_in_trait)]
pub(crate) async fn get_pull_request_title(
    github_graphql: &gql_client::Client,
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

    let vars = Vars {
        owner: owner.to_string(),
        name: name.to_string(),
        number,
    };

    let data_res = github_graphql
        .query_with_vars_unwrap::<GetPullRequestTitle, Vars>(query, vars)
        .await;

    log::trace!("data_res: {:?}", data_res);

    let data = data_res.map_err(GraphQLWrapper::from)?;

    log::trace!("data: {:?}", data);

    let title = data.repository.pull_request.title;

    Ok(title)
}
