#![allow(dead_code)]
// use named_colour::ColourRgb;
use serde::{Deserialize, Serialize};

use crate::{
    client::graphql::{GraphQLCreateLabel, GraphQLGetRepoID},
    Client, Error, GraphQLWrapper,
};

use tracing::instrument;

const LABEL: &str = "rebase";
const LABEL_COLOR: &str = "B22222";
const LABEL_DESCRIPTION: &str = "Label to trigger rebase";

pub(crate) trait GraphQLGetLabel {
    #[allow(async_fn_in_trait)]
    async fn get_or_create_label_id(
        &self,
        label: Option<&str>,
        desc: Option<&str>,
        colour: Option<&str>,
    ) -> Result<String, Error>;
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
    // #[serde(skip_deserializing)]
    label: Label,
}

#[derive(Deserialize, Debug, Clone)]
struct LabelId {
    id: String,
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    owner: String,
    name: String,
    label: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Label {
    name: String,
    color: String,
    id: String,
}

impl GraphQLGetLabel for Client {
    #[instrument(skip(self))]
    async fn get_or_create_label_id(
        &self,
        label: Option<&str>,
        desc: Option<&str>,
        colour: Option<&str>,
    ) -> Result<String, Error> {
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

        let label = if let Some(l) = label {
            l.to_string()
        } else {
            LABEL.to_string()
        };

        let desc = if let Some(d) = desc {
            d.to_string()
        } else {
            LABEL_DESCRIPTION.to_string()
        };

        let colour = if let Some(c) = colour {
            c.to_string()
        } else {
            LABEL_COLOR.to_string()
        };

        let vars = Vars {
            owner: self.owner.clone(),
            name: self.repo().to_string(),
            label: label.clone(),
        };

        log::trace!("vars: {vars:?}");

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<Data, Vars>(query, vars)
            .await;

        log::trace!("data_res: {data_res:?}");

        let label_id = if data_res.is_err() {
            log::trace!("label `{label}` not found");

            let repo_id_res = self.get_repository_id().await;

            log::trace!("repo_id_res: {repo_id_res:?}");

            let repo_id = repo_id_res?;

            log::trace!("repo_id: {repo_id:?}");

            let id_res = self.create_label(&repo_id, &label, &colour, &desc).await;
            log::trace!("id_res: {id_res:?}");

            let id = id_res?;
            log::debug!("label_id: {id:?}");

            id
        } else {
            let data = data_res.map_err(GraphQLWrapper::from)?;
            log::debug!("data: {data:?}");
            data.repository.label.id
        };

        log::trace!("label_id: {label_id:?}");

        Ok(label_id)
    }
}
