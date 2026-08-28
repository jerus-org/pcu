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
    /// Assets cannot be attached to a release that is already published on a
    /// repository with immutable releases enabled.
    ///
    /// GitHub freezes a release's assets at publication, so neither an upload
    /// nor the delete-then-replace retry can succeed. Immutability is
    /// default-on for repositories created recently. Message text mirrors
    /// `pcu::Error::ImmutableRelease` (jerus-org/pcu#1027's precedent for
    /// this exact failure mode).
    #[error(
        "release '{0}' is already published and its assets are immutable, so they cannot be \
         added or replaced. Create the release as a draft, attach every asset, then publish it \
         last. This release cannot be repaired in place — release the next patch version \
         instead. GitHub said: {1}"
    )]
    ImmutableRelease(String, String),
}
