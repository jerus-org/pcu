#![allow(dead_code)]
use std::{future::Future, pin::Pin, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{Error, GraphQLWrapper};

/// Default number of retry attempts when `associatedPullRequests` returns empty.
const DEFAULT_RETRY_ATTEMPTS: u32 = 3;

/// Default base delay between retry attempts (seconds).
const DEFAULT_BASE_DELAY_SECS: u64 = 5;

/// Configuration for retry-with-exponential-backoff behaviour.
#[derive(Debug, Clone)]
struct RetryConfig {
    /// Maximum number of retry attempts (not counting the initial try).
    max_retries: u32,
    /// Base delay for the first retry.  Subsequent retries double this value.
    base_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_RETRY_ATTEMPTS,
            base_delay: Duration::from_secs(DEFAULT_BASE_DELAY_SECS),
        }
    }
}

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
    body: String,
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

/// The outcome of a single query attempt, used by the retry loop.
#[derive(Debug)]
enum QueryOutcome {
    /// A PR was found.
    Found(i64, String, String, String),
    /// The query succeeded but `associatedPullRequests` returned no nodes, or
    /// the commit object was not yet indexed.  Retrying may help.
    TransientEmpty,
    /// A hard error occurred (auth failure, network error, …).  Do not retry.
    HardError(Error),
}

/// Select the best PR from a non-empty list and return the tuple fields.
fn select_pr(prs: &[PullRequest]) -> Option<(i64, String, String, String)> {
    let pr = prs
        .iter()
        .filter(|pr| pr.merged_at.is_some())
        .max_by_key(|pr| pr.merged_at.as_ref())
        .or_else(|| prs.first())?;
    Some((pr.number, pr.title.clone(), pr.url.clone(), pr.body.clone()))
}

/// Core retry loop — separated from the GraphQL call so that tests can inject
/// a fake query provider without hitting the network.
///
/// `query_fn` is called once per attempt and must return the raw
/// `Result<Data, Error>` that a single GraphQL call would produce.
///
/// Retries only the transient-empty case (empty `associatedPullRequests` or
/// commit object not yet indexed). Hard errors are propagated immediately.
async fn get_pull_request_by_commit_with_retry<F>(
    query_fn: F,
    config: RetryConfig,
) -> Result<(i64, String, String, String), Error>
where
    F: Fn() -> Pin<Box<dyn Future<Output = Result<Data, Error>> + Send>>,
{
    let mut attempt = 0u32;
    let mut delay = config.base_delay;

    loop {
        let outcome = match query_fn().await {
            Err(e) => QueryOutcome::HardError(e),
            Ok(data) => match data.repository.object {
                None => {
                    // Commit not yet indexed — transient, worth retrying.
                    log::debug!(
                        "Commit object not found in GitHub index (attempt {attempt}), will retry"
                    );
                    QueryOutcome::TransientEmpty
                }
                Some(commit) => {
                    let prs = commit.associated_pull_requests.nodes;
                    if prs.is_empty() {
                        log::debug!(
                            "associatedPullRequests returned empty (attempt {attempt}), \
                             will retry"
                        );
                        QueryOutcome::TransientEmpty
                    } else {
                        match select_pr(&prs) {
                            Some(result) => {
                                QueryOutcome::Found(result.0, result.1, result.2, result.3)
                            }
                            None => QueryOutcome::HardError(Error::InvalidMergeCommitMessage),
                        }
                    }
                }
            },
        };

        match outcome {
            QueryOutcome::Found(number, title, url, body) => {
                return Ok((number, title, url, body));
            }
            QueryOutcome::HardError(e) => {
                return Err(e);
            }
            QueryOutcome::TransientEmpty => {
                if attempt >= config.max_retries {
                    log::warn!(
                        "Exhausted {max} retries waiting for associatedPullRequests; \
                         giving up",
                        max = config.max_retries
                    );
                    return Err(Error::InvalidMergeCommitMessage);
                }
                log::info!(
                    "Retrying in {secs}s (attempt {attempt}/{max})",
                    secs = delay.as_secs(),
                    max = config.max_retries,
                );
                tokio::time::sleep(delay).await;
                delay *= 2;
                attempt += 1;
            }
        }
    }
}

