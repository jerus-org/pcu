#![allow(dead_code)]
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{Client, Error, GraphQLWrapper};

use tracing::instrument;

pub(crate) trait GraphQLGetTag {
    #[allow(async_fn_in_trait)]
    async fn get_tag(&self, tag: &str) -> Result<TagTarget, Error>;
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
    target: TagTarget,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct TagTarget {
    #[serde(rename = "__typename")]
    typename: String,
    name: String,
    message: String,
    tagger: Tagger,
    commit: Option<Commit>,
    target: Option<CommitTarget>,
}

impl TagTarget {
    pub fn commit_sha(&self) -> Option<String> {
        let mut sha = None;
        if let Some(commit) = &self.commit {
            sha = Some(commit.oid.clone());
        }

        if let Some(target) = &self.target {
            sha = Some(target.oid.clone());
        }

        sha
    }
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Tagger {
    name: String,
    email: String,
    date: String,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Commit {
    oid: String,
    #[serde(rename = "committedDate")]
    committed_date: NaiveDateTime,
    author: Author,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Author {
    name: String,
    email: String,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct CommitTarget {
    oid: String,
    #[serde(rename = "committedDate")]
    committed_date: NaiveDateTime,
    author: Author,
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    owner: String,
    name: String,
    tag: String,
}

impl GraphQLGetTag for Client {
    #[instrument(skip(self))]
    async fn get_tag(&self, tag: &str) -> Result<TagTarget, Error> {
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
                        target {
                            ... on Commit {
                              oid
                              committedDate
                              author {
                                name
                                email
                              }
                            }
                          }
                        }
                        ... on Commit {
                          oid
                          committedDate
                          author {
                            name
                            email
                          }
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
