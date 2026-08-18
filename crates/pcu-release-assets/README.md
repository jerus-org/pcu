# pcu-release-assets

Read-only client for downloading a named asset from an already-published GitHub release, with no git checkout required.

Extracted from `pcu`'s `Client` (jerus-org/pcu#1051) so a consumer that only needs this one capability — e.g. `jci-audit verify --release-version` fetching a signed audit record from a bare directory, with no clone — does not have to depend on `pcu`'s git/CLI/changelog toolchain to get it.

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
