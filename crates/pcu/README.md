# Pcu

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![circleci-badge]][circleci-url]
[![Rust 1.87+][version-badge]][version-url]
[![FOSSA Status][fossa-badge]][fossa-url]
[![Docs][docs-badge]][docs-url]
[![BuyMeaCoffee][bmac-badge]][bmac-url]
[![GitHubSponsors][ghub-badge]][ghub-url]

[crates-badge]: https://img.shields.io/crates/v/pcu.svg
[crates-url]: https://crates.io/crates/pcu
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/jerusdp/pcu/blob/main/LICENSE
[circleci-badge]: https://dl.circleci.com/status-badge/img/gh/jerus-org/pcu/tree/main.svg?style=svg
[circleci-url]: https://dl.circleci.com/status-badge/redirect/gh/jerus-org/pcu/tree/main
[version-badge]: https://img.shields.io/badge/rust-1.81+-orange.svg
[version-url]: https://www.rust-lang.org
[fossa-badge]: https://app.fossa.com/api/projects/custom%2B22707%2Fgit%40github.com%3Ajerus-org%2Fpcu.git.svg?type=shield&issueType=license
[fossa-url]: (https://app.fossa.com/projects/custom%2B22707%2Fgit%40github.com%3Ajerus-org%2Fpcu.git?ref=badge_shield&issueType=license)

[docs-badge]:  https://docs.rs/pcu/badge.svg
[docs-url]:  https://docs.rs/pcu
[bmac-badge]: https://badgen.net/badge/icon/buymeacoffee?color=yellow&icon=buymeacoffee&label
[bmac-url]: https://buymeacoffee.com/jerusdp
[ghub-badge]: https://img.shields.io/badge/sponsor-30363D?logo=GitHub-Sponsors&logoColor=#white
[ghub-url]: https://github.com/sponsors/jerusdp

A CI utility to update the Unreleased section of the changelog with the title of the pull request and include a link to the pull request.

## Feature set

- [x] Use GitHub as source control system
- [x] Use of CircleCI as CI

## CLI Usage

Install the CLI using cargo install.

```sh

cargo install pcu

```

Run in the CI script following successful completion of build tests.

```console
pcu 

```

The change log will be amended and committed as part of the change, triggering a recheck. On the recheck pcu will exit early as the change has already been applied.

Help provides all the options

```sh

$ pcu -h
A CI tool to update change log in a PR

Usage: pcu [OPTIONS]

Options:
  -v, --verbose...   Increase logging verbosity
  -q, --quiet...     Decrease logging verbosity
  -s, --sign <SIGN>  [possible values: gpg, none]
  -h, --help         Print help
  -V, --version      Print version
$

```

## License

 Licensed under either of

- Apache License, Version 2.0 (LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license (LICENSE-MIT or <http://opensource.org/licenses/MIT>)
 at your option.

## Contribution

 Unless you explicitly state otherwise, any contribution intentionally submitted
 for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
 dual licensed as above, without any additional terms or conditions.
