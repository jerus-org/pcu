#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{Client, Error, GraphQLWrapper};

pub(crate) trait GraphQLGetPRId {
    #[allow(async_fn_in_trait)]
    async fn get_pull_request_id(&self, pr_number: i64) -> Result<String, Error>;
}

#[derive(Deserialize, Debug, Clone)]
struct Data {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
struct Repository {
    #[serde(skip_deserializing)]
    owner: String,
    #[serde(skip_deserializing)]
    name: String,
    #[serde(rename = "pullRequest")]
    pull_request: PullRequest,
}

#[derive(Deserialize, Debug, Clone)]
struct PullRequest {
    id: String,
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    owner: String,
    name: String,
    pr_number: i64,
}

impl GraphQLGetPRId for Client {
    #[instrument(skip(self))]
    async fn get_pull_request_id(&self, pr_number: i64) -> Result<String, Error> {
        let query = r#"
            query($owner: String!, $name: String!, $pr_number: Int!) {
              repository(owner: $owner, name: $name) {
                pullRequest(number: $pr_number) {
                  id
                }
              }
            }
            "#;

        let vars = Vars {
            owner: self.owner.clone(),
            name: self.repo().to_string(),
            pr_number,
        };

        log::trace!("vars: {vars:?}");

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<Data, Vars>(query, vars)
            .await;

        log::trace!("data_res: {data_res:?}");

        let data = data_res.map_err(GraphQLWrapper::from)?;

        log::trace!("data: {data:?}");

        Ok(data.repository.pull_request.id)
    }
}
