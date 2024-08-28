#![allow(dead_code)]
// use named_colour::ColourRgb;
use serde::{Deserialize, Serialize};

use crate::{client::graphql::repo::GraphQLRepo, Client, Error, GraphQLWrapper};

use tracing::instrument;

const LABEL: &str = "rebase";
const COLOR: &str = "B22222";

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

#[derive(Serialize, Debug, Clone)]
pub(crate) struct Vars {
    owner: String,
    name: String,
    label: String,
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct CreateLabelData {
    #[serde(rename = "createLabel")]
    create_label: CreateLabel,
}
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct CreateLabel {
    label: Label,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct CreateVars {
    repo_node: String,
    label: String,
    color: String,
    desc: String,
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
    async fn create_label(
        &self,
        repo_node: &str,
        label: &str,
        color: &str,
        desc: &str,
    ) -> Result<String, Error>;
}

impl GraphQLLabel for Client {
    #[instrument(skip(self))]
    async fn label_pr(&self, pr_number: i64) -> Result<(), Error> {
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

        let label_id = if data_res.is_err() {
            tracing::trace!("label `{}` not found", label);

            let repo_id_res = self.get_repository_id().await;

            tracing::trace!("repo_id_res: {:?}", repo_id_res);

            let repo_id = repo_id_res?;

            tracing::trace!("repo_id: {:?}", repo_id);

            let id_res = self
                .create_label(&repo_id, &label, COLOR, "Label to trigger rebase")
                .await;
            tracing::trace!("id_res: {:?}", id_res);

            let id = id_res?;
            tracing::debug!("label_id: {:?}", id);

            id
        } else {
            let data = data_res.map_err(GraphQLWrapper::from)?;
            tracing::debug!("data: {:?}", data);
            data.repository.label.id
        };

        tracing::trace!("label_id: {:?}", label_id);

        Ok(())
    }

    #[instrument(skip(self))]
    async fn create_label(
        &self,
        repo_node: &str,
        label: &str,
        color: &str,
        desc: &str,
    ) -> Result<String, Error> {
        let mutation = r#"
        mutation ($repo_node: ID!, $label: String!, $color: String!) {
            createLabel(input: {
              repositoryId: $repo_node,
              name: $label,
              color: $color
              description: "Label to trigger rebase"
            }) {
              label {
                id
                name
                color
              }
            }
          }
        "#;

        let vars = CreateVars {
            repo_node: repo_node.to_string(),
            label: label.to_string(),
            color: color.to_string(),
            desc: desc.to_string(),
        };

        tracing::trace!("vars: {:?}", vars);

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<CreateLabelData, CreateVars>(mutation, vars)
            .await;

        tracing::trace!("data_res: {:?}", data_res);

        let data = data_res.map_err(GraphQLWrapper::from)?;

        tracing::trace!("data: {:?}", data);

        Ok(data.create_label.label.id)
    }
}
