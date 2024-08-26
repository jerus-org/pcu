#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use crate::{Client, Error, GraphQLWrapper};

use super::GraphQL;

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
    #[serde(rename = "pullRequests")]
    pull_requests: PullRequests,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct PullRequests {
    edges: Vec<Edge>,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Edge {
    node: PullRequest,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct PullRequest {
    number: i64,
    title: String,
    author: Actor,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Actor {
    login: String,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct Vars {
    owner: String,
    name: String,
}

impl GraphQL for Client {
    async fn get_open_pull_requests(&self) -> Result<Vec<Edge>, Error> {
        log::trace!("get_open_pull_requests");
        let query = r#"
        query($owner:String!, $name:String!){
            repository(owner: $owner, name: $name) {
                pullRequests(first: 100, states: OPEN) {
                    edges {
                        node {
                            title
                            number
                            author {login}
                            }
                        }
                    }
                }
            }
            "#;

        let vars = Vars {
            owner: self.owner.clone(),
            name: self.repo().to_string(),
        };

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<GetPullRequestTitle, Vars>(query, vars)
            .await;

        log::trace!("data_res: {:?}", data_res);

        let data = data_res.map_err(GraphQLWrapper::from)?;

        log::trace!("data: {:?}", data);

        let edges = data.repository.pull_requests.edges;

        Ok(edges)
    }
}
