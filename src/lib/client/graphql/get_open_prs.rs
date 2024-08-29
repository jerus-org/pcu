#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use crate::{Client, Error, GraphQLWrapper};

pub(crate) trait GraphQLGetOpenPRs {
    #[allow(async_fn_in_trait)]
    async fn get_open_pull_requests(&self) -> Result<Vec<PrItem>, Error>;
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
    #[serde(rename = "pullRequests")]
    pull_requests: PullRequests,
}

#[derive(Deserialize, Debug, Clone)]
struct PullRequests {
    edges: Vec<Edge>,
}

#[derive(Deserialize, Debug, Clone)]
struct Edge {
    node: PullRequest,
}

#[derive(Deserialize, Debug, Clone)]
struct PullRequest {
    number: i64,
    title: String,
    author: Actor,
}

#[derive(Deserialize, Debug, Clone)]
struct Actor {
    login: String,
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    owner: String,
    name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PrItem {
    pub(crate) number: i64,
    title: String,
    pub(crate) login: String,
}

impl GraphQLGetOpenPRs for Client {
    async fn get_open_pull_requests(&self) -> Result<Vec<PrItem>, Error> {
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
            .query_with_vars_unwrap::<Data, Vars>(query, vars)
            .await;

        log::trace!("data_res: {:?}", data_res);

        let data = data_res.map_err(GraphQLWrapper::from)?;

        log::trace!("data: {:?}", data);

        let edges = data
            .repository
            .pull_requests
            .edges
            .iter()
            .map(|pr| PrItem {
                number: pr.node.number,
                title: pr.node.title.clone(),
                login: pr.node.author.login.clone(),
            })
            .collect();

        Ok(edges)
    }
}
