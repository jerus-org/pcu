#![allow(dead_code)]
// use named_colour::ColourRgb;
use serde::{Deserialize, Serialize};

use crate::{Client, Error, GraphQLWrapper};

use tracing::instrument;

pub(crate) trait GraphQLCreateLabel {
    #[allow(async_fn_in_trait)]
    async fn create_label(
        &self,
        repo_node: &str,
        label: &str,
        color: &str,
        desc: &str,
    ) -> Result<String, Error>;
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Data {
    #[serde(rename = "createLabel")]
    create_label: CreateLabel,
}
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct CreateLabel {
    label: Label,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct Vars {
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

impl GraphQLCreateLabel for Client {
    #[instrument(skip(self))]
    async fn create_label(
        &self,
        repo_node: &str,
        label: &str,
        color: &str,
        desc: &str,
    ) -> Result<String, Error> {
        let mutation = r#"
        mutation ($repo_node: ID!, $label: String!, $color: String!, $desc: String!) {
            createLabel(input: {
              repositoryId: $repo_node,
              name: $label,
              color: $color
              description: $desc
            }) {
              label {
                id
                name
                color
              }
            }
          }
        "#;

        let vars = Vars {
            repo_node: repo_node.to_string(),
            label: label.to_string(),
            color: color.to_string(),
            desc: desc.to_string(),
        };

        log::trace!("vars: {vars:?}");

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<Data, Vars>(mutation, vars)
            .await;

        log::trace!("data_res: {data_res:?}");

        let data = data_res.map_err(GraphQLWrapper::from)?;

        log::trace!("data: {data:?}");

        Ok(data.create_label.label.id)
    }
}
