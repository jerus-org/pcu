# pcu-release-assets

Read-only client for downloading a named asset from an already-published GitHub release, with no git checkout required.

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

Extracted from `pcu`'s `Client` (jerus-org/pcu#1051) so a consumer that only needs this one capability — e.g. `jci-audit verify --release-version` fetching a signed audit record from a bare directory, with no clone — does not have to depend on `pcu`'s git/CLI/changelog toolchain to get it.

## Installation

```toml
[dependencies]
pcu-release-assets = "0.0.0"
```

## Design

- No `git2` dependency: `ReleaseAssetClient` never touches the filesystem or a git repository.
- No write capability: there is no upload or publish method on this type. The capability boundary is enforced by the method not existing, not by a runtime flag.
- No draft access: [`ReleaseAssetClient::download_release_asset`] always resolves to the published release for a tag — there is no `allow_draft` parameter on this entry point. A draft's assets can still be replaced, so a verifier must never trust one.

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

## Feature set

- [X] Locate the release for a tag (published or draft, listing both)
- [X] Download a named asset from the **published** release for a tag
- [ ] Upload/replace an asset — deliberately out of scope; see `pcu::Client::upload_release_asset` for the write path

[Contributing Guide](https://github.com/jerus-org/pcu/blob/main/CONTRIBUTING.md)

[Code of Conduct](https://github.com/jerus-org/pcu/blob/main/CODE_OF_CONDUCT.md)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
