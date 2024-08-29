#![allow(dead_code)]
// use named_colour::ColourRgb;
use serde::{Deserialize, Serialize};

use crate::{
    client::graphql::{GraphQLCreateLabel, GraphQLGetRepoID},
    Client, Error, GraphQLWrapper,
};

use tracing::instrument;

const LABEL: &str = "rebase";
const COLOR: &str = "B22222";
pub(crate) trait GraphQLGetLabel {
    #[allow(async_fn_in_trait)]
    async fn get_or_create_label_id(&self, label: Option<&str>) -> Result<String, Error>;
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
    async fn get_or_create_label_id(&self, label: Option<&str>) -> Result<String, Error> {
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

        let vars = Vars {
            owner: self.owner.clone(),
            name: self.repo().to_string(),
            label: label.clone(),
        };

        tracing::trace!("vars: {:?}", vars);

        let data_res = self
            .github_graphql
            .query_with_vars_unwrap::<Data, Vars>(query, vars)
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

        Ok(label_id)
    }
}
