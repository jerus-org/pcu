#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{Client, Error, GraphQLWrapper};

pub(crate) trait GraphQLGetRepoID {
    #[allow(async_fn_in_trait)]
    async fn get_repository_id(&self) -> Result<String, Error>;
}

#[derive(Deserialize, Debug, Clone)]
struct GetRepositoryId {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
struct Repository {
    #[serde(skip_deserializing)]
    owner: String,
    #[serde(skip_deserializing)]
    name: String,
    id: String,
}
#[derive(Serialize, Debug, Clone)]
struct Vars {
    owner: String,
    name: String,
}

impl GraphQLGetRepoID for Client {
    #[instrument(skip(self))]
    async fn get_repository_id(&self) -> Result<String, Error> {
        let query = r#"
                    query ($owner: String!, $name: String!){
                repository(owner: $owner, name: $name) {
                  id
                }
              }
            "#;

        let vars = Vars {
            owner: self.owner.clone(),
            name: self.repo().to_string(),
        };

        log::trace!("vars: {vars:?}");

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<GetRepositoryId, Vars>(query, vars)
            .await;

        log::trace!("data_res: {data_res:?}");

        let data = data_res.map_err(GraphQLWrapper::from)?;

        log::trace!("data: {data:?}");

        Ok(data.repository.id)
    }
}
