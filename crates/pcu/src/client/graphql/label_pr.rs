#![allow(dead_code)]
// use named_colour::ColourRgb;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    client::graphql::{GraphQLGetLabel, GraphQLGetPRId},
    Client, Error, GraphQLWrapper,
};

pub(crate) trait GraphQLLabelPR {
    async fn add_label_to_pr(
        &self,
        pr_number: i64,
        label: Option<&str>,
        desc: Option<&str>,
        colour: Option<&str>,
    ) -> Result<(), Error>;
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    label_id: String,
    pr_id: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Data {
    #[serde(rename = "addLabelsToLabelable")]
    add_labels_to_labelable: AddLabel,
}
#[derive(Deserialize, Debug, Clone)]
struct AddLabel {
    labelable: LabelAble,
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
    name: String,
}

impl GraphQLLabelPR for Client {
    #[instrument(skip(self))]
    async fn add_label_to_pr(
        &self,
        pr_number: i64,
        label: Option<&str>,
        desc: Option<&str>,
        colour: Option<&str>,
    ) -> Result<(), Error> {
        let label_id = self.get_or_create_label_id(label, desc, colour).await?;
        log::trace!("label_id: {label_id:?}");

        let pr_id = self.get_pull_request_id(pr_number).await?;
        log::trace!("pr_id: {pr_id:?}");

        let vars = Vars {
            label_id: label_id.to_string(),
            pr_id: pr_id.to_string(),
        };
        log::trace!("vars: {vars:?}");

        let mutation = r#"
            mutation ($pr_id: ID!, $label_id: ID!) {
              addLabelsToLabelable(input: {labelableId: $pr_id, labelIds: [$label_id]}) {
                labelable {
                  labels(first: 10) {
                    edges {
                      node {
                        name
                      }
                    }
                  }
                }
              }
            }
        "#;

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<Data, Vars>(mutation, vars)
            .await;

        log::trace!("data_res: {data_res:?}");

        let data = data_res.map_err(GraphQLWrapper::from)?;

        log::trace!("data: {data:?}");

        Ok(())
    }
}
