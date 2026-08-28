# pcu-release-assets

Headless clients for a GitHub release's assets — download, upload, and publish — with no git checkout required.

[![Rust 1.89+][version-badge]][version-url]
[![circleci-badge]][circleci-url]
[![Crates.io][crates-badge]][crates-url]
[![Docs][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]
[![APACHE licensed][apache-badge]][apache-url]
[![BuyMeaCoffee][bmac-badge]][bmac-url]
[![GitHubSponsors][ghub-badge]][ghub-url]

[crates-badge]: https://img.shields.io/crates/v/pcu-release-assets.svg
[crates-url]: https://crates.io/crates/pcu-release-assets
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/jerus-org/pcu/blob/main/crates/pcu-release-assets/LICENSE-MIT
[apache-badge]: https://img.shields.io/badge/license-APACHE-blue.svg
[apache-url]: https://github.com/jerus-org/pcu/blob/main/crates/pcu-release-assets/LICENSE-APACHE
[circleci-badge]: https://dl.circleci.com/status-badge/img/gh/jerus-org/pcu/tree/main.svg?style=svg
[circleci-url]: https://dl.circleci.com/status-badge/redirect/gh/jerus-org/pcu/tree/main
[version-badge]: https://img.shields.io/badge/rust-1.89+-orange.svg
[version-url]: https://www.rust-lang.org
[docs-badge]:  https://docs.rs/pcu-release-assets/badge.svg
[docs-url]:  https://docs.rs/pcu-release-assets
[bmac-badge]: https://badgen.net/badge/icon/buymeacoffee?color=yellow&icon=buymeacoffee&label
[bmac-url]: https://buymeacoffee.com/jerusdp
[ghub-badge]: https://img.shields.io/badge/sponsor-30363D?logo=GitHub-Sponsors&logoColor=#white
[ghub-url]: https://github.com/sponsors/jerusdp

`ReleaseAssetClient` (read-only) was extracted from `pcu`'s `Client` (jerus-org/pcu#1051) so a consumer that only needs this one capability — e.g. `jci-audit verify --release-version` fetching a signed audit record from a bare directory, with no clone — does not have to depend on `pcu`'s git/CLI/changelog toolchain to get it. `ReleaseAssetWriter` (jerus-org/pcu#1059) is its write-capable sibling, for a consumer that needs to upload an asset and/or publish the release, still headless.

## Installation

```toml
[dependencies]
pcu-release-assets = "0.1.1"
```

## Design

- No `git2` dependency: neither client touches the filesystem or a git repository (beyond reading the local asset file to upload).
- Capability boundary by type: `ReleaseAssetClient` has no upload or publish method at all — the boundary is enforced by the method not existing, not by a runtime flag. A consumer that only ever verifies/downloads never depends on write capability. `ReleaseAssetWriter` is a separate type for callers that do need to write.
- No draft access on the read side: [`ReleaseAssetClient::download_release_asset`] always resolves to the published release for a tag — there is no `allow_draft` parameter on this entry point. A draft's assets can still be replaced, so a verifier must never trust one. `ReleaseAssetWriter::upload_release_asset` and `publish_release` do work against a draft, since a writer is the one attaching the assets before the draft is published.
- Authentication is optional only for `download_release_asset`: build with [`ReleaseAssetClient::new_unauthenticated`] to read a **public** repo's published release with no `GITHUB_TOKEN` at all (jerus-org/pcu#1064) — that method looks the release up via REST, which serves public repos unauthenticated. Every other entry point — `find_release_for_tag`, `download_release_asset_allowing_draft`, and all of `ReleaseAssetWriter` — needs draft visibility or write access, both of which require a token regardless of repo visibility (draft lookups go through GitHub's GraphQL API, which has no anonymous path at all); calling one of these on an unauthenticated client returns a clear error instead of a confusing 401 from GitHub.

## Usage

```rust,no_run
# async fn demo() -> Result<(), pcu_release_assets::Error> {
use pcu_release_assets::ReleaseAssetClient;

let client = ReleaseAssetClient::new("jerus-org", "jci-audit", std::env::var("GITHUB_TOKEN").unwrap());
let bytes = client
    .download_release_asset("jci-audit-v0.1.0", "release-0.1.0.json")
    .await?;
# Ok(())
# }
```

```rust,no_run
# async fn demo() -> Result<(), pcu_release_assets::Error> {
use pcu_release_assets::ReleaseAssetClient;

// No GITHUB_TOKEN needed — jci-audit is a public repo, and download_release_asset
// is the one entry point that works with no authentication at all.
let client = ReleaseAssetClient::new_unauthenticated("jerus-org", "jci-audit");
let bytes = client
    .download_release_asset("jci-audit-v0.1.0", "release-0.1.0.json")
    .await?;
# Ok(())
# }
```

```rust,no_run
# async fn demo() -> Result<(), pcu_release_assets::Error> {
use pcu_release_assets::ReleaseAssetWriter;
use std::path::Path;

let writer = ReleaseAssetWriter::new("jerus-org", "jci-audit", std::env::var("GITHUB_TOKEN").unwrap());
writer
    .upload_release_asset("jci-audit-v0.1.0", Path::new("release-0.1.0.json"), "release-0.1.0.json")
    .await?;
writer.publish_release("jci-audit-v0.1.0").await?;
# Ok(())
# }
```

## Feature set

- [X] Locate the release for a tag (published or draft, listing both)
- [X] Download a named asset from the **published** release for a tag
- [X] Download a named asset from a public repo with no `GITHUB_TOKEN`
- [X] Upload/replace an asset on a draft or published (non-immutable) release
- [X] Publish (un-draft) a release

[Contributing Guide](https://github.com/jerus-org/pcu/blob/main/CONTRIBUTING.md)

[Code of Conduct](https://github.com/jerus-org/pcu/blob/main/CODE_OF_CONDUCT.md)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
