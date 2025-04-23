#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use crate::{Client, Error, GraphQLWrapper};

use tracing::instrument;

pub(crate) trait GraphQLGetTag {
    #[allow(async_fn_in_trait)]
    async fn get_tag(&self, tag: &str) -> Result<Target, Error>;
}

#[derive(Deserialize, Debug, Clone)]
struct GetTag {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
struct Repository {
    #[serde(skip_deserializing)]
    owner: String,
    #[serde(skip_deserializing)]
    name: String,
    #[serde(rename = "ref")]
    _ref: References,
}

#[derive(Deserialize, Debug, Clone)]
struct References {
    target: Target,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Target {
    #[serde(rename = "__typename")]
    typename: String,
    name: String,
    message: String,
    tagger: Tagger,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Tagger {
    name: String,
    email: String,
    date: String,
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    owner: String,
    name: String,
    tag: String,
}

impl GraphQLGetTag for Client {
    #[instrument(skip(self))]
    async fn get_tag(&self, tag: &str) -> Result<Target, Error> {
        let query = r#"
                    query ($owner: String!, $name: String!, $tag: String!) {
                repository(owner: $owner, name: $name) {
                  ref(qualifiedName: $tag) {
                    target {
                      __typename
                      ... on Tag {
                        name
                        message
                        tagger {
                          name
                          email
                          date
                        }
                      }
                    }
                  }
                }
            }
            "#;

        let tag = format!("refs/tags/{tag}");

        let vars = Vars {
            owner: self.owner.clone(),
            name: self.repo().to_string(),
            tag,
        };

        tracing::trace!("vars: {:?}", vars);

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<GetTag, Vars>(query, vars)
            .await;

        tracing::trace!("data_res: {:?}", data_res);

        let data = data_res.map_err(GraphQLWrapper::from)?;

        tracing::trace!("data: {:?}", data);

        Ok(data.repository._ref.target)
    }
}
