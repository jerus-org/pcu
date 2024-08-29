#![allow(dead_code)]
// use named_colour::ColourRgb;
use serde::{Deserialize, Serialize};

use crate::{
    client::graphql::{GraphQLGetLabel, GraphQLGetPRId},
    Client, Error, GraphQLWrapper,
};

use tracing::instrument;

pub(crate) trait GraphQLLabelPR {
    async fn add_label_to_pr(&self, pr_number: i64) -> Result<(), Error>;
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    label_id: String,
    pr_id: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Data {
    lableable: LabelAble,
}

#[derive(Deserialize, Debug, Clone)]
struct LabelAble {
    labels: Labels,
}

#[derive(Deserialize, Debug, Clone)]
struct Labels {
    edges: Vec<Edge>,
}

#[derive(Deserialize, Debug, Clone)]
struct Edge {
    node: Label,
}

#[derive(Deserialize, Debug, Clone)]
struct Label {
    id: String,
}

impl GraphQLLabelPR for Client {
    #[instrument(skip(self))]
    async fn add_label_to_pr(&self, pr_number: i64) -> Result<(), Error> {
        let label_id = self.get_or_create_label_id().await?;
        tracing::trace!("label_id: {:?}", label_id);

        let pr_id = self.get_pull_request_id(pr_number).await?;
        tracing::trace!("pr_id: {:?}", pr_id);

        let vars = Vars {
            label_id: label_id.to_string(),
            pr_id: pr_id.to_string(),
        };
        tracing::trace!("vars: {:?}", vars);

        let mutation = r#"
        mutation ($pr_id: ID!, $label_id: String!) {
        addLabelsToLabelable(input: {labelableId: $pr_id, labelIds: [$label_id]}) {
          labelable {
            labels(first: 10) {
              edges {
                node {
                  id
                }
              }
            }
          }
        }
        "#;

        tracing::trace!("vars: {:?}", vars);

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<Data, Vars>(mutation, vars)
            .await;

        tracing::trace!("data_res: {:?}", data_res);

        let data = data_res.map_err(GraphQLWrapper::from)?;

        tracing::trace!("data: {:?}", data);

        Ok(())
    }
}
