use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{Client, Error, GraphQLWrapper};

/// Every release that could match a tag, gathered in one round trip.
///
/// Both halves are needed because neither is sufficient alone:
///
/// - `release(tagName:)` resolves a tag directly at any depth of history, but —
///   exactly like the REST `get_release_by_tag` endpoint — it returns
///   **published releases only**. Asked for a tag that carries both a draft and
///   a published release it answers with the published one, never the draft.
/// - `releases(first:)` does include drafts (for a viewer with push access),
///   and is the only way to see one.
///
/// GraphQL has no server-side filter for drafts — `releases()` accepts only
/// pagination and `orderBy` — so the listing is fetched whole. That is
/// affordable here precisely because GraphQL returns only the four fields the
/// decision needs: for pcu's own history a 100-release page costs ~8 KB, against
/// ~411 KB for the equivalent REST listing, which drags along every release's
/// full notes, author, asset array and a dozen URLs.
///
/// Only the newest page is fetched. A draft worth reusing was created by the
/// current run or a recent one, and any *published* release, however old, is
/// found by the by-tag half of the same query.
pub(crate) trait GraphQLGetReleases {
    #[allow(async_fn_in_trait)]
    async fn get_release_candidates(&self, tag: &str) -> Result<Vec<ReleaseNode>, Error>;
}

#[derive(Deserialize, Debug, Clone)]
struct GetReleases {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
struct Repository {
    release: Option<ReleaseNode>,
    releases: ReleaseConnection,
}

#[derive(Deserialize, Debug, Clone)]
struct ReleaseConnection {
    nodes: Vec<ReleaseNode>,
}

/// One release, reduced to what choosing between releases actually requires.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReleaseNode {
    /// REST id. Asset upload goes to `uploads.github.com`, which is REST-only,
    /// so the numeric id is required even though the lookup is GraphQL.
    #[serde(rename = "databaseId")]
    pub(crate) database_id: Option<i64>,
    #[serde(rename = "tagName")]
    pub(crate) tag_name: String,
    #[serde(rename = "isDraft")]
    pub(crate) is_draft: bool,
    /// Whether GitHub has frozen this release's assets.
    ///
    /// Not exposed by octocrate's REST model, which predates the field — the
    /// single reason this lookup is worth expressing in GraphQL beyond the
    /// payload saving. It turns "attempt the upload and interpret the error
    /// message" into a decision made before any upload is attempted.
    pub(crate) immutable: bool,
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    owner: String,
    name: String,
    tag: String,
}

impl GraphQLGetReleases for Client {
    #[instrument(skip(self))]
    async fn get_release_candidates(&self, tag: &str) -> Result<Vec<ReleaseNode>, Error> {
        log::trace!("In get_release_candidates looking for releases tagged: {tag}");
        let query = r#"
            query ($owner: String!, $name: String!, $tag: String!) {
              repository(owner: $owner, name: $name) {
                release(tagName: $tag) {
                  databaseId
                  tagName
                  isDraft
                  immutable
                }
                releases(first: 100, orderBy: {field: CREATED_AT, direction: DESC}) {
                  nodes {
                    databaseId
                    tagName
                    isDraft
                    immutable
                  }
                }
              }
            }"#;

        let vars = Vars {
            owner: self.owner.clone(),
            name: self.repo().to_string(),
            tag: tag.to_string(),
        };

        log::trace!("vars: {vars:?}");

        let data = self
            .github_graphql
            .query_with_vars_unwrap::<GetReleases, Vars>(query, vars)
            .await
            .map_err(GraphQLWrapper::from)?;

        log::trace!("data: {data:?}");

        Ok(collect_candidates(data.repository))
    }
}

/// Flatten the two halves of the response into one candidate list.
///
/// The by-tag result leads, so a published release found directly is considered
/// before anything in the listing; the listing then contributes the drafts. A
/// release appearing in both is harmless — selection matches on tag name and
/// takes the first acceptable candidate.
fn collect_candidates(repository: Repository) -> Vec<ReleaseNode> {
    let mut candidates = Vec::with_capacity(repository.releases.nodes.len() + 1);
    if let Some(release) = repository.release {
        candidates.push(release);
    }
    candidates.extend(repository.releases.nodes);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shaped on the real response for jerus-org/gen-circleci-orb `v0.0.41`,
    /// which carries an abandoned empty draft alongside the published release
    /// that superseded it 37 minutes later.
    const RESPONSE: &str = r#"{
        "repository": {
            "release": {"databaseId": 333744509, "tagName": "v0.0.41", "isDraft": false, "immutable": false},
            "releases": {"nodes": [
                {"databaseId": 333744509, "tagName": "v0.0.41", "isDraft": false, "immutable": false},
                {"databaseId": 333719504, "tagName": "v0.0.41", "isDraft": true, "immutable": false},
                {"databaseId": 361109117, "tagName": "jci-audit-v0.0.1", "isDraft": false, "immutable": true}
            ]}
        }
    }"#;

    #[test]
    fn deserialises_a_release_lookup_response() {
        let data: GetReleases = serde_json::from_str(RESPONSE).unwrap();
        assert_eq!(
            data.repository.release.unwrap().database_id,
            Some(333744509)
        );
        assert_eq!(data.repository.releases.nodes.len(), 3);
        assert!(data.repository.releases.nodes[1].is_draft);
        assert!(
            data.repository.releases.nodes[2].immutable,
            "the immutable field must survive deserialisation — it is the reason \
             this lookup is GraphQL"
        );
    }

    #[test]
    fn collect_candidates_puts_the_by_tag_release_first() {
        let data: GetReleases = serde_json::from_str(RESPONSE).unwrap();
        let candidates = collect_candidates(data.repository);
        assert_eq!(
            candidates.len(),
            4,
            "by-tag result plus every listed release"
        );
        assert_eq!(candidates[0].database_id, Some(333744509));
    }

    #[test]
    fn collect_candidates_tolerates_a_tag_with_no_published_release() {
        // The normal draft-first case: nothing published yet, so `release` is
        // null and only the listing carries the draft.
        let response = r#"{
            "repository": {
                "release": null,
                "releases": {"nodes": [
                    {"databaseId": 42, "tagName": "pcu-v0.7.0", "isDraft": true, "immutable": false}
                ]}
            }
        }"#;
        let data: GetReleases = serde_json::from_str(response).unwrap();
        let candidates = collect_candidates(data.repository);
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].is_draft);
    }
}
