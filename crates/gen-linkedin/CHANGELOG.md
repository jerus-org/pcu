<!-- LTex: Enabled=false -->
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.29] - 2026-08-19

Summary: 

## [0.1.28] - 2026-08-16

Summary: Chore[1]

## [0.1.27] - 2026-08-12

Summary: Chore[1], Fixed[2]

### Fixed

 - fix(deps): update rust crate toml to 1.1.4
 - fix(deps): update rust crate thiserror to 2.0.20

## [0.1.26] - 2026-07-31

Summary: Chore[1]

## [0.1.25] - 2026-07-23

Summary: Chore[1], Fixed[3]

### Fixed

 - fix(deps): update rust crate toml to 1.1.3
 - fix(deps): update rust crate tokio to 1.53.1
 - fix(deps): update rust crate thiserror to 2.0.19

## [0.1.24] - 2026-06-26

Summary: Chore[1]

## [0.1.23] - 2026-06-26

Summary: Chore[2]

## [0.1.22] - 2026-06-23

Summary: Chore[1]

## [0.1.21] - 2026-06-22

Summary: Chore[1], Fixed[1]

### Fixed

 - fix(deps): update rust crate log to 0.4.33

## [0.1.20] - 2026-06-18

Summary: Chore[1]

## [0.1.19] - 2026-06-18

Summary: Chore[1]

## [0.1.18] - 2026-06-17

Summary: Chore[1]

## [0.1.17] - 2026-06-12

Summary: Chore[1], Fixed[4]

### Fixed

 - fix(deps): update rust crate reqwest to 0.13.4
 - fix(deps): update rust crate log to 0.4.32
 - fix(deps): update rust crate chrono to 0.4.45
 - fix(deps): update rust crate serde_json to 1.0.150

## [0.1.16] - 2026-05-21

Summary: Chore[1]

## [0.1.15] - 2026-05-20

Summary: Added[1], Chore[1], Fixed[3], Testing[1]

### Added

 - feat(gen-linkedin): warn when PCU_LINKEDIN_API_VERSION override is stale

### Fixed

 - fix(deps): update rust crate tokio to 1.52.3
 - fix(deps): update rust crate reqwest to 0.13.3
 - fix(gen-linkedin): update default API version to 202604

## [0.1.14] - 2026-04-18

Summary: Changed[1], Chore[1], Fixed[3]

### Fixed

 - fix(deps): update rust crate tokio to 1.52.1
 - fix(gen-linkedin): make LinkedIn-Version header configurable; fix rustls-webpki CVEs
 - fix(gen-linkedin): add LinkedIn-Version header to posts API requests

### Changed

 - refactor(tests): extract helper to reduce test boilerplate in posts

## [0.1.13] - 2026-04-09

Summary: Chore[1]

## [0.1.12] - 2026-04-07

Summary: Chore[1]

## [0.1.11] - 2026-04-06

Summary: Added[1], Changed[3], Chore[1], Fixed[5]

### Added

 - feat(linkedin): add draft and post subcommands

### Fixed

 - fix(gen-linkedin): use workspace deps for reqwest and wiremock
 - fix(deps): update rust crate tokio to 1.51.0
 - fix(deps): update rust crate toml to 1.1.2
 - fix(deps): update rust crate reqwest to 0.13.2
 - fix(gen-linkedin): remove trailing blank line in test module

### Changed

 - refactor(gen-linkedin): redesign with serde frontmatter and validation
 - refactor(gen-linkedin): parameterise frontmatter helpers by section name
 - refactor(gen-linkedin): extract publish_one to reduce cognitive complexity

## [0.1.10] - 2026-03-28

Summary: Chore[1], Fixed[1]

### Fixed

 - fix(deps): update rust crate toml to 1.1.0

## [0.1.9] - 2026-03-19

Summary: Chore[1], Fixed[1]

### Fixed

 - fix(deps): update rust crate toml to 1.0.7

## [0.1.8] - 2026-03-12

Summary: Chore[1]

## [0.1.7] - 2026-03-11

Summary: Chore[1], Fixed[2]

### Fixed

 - fix(deps): update rust crate tempfile to 3.27.0
 - fix(deps): update rust crate toml to 1.0.6

## [0.1.6] - 2026-03-10

Summary: Chore[1]

