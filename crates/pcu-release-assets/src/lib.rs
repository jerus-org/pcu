//! Read-only client for downloading a named asset from an already-published
//! GitHub release, with no git checkout required.
//!
//! Extracted from `pcu`'s `Client` (see jerus-org/pcu#1051) so a consumer
//! that only ever needs this one capability — e.g. `jci-audit verify
//! --release-version` fetching a signed audit record from a bare directory —
//! does not have to depend on `pcu`'s git/CLI/changelog machinery to get it.

mod client;
mod error;

pub use client::{ReleaseAssetClient, ReleaseRef};
pub use error::{Error, GraphQLWrapper};