/// Get pull request information from a commit SHA
///
/// This works for all merge strategies (merge commit, rebase, squash).
///
/// When `associatedPullRequests` returns an empty list (which can happen
/// transiently in the seconds immediately after a PR merge while GitHub's
/// internal index catches up), the function retries up to
/// [`DEFAULT_RETRY_ATTEMPTS`] times with exponential back-off starting at
/// [`DEFAULT_BASE_DELAY_SECS`] seconds (5 s → 10 s → 20 s).
///
/// Hard errors (network failures, authentication errors, repository not
/// found) are **not** retried.
pub(crate) async fn get_pull_request_by_commit(
    github_graphql: &gql_client::Client,
    owner: &str,
    name: &str,
    commit_sha: &str,
) -> Result<(i64, String, String, String), Error> {
    let query = r#"
            query($owner: String!, $name: String!, $oid: GitObjectID!) {
                repository(owner: $owner, name: $name) {
                    object(oid: $oid) {
                        ... on Commit {
                            associatedPullRequests(first: 5) {
                                nodes {
                                    number
                                    title
                                    body
                                    url
                                    mergedAt
                                }
                            }
                        }
                    }
                }
            }
            "#;

    let owner = owner.to_string();
    let name = name.to_string();
    let oid = commit_sha.to_string();
    let github_graphql = github_graphql.clone();
    let query = query.to_string();

    let query_fn = move || -> Pin<Box<dyn Future<Output = Result<Data, Error>> + Send>> {
        let vars = Vars {
            owner: owner.clone(),
            name: name.clone(),
            oid: oid.clone(),
        };
        let github_graphql = github_graphql.clone();
        let query = query.clone();
        Box::pin(async move {
            let data_res = github_graphql
                .query_with_vars_unwrap::<Data, Vars>(&query, vars)
                .await;

            log::trace!("data_res: {data_res:?}");
            let data = data_res.map_err(GraphQLWrapper::from)?;
            log::trace!("data: {data:?}");
            Ok(data)
        })
    };

    get_pull_request_by_commit_with_retry(query_fn, RetryConfig::default()).await
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use super::*;

    // ---------------------------------------------------------------------------
    // Helper: build a Data value with a specific list of PRs (or None for the
    // commit object being absent).
    // ---------------------------------------------------------------------------

    fn data_with_prs(prs: Vec<PullRequest>) -> Data {
        Data {
            repository: Repository {
                object: Some(Commit {
                    associated_pull_requests: AssociatedPullRequests { nodes: prs },
                }),
            },
        }
    }

    fn data_commit_not_found() -> Data {
        Data {
            repository: Repository { object: None },
        }
    }

    fn make_pr(number: i64) -> PullRequest {
        PullRequest {
            number,
            title: format!("PR #{number}"),
            body: format!("Body of PR #{number}"),
            url: format!("https://github.com/owner/repo/pull/{number}"),
            merged_at: Some("2024-01-01T00:00:00Z".to_string()),
        }
    }

    // ---------------------------------------------------------------------------
    // RED: retry-succeeds-on-second-attempt test
    //
    // Scenario: first call returns empty PR list (transient), second call
    // returns one PR.  The function should eventually return Ok with that PR.
    // ---------------------------------------------------------------------------
    #[tokio::test]
    async fn test_retry_succeeds_on_second_attempt() {
        // Shared call counter.
        let call_count = Arc::new(Mutex::new(0u32));

        let call_count_clone = Arc::clone(&call_count);

        // RetryConfig with zero sleep to keep tests fast.
        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(0),
        };

        let query_fn = move || -> Pin<Box<dyn Future<Output = Result<Data, Error>> + Send>> {
            let call_count = Arc::clone(&call_count_clone);
            Box::pin(async move {
                let mut count = call_count.lock().unwrap();
                *count += 1;
                if *count == 1 {
                    // First attempt: empty PR list — simulates GitHub indexing lag.
                    Ok(data_with_prs(vec![]))
                } else {
                    // Second attempt: PR is now indexed.
                    Ok(data_with_prs(vec![make_pr(42)]))
                }
            })
        };

        let result = get_pull_request_by_commit_with_retry(query_fn, config).await;

        assert!(result.is_ok(), "Expected Ok after retry, got: {result:?}");
        let (number, title, url, body) = result.unwrap();
        assert_eq!(number, 42);
        assert_eq!(title, "PR #42");
        assert_eq!(url, "https://github.com/owner/repo/pull/42");
        assert_eq!(body, "Body of PR #42");

        let final_count = *call_count.lock().unwrap();
        assert_eq!(
            final_count, 2,
            "Expected exactly 2 calls (initial + 1 retry)"
        );
    }

    // ---------------------------------------------------------------------------
    // RED: all-retries-exhausted test
    //
    // Scenario: all attempts return empty PR list.  The function should return
    // Err(InvalidMergeCommitMessage) after exhausting retries.
    // ---------------------------------------------------------------------------
    #[tokio::test]
    async fn test_all_retries_exhausted_returns_error() {
        let call_count = Arc::new(Mutex::new(0u32));
        let call_count_clone = Arc::clone(&call_count);

        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(0),
        };

        let query_fn = move || -> Pin<Box<dyn Future<Output = Result<Data, Error>> + Send>> {
            let call_count = Arc::clone(&call_count_clone);
            Box::pin(async move {
                let mut count = call_count.lock().unwrap();
                *count += 1;
                // Always return empty — simulates GitHub never indexing the PR.
                Ok(data_with_prs(vec![]))
            })
        };

        let result = get_pull_request_by_commit_with_retry(query_fn, config).await;

        assert!(
            result.is_err(),
            "Expected Err after retries exhausted, got: {result:?}"
        );
        assert!(
            matches!(result.unwrap_err(), Error::InvalidMergeCommitMessage),
            "Expected InvalidMergeCommitMessage"
        );

        // Called 4 times: initial attempt + 3 retries.
        let final_count = *call_count.lock().unwrap();
        assert_eq!(
            final_count, 4,
            "Expected 4 calls (initial + 3 retries), got {final_count}"
        );
    }

    // ---------------------------------------------------------------------------
    // RED: commit-not-found also triggers retry
    //
    // Scenario: first attempt returns `object: null` (commit not yet indexed),
    // second attempt returns the commit with a PR.
    // ---------------------------------------------------------------------------
    #[tokio::test]
    async fn test_commit_not_found_retries_and_succeeds() {
        let call_count = Arc::new(Mutex::new(0u32));
        let call_count_clone = Arc::clone(&call_count);

        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(0),
        };

        let query_fn = move || -> Pin<Box<dyn Future<Output = Result<Data, Error>> + Send>> {
            let call_count = Arc::clone(&call_count_clone);
            Box::pin(async move {
                let mut count = call_count.lock().unwrap();
                *count += 1;
                if *count == 1 {
                    Ok(data_commit_not_found())
                } else {
                    Ok(data_with_prs(vec![make_pr(7)]))
                }
            })
        };

        let result = get_pull_request_by_commit_with_retry(query_fn, config).await;

        assert!(result.is_ok(), "Expected Ok, got: {result:?}");
        let (number, ..) = result.unwrap();
        assert_eq!(number, 7);
    }

    // ---------------------------------------------------------------------------
    // RED: hard error is not retried
    //
    // Scenario: query returns a hard error (e.g. auth error).  The function
    // should propagate that error immediately without retrying.
    // ---------------------------------------------------------------------------
    #[tokio::test]
    async fn test_hard_error_is_not_retried() {
        let call_count = Arc::new(Mutex::new(0u32));
        let call_count_clone = Arc::clone(&call_count);

        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(0),
        };

        let query_fn = move || -> Pin<Box<dyn Future<Output = Result<Data, Error>> + Send>> {
            let call_count = Arc::clone(&call_count_clone);
            Box::pin(async move {
                let mut count = call_count.lock().unwrap();
                *count += 1;
                // Return a hard error every time.
                Err(Error::NoGitHubAPIAuth)
            })
        };

        let result = get_pull_request_by_commit_with_retry(query_fn, config).await;

        assert!(result.is_err(), "Expected Err, got: {result:?}");
        assert!(
            matches!(result.unwrap_err(), Error::NoGitHubAPIAuth),
            "Expected NoGitHubAPIAuth"
        );

        let final_count = *call_count.lock().unwrap();
        assert_eq!(
            final_count, 1,
            "Expected only 1 call — no retries for hard errors"
        );
    }

    // ---------------------------------------------------------------------------
    // Deserialization sanity test (mirrors style from get_tag.rs)
    // ---------------------------------------------------------------------------
    #[test]
    fn test_deserialize_response_with_pr() {
        let response = r#"{
            "repository": {
                "object": {
                    "associatedPullRequests": {
                        "nodes": [
                            {
                                "number": 99,
                                "title": "feat: add retry",
                                "body": "Retry logic",
                                "url": "https://github.com/owner/repo/pull/99",
                                "mergedAt": "2024-06-01T12:00:00Z"
                            }
                        ]
                    }
                }
            }
        }"#;

        let data: Data = serde_json::from_str(response).unwrap();
        let prs = &data
            .repository
            .object
            .unwrap()
            .associated_pull_requests
            .nodes;
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 99);
        assert_eq!(prs[0].title, "feat: add retry");
    }

    #[test]
    fn test_deserialize_response_empty_nodes() {
        let response = r#"{
            "repository": {
                "object": {
                    "associatedPullRequests": {
                        "nodes": []
                    }
                }
            }
        }"#;

        let data: Data = serde_json::from_str(response).unwrap();
        let prs = &data
            .repository
            .object
            .unwrap()
            .associated_pull_requests
            .nodes;
        assert!(prs.is_empty());
    }

    #[test]
    fn test_deserialize_response_commit_not_found() {
        let response = r#"{
            "repository": {
                "object": null
            }
        }"#;

        let data: Data = serde_json::from_str(response).unwrap();
        assert!(data.repository.object.is_none());
    }
}