## [0.1.5] - 2026-03-09

Summary: Chore[1]

## [0.1.4] - 2026-03-06

Summary: Chore[1], Fixed[1]

### Fixed

 - fix(deps): update rust crate toml to 1.0.4

## [0.1.3] - 2026-03-03

Summary: Chore[1], Fixed[1]

### Fixed

 - fix(deps): update rust crate tokio to 1.50.0

## [0.1.2] - 2026-02-28

Summary: Chore[1], Fixed[2]

### Fixed

 - fix(deps): update rust crate tempfile to 3.26.0
 - fix(deps): update rust crate chrono to 0.4.44

## [0.1.1] - 2026-02-23

Summary: Chore[2], Fixed[12]

### Fixed

 - fix(deps): update rust crate toml to 1.0.3
 - fix(deps): update rust crate toml to v1
 - fix(deps): update rust crate tempfile to 3.25.0
 - fix(deps): update rust crate toml to 0.9.12
 - fix(deps): update rust crate reqwest to 0.13.2
 - fix(deps): update rust crate tokio to 1.49.0
 - fix(deps): update rust crate url to 2.5.8
 - fix(deps): update rust crate toml to 0.9.11
 - fix(deps): update rust crate thiserror to 2.0.18
 - fix(deps): update rust crate serde_json to 1.0.149
 - fix(deps): update rust crate chrono to 0.4.43
 - fix(deps): update rust crate reqwest to 0.13.1

## [0.1.0] - 2026-01-10

Summary: Added[2], Fixed[78]

### Added

 - feat: add per-crate release workflow with independent versioning
 - ✨ feat(linkedin): add gen-linkedin client crate (Posts API)

### Fixed

 - fix: use NEW_VERSION env var in release hooks
 - fix: redirect error messages to stderr in release hooks
 - fix: address SonarQube shell script code smells
 - fix(deps): update rust crate tempfile to 3.24.0
 - fix(deps): update rust crate wiremock to 0.6.5
 - fix(deps): update rust crate toml to 0.9.10
 - fix(deps): update rust crate serde_json to 1.0.147
 - fix(deps): update rust crate reqwest to 0.12.28
 - fix(deps): update rust crate log to 0.4.29
 - 🔒 fix(security): replace httpmock with wiremock to avoid async-std advisory (RUSTSEC-2025-0052)
 - fix(deps): update rust crate tokio to 1.48.0
 - fix(deps): update rust crate tempfile to 3.23.0
 - fix(deps): update rust crate toml to 0.9.8
 - fix(deps): update rust crate thiserror to 2.0.17
 - fix(deps): update rust crate serde to 1.0.228
 - fix(deps): update rust crate url to 2.5.7
 - fix(deps): update rust crate toml to 0.9.7
 - fix(deps): update rust crate log to 0.4.28
 - fix(deps): update rust crate chrono to 0.4.42
 - fix(deps): update rust crate toml to 0.9.5
 - fix(deps): update rust crate tokio to 1.47.1
 - fix(deps): update rust crate thiserror to 2.0.16
 - fix(deps): update rust crate serde_json to 1.0.143
 - fix(deps): update rust crate toml to 0.9.2
 - fix(deps): update rust crate tokio to 1.46.1
 - fix(deps): update rust crate tempfile to 3.20.0
 - fix(deps): update rust crate serde_json to 1.0.141
 - fix(deps): update rust crate url to 2.5.4
 - fix(deps): update rust crate toml to 0.8.23
 - fix(deps): update rust crate toml to 0.8.22
 - fix(deps): update rust crate chrono to 0.4.41
 - fix(deps): update rust crate tokio to 1.44.2
 - fix(deps): update rust crate log to 0.4.27
 - fix(deps): update rust crate log to 0.4.27
 - fix(deps): update rust crate tempfile to 3.19.1
 - fix(deps): update rust crate tokio to 1.44.1
 - fix(deps): update rust crate serde to 1.0.219
 - fix(deps): update rust crate thiserror to 2.0.12
 - fix(deps): update rust crate chrono to 0.4.40
 - fix(deps): update rust crate log to 0.4.26
 - fix(deps): update rust crate serde to 1.0.218
 - fix(deps): update rust crate tokio to 1.43.0
 - fix(deps): update rust crate thiserror to 2.0.11
 - fix(deps): update rust crate log to 0.4.25
 - fix(deps): update rust crate tokio to 1.42.0
 - fix(deps): update rust crate serde to 1.0.217
 - fix(deps): update rust crate thiserror to 2.0.9
 - fix(deps): update rust crate thiserror to 2.0.8
 - fix(deps): update rust crate thiserror to 2.0.6
 - fix(deps): update rust crate serde to 1.0.216
 - fix(deps): update rust crate chrono to 0.4.39
 - fix(deps): update rust crate thiserror to 2.0.4
 - fix(deps): update rust crate thiserror to 2.0.3
 - fix(deps): update rust crate serde to 1.0.215
 - fix(deps): update rust crate thiserror to 2.0.2
 - fix(deps): update rust crate thiserror to 2.0.1
 - fix(deps): update rust crate tokio to 1.41.1
 - fix(deps): update rust crate thiserror to v2
 - fix(deps): update rust crate thiserror to 1.0.68
 - fix(deps): update rust crate url to 2.5.3
 - fix(deps): update rust crate thiserror to 1.0.67
 - fix(deps): update rust crate thiserror to 1.0.66
 - fix(deps): update rust crate serde to 1.0.214
 - fix(deps): update rust crate thiserror to 1.0.65
 - fix(deps): update rust crate serde to 1.0.213
 - fix(deps): update rust crate tokio to 1.41.0
 - fix(deps): update rust crate serde to 1.0.211
 - fix(deps): update rust crate thiserror to 1.0.64
 - fix(deps): update rust crate tokio to 1.40.0
 - fix(deps): update rust crate serde to 1.0.210
 - fix(deps): update rust crate tokio to 1.39.0
 - fix(deps): update rust crate tokio to 1.38.1
 - fix(deps): update rust crate thiserror to 1.0.63
 - fix(deps): update rust crate url to 2.5.2
 - fix(deps): update rust crate tokio to 1.38.0
 - fix(deps): update rust crate log to 0.4.22
 - fix(deps): update rust crate log to v0.4.22
 - fix(deps): update rust crate url to v2.5.2

