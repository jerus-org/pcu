//! Headless clients for a GitHub release's assets, with no git checkout
//! required.
//!
//! [`ReleaseAssetClient`] is read-only by design — extracted from `pcu`'s
//! `Client` (see jerus-org/pcu#1051) so a consumer that only ever needs to
//! download an asset — e.g. `jci-audit verify --release-version` fetching a
//! signed audit record from a bare directory — does not have to depend on
//! `pcu`'s git/CLI/changelog machinery to get it. [`ReleaseAssetWriter`]
//! (jerus-org/pcu#1059) is its write-capable sibling, for a consumer that
//! needs to upload an asset and/or publish the release, still headless.

mod client;
mod error;
mod writer;

pub use client::{ReleaseAssetClient, ReleaseRef};
pub use error::Error;
pub use writer::ReleaseAssetWriter;
