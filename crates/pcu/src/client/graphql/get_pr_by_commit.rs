#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use crate::{Error, GraphQLWrapper};

#[derive(Deserialize, Debug, Clone)]
struct Data {
    repository: Repository,
}

#[derive(Deserialize, Debug, Clone)]
struct Repository {
    object: Option<Commit>,
}

#[derive(Deserialize, Debug, Clone)]
struct Commit {
    #[serde(rename = "associatedPullRequests")]
    associated_pull_requests: AssociatedPullRequests,
}

#[derive(Deserialize, Debug, Clone)]
struct AssociatedPullRequests {
    nodes: Vec<PullRequest>,
}

#[derive(Deserialize, Debug, Clone)]
struct PullRequest {
    number: i64,
    title: String,
    url: String,
    #[serde(rename = "mergedAt")]
    merged_at: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
struct Vars {
    owner: String,
    name: String,
    oid: String,
}

/// Get pull request information from a commit SHA
///
/// This works for all merge strategies (merge commit, rebase, squash)
pub(crate) async fn get_pull_request_by_commit(
    github_graphql: &gql_client::Client,
    owner: &str,
    name: &str,
    commit_sha: &str,
) -> Result<(i64, String, String), Error> {
    let query = r#"
            query($owner: String!, $name: String!, $oid: GitObjectID!) {
                repository(owner: $owner, name: $name) {
                    object(oid: $oid) {
                        ... on Commit {
                            associatedPullRequests(first: 5) {
                                nodes {
                                    number
                                    title
                                    url
                                    mergedAt
                                }
                            }
                        }
                    }
                }
            }
            "#;

    let vars = Vars {
        owner: owner.to_string(),
        name: name.to_string(),
        oid: commit_sha.to_string(),
    };

    let data_res = github_graphql
        .query_with_vars_unwrap::<Data, Vars>(query, vars)
        .await;

    log::trace!("data_res: {data_res:?}");

    let data = data_res.map_err(GraphQLWrapper::from)?;

    log::trace!("data: {data:?}");

    let commit = data
        .repository
        .object
        .ok_or(Error::InvalidMergeCommitMessage)?;

    let prs = commit.associated_pull_requests.nodes;

    if prs.is_empty() {
        return Err(Error::InvalidMergeCommitMessage);
    }

    // If multiple PRs are associated, prefer the most recently merged one
    let pr = prs
        .iter()
        .filter(|pr| pr.merged_at.is_some())
        .max_by_key(|pr| pr.merged_at.as_ref())
        .or_else(|| prs.first())
        .ok_or(Error::InvalidMergeCommitMessage)?;

    Ok((pr.number, pr.title.clone(), pr.url.clone()))
}
