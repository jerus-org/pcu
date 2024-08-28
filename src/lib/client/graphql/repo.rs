#![allow(dead_code)]
use ansi_term::Style;
use serde::{Deserialize, Serialize};

use crate::{Client, Error, GraphQLWrapper};

use tracing::instrument;

const LABEL: &str = "rebase";
const COLOR: &str = "FF0000";

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct GetRepositoryId {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Repository {
    owner: String,
    name: String,
    id: String,
}
#[derive(Serialize, Debug, Clone)]
pub(crate) struct Vars {
    owner: String,
    name: String,
}

pub(crate) trait GraphQLRepo {
    #[allow(async_fn_in_trait)]
    async fn get_repository_id(&self) -> Result<String, Error>;
}

impl GraphQLRepo for Client {
    #[instrument(skip(self))]
    async fn get_repository_id(&self) -> Result<String, Error> {
        tracing::trace!("{}", Style::new().bold().paint("get_repository_id"));

        // Get the label ID
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

        tracing::trace!("vars: {:?}", vars);

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<GetRepositoryId, Vars>(query, vars)
            .await;

        tracing::trace!("data_res: {:?}", data_res);

        let data = data_res.map_err(GraphQLWrapper::from)?;

        tracing::trace!("data: {:?}", data);

        Ok(data.repository.id)
    }
}
