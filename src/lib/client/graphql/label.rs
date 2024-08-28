#![allow(dead_code)]
use color_eyre::owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::{client::graphql::repo::GraphQLRepo, Client, Error, GraphQLWrapper};

use tracing::instrument;

const LABEL: &str = "rebase";
const COLOR: &str = "FF0000";

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct GetLabelID {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Repository {
    #[serde(skip_deserializing)]
    owner: String,
    #[serde(skip_deserializing)]
    name: String,
    // #[serde(skip_deserializing)]
    label: Label,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct LabelId {
    id: String,
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
    label: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PrItem {
    pub(crate) number: i64,
    pub(crate) title: String,
    pub(crate) login: String,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Label {
    pub(crate) name: String,
    pub(crate) color: String,
    pub(crate) id: String,
}

pub(crate) trait GraphQLLabel {
    #[allow(async_fn_in_trait)]
    async fn label_pr(&self, pr_number: i64) -> Result<(), Error>;
}

impl GraphQLLabel for Client {
    // query ($owner: String!, $name: String!){
    //     repository(owner: $owner, name: $name) {
    //       id
    //     }
    //   }

    #[instrument(skip(self))]
    async fn label_pr(&self, pr_number: i64) -> Result<(), Error> {
        tracing::trace!("label_pr number: {}", pr_number);

        // Get the label ID
        let query = r#"
        query($owner:String!, $name:String!, $label:String!) {
            repository(owner: $owner, name: $name) {
                label(name: $label) {
                    id,
                    name,
                    color
                    }
                }
            }
            "#;

        // let label = LABEL.to_string();
        let label = "test".to_string();

        let vars = Vars {
            owner: self.owner.clone(),
            name: self.repo().to_string(),
            label: label.clone(),
        };

        tracing::trace!("vars: {:?}", vars);

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<GetLabelID, Vars>(query, vars)
            .await;

        tracing::trace!("data_res: {:?}", data_res);

        if data_res.is_err() {
            tracing::trace!("label `{}` not found", label.white());

            let repo_id_res = self.get_repository_id().await;

            tracing::trace!("repo_id_res: {:?}", repo_id_res);

            let repo_id = repo_id_res?;

            tracing::trace!("repo_id: {:?}", repo_id);

            // let mutation = r#"
            // mutation ($repo_node: ID!, $label: String!, $color: String!) {
            //     createLabel(input: {
            //       repositoryId: $repo_node,
            //       name: $label,
            //       color: $color
            //       description: "Label to trigger rebase"
            //     }) {
            //       label {
            //         id
            //         name
            //         color
            //       }
            //     }
            //   }
            // "#;

            return Ok(());
        };

        let data = data_res.map_err(GraphQLWrapper::from)?;

        tracing::trace!("data: {:?}", data);

        Ok(())
    }
}
