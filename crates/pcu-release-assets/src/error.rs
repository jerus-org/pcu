use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Octocrate says: {0:?}")]
    Octocrate(#[from] octocrate::Error),
    /// Catch-all for GraphQL failures and release/asset lookup failures
    /// raised by this crate itself (not-found, still-a-draft, bad HTTP
    /// status) — a plain string so callers don't need a wrapper type just to
    /// give `gql_client::GraphQLError` (which already implements `Display`)
    /// a home in this enum.
    #[error("{0}")]
    ReleaseAsset(String),
}