[Unreleased]: https://github.com/jerus-org/pcu/compare/v0.1.28...HEAD
[0.1.28]: https://github.com/jerus-org/pcu/compare/v0.1.27...v0.1.28
[0.1.27]: https://github.com/jerus-org/pcu/compare/v0.1.26...v0.1.27
[0.1.26]: https://github.com/jerus-org/pcu/compare/v0.1.25...v0.1.26
[0.1.25]: https://github.com/jerus-org/pcu/compare/v0.1.24...v0.1.25
[0.1.24]: https://github.com/jerus-org/pcu/compare/v0.1.23...v0.1.24
[0.1.23]: https://github.com/jerus-org/pcu/compare/v0.1.22...v0.1.23
[0.1.22]: https://github.com/jerus-org/pcu/compare/v0.1.21...v0.1.22
[0.1.21]: https://github.com/jerus-org/pcu/compare/v0.1.20...v0.1.21
[0.1.20]: https://github.com/jerus-org/pcu/compare/v0.1.19...v0.1.20
[0.1.19]: https://github.com/jerus-org/pcu/compare/v0.1.18...v0.1.19
[0.1.18]: https://github.com/jerus-org/pcu/compare/v0.1.17...v0.1.18
[0.1.17]: https://github.com/jerus-org/pcu/compare/v0.1.16...v0.1.17
[0.1.16]: https://github.com/jerus-org/pcu/compare/v0.1.15...v0.1.16
[0.1.15]: https://github.com/jerus-org/pcu/compare/v0.1.14...v0.1.15
[0.1.14]: https://github.com/jerus-org/pcu/compare/v0.1.13...v0.1.14
[0.1.13]: https://github.com/jerus-org/pcu/compare/v0.1.12...v0.1.13
[0.1.12]: https://github.com/jerus-org/pcu/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/jerus-org/pcu/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/jerus-org/pcu/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/jerus-org/pcu/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/jerus-org/pcu/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/jerus-org/pcu/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/jerus-org/pcu/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/jerus-org/pcu/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/jerus-org/pcu/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/jerus-org/pcu/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/jerus-org/pcu/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/jerus-org/pcu/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/jerus-org/pcu/releases/tag/v0.1.0

