use std::fmt::Display;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Octocrate says: {0:?}")]
    Octocrate(#[from] octocrate::Error),
    #[error("GraphQL says: {0:?}")]
    GraphQL(#[from] GraphQLWrapper),
    /// Catch-all for release/asset lookup failures raised by this crate
    /// itself (not-found, still-a-draft, bad HTTP status) — pure so the
    /// message is unit-testable without a network round trip.
    #[error("{0}")]
    ReleaseAsset(String),
}

#[derive(Debug)]
pub struct GraphQLWrapper(gql_client::GraphQLError);

impl std::error::Error for GraphQLWrapper {
    fn description(&self) -> &str {
        "A GraphQL error occurred"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        None
    }
}

impl From<gql_client::GraphQLError> for GraphQLWrapper {
    fn from(err: gql_client::GraphQLError) -> Self {
        GraphQLWrapper(err)
    }
}

impl Display for GraphQLWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
