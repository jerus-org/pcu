<!-- LTex: Enabled=false -->
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.32] - 2026-08-16

Summary: Added[1], Fixed[1]

### Added

 - feat(release): add download_release_asset to Client

### Fixed

 - fix(release): gate download_release_asset on draft

## [0.6.31] - 2026-08-12

Summary: Added[1], Changed[1], Chore[1], Fixed[8]

### Added

 - feat: gate bsky and linkedin behind features

### Fixed

 - fix(release): add missing binstall signing scaffold to pcu
 - fix(deps): update rust crate config to 0.15.25
 - fix(deps): update rust crate toml to 1.1.4
 - fix(deps): update rust crate thiserror to 2.0.20
 - fix(deps): update rust crate kdeets to 0.1.32
 - fix(deps): update rust crate clap to 4.6.6
 - fix(deps): update rust crate base64 to 0.23.1
 - fix: dedupe crate-absence checks, add tracking issues

### Changed

 - refactor: simplify feature-gate guard and settings

## [0.6.30] - 2026-07-31

Summary: Added[1], Changed[1], Chore[2], Fixed[4]

### Added

 - feat: gate the attestation surface behind a feature

### Fixed

 - fix: stage absolute paths instead of silently ignoring them
 - fix: prefer a published release over a stale draft
 - fix: support immutable releases via draft-first flow
 - fix(deps): update rust crate base64 to 0.23.0

### Changed

 - refactor: fetch release candidates in one GraphQL query

## [0.6.29] - 2026-07-23

Summary: Added[1], Chore[1], Fixed[8]

### Added

 - feat: default `pcu release` prlog commit to ci-skip

### Fixed

 - fix(deps): update rust crate uuid to 1.24.0
 - fix(deps): update rust crate toml to 1.1.3
 - fix(deps): update rust crate tokio to 1.53.1
 - fix(deps): update rust crate thiserror to 2.0.19
 - fix(deps): update rust crate regex to 1.13.1
 - fix(deps): update rust crate kdeets to 0.1.31
 - fix(deps): update rust crate clap to 4.6.4
 - fix(release): fail on missing binary signing scaffold

## [0.6.28] - 2026-06-26

Summary: Added[3], Changed[1], Chore[2], Fixed[6]

### Added

 - feat: make pr ci-skip flag negatable too
 - feat: make release ci-skip flag negatable
 - feat: controllable ci-skip on the release prlog commit

### Fixed

 - fix(deps): update rust crate cargo_toml to v1
 - fix(deps): update rust crate uuid to 1.23.4
 - fix(deps): update rust crate env_logger to 0.11.11
 - fix: pcu owns a safe, well-named release push
 - fix: ensure_github_release never reports false success
 - fix: retry tag visibility before skipping release

### Changed

 - refactor: move test modules to end of release.rs

## [0.6.27] - 2026-06-23

Summary: Chore[1]

## [0.6.26] - 2026-06-22

Summary: Chore[1], Fixed[2], Testing[1]

### Fixed

 - fix(deps): update rust crate log to 0.4.33
 - fix(deps): update rust crate link-bridge to 0.2.6

## [0.6.25] - 2026-06-18

Summary: Chore[1], Fixed[2]

### Fixed

 - fix: only add ci-skip marker on default-branch prlog
 - fix: pr commit uses configured message (skip-ci)

## [0.6.24] - 2026-06-18

Summary: Chore[1], Fixed[1]

### Fixed

 - fix: enable git2 https+ssh features (TLS)

## [0.6.23] - 2026-06-17

Summary: Added[1], Chore[2]

### Added

 - feat: add --skip-ci to pcu pr

## [0.6.22] - 2026-06-12

Summary: Added[3], Chore[1], Fixed[9]

### Added

 - feat: explicit GPG signing key on SignConfig
 - feat: idempotent upload_release_asset (delete-then-replace)
 - feat: explicit commit identity on SignConfig

### Fixed

 - fix(deps): update rust crate regex to 1.12.4
 - fix(deps): update rust crate uuid to 1.23.3
 - fix(deps): update rust crate unicode-segmentation to 1.13.3
 - fix(deps): update rust crate reqwest to 0.13.4
 - fix: adapt to git2 0.21 breaking string-method changes
 - fix(deps): update rust crate log to 0.4.32
 - fix(deps): update rust crate chrono to 0.4.45
 - fix(deps): update rust crate sigstore to 0.14.0
 - fix(deps): update rust crate serde_json to 1.0.150

## [0.6.21] - 2026-05-21

Summary: Chore[1], Fixed[1]

### Fixed

 - fix(git_ops): stage_paths handles directory paths

## [0.6.20] - 2026-05-20

Summary: Added[2], Changed[1], Chore[1], Fixed[6]

### Added

 - feat: expose library APIs for GPG import, staged paths, local client, and release asset upload
 - feat(gen-linkedin): warn when PCU_LINKEDIN_API_VERSION override is stale

### Fixed

 - fix(deps): update rust crate tokio to 1.52.3
 - fix(deps): update rust crate reqwest to 0.13.3
 - fix(docs): resolve intra-doc links in new_local methods
 - fix(clippy): replace sort_by with sort_by_key
 - fix(gen-linkedin): update default API version to 202604
 - fix(logging): add gen_linkedin to log filter

### Changed

 - refactor: eliminate code duplication flagged by SonarQube

## [0.6.19] - 2026-04-18

Summary: Chore[1], Fixed[6]

### Fixed

 - fix(deps): update rust crate tokio to 1.52.1
 - fix(deps): update rust crate uuid to 1.23.1
 - fix(deps): update rust crate sigstore_protobuf_specs to 0.5.1
 - fix(deps): update rust crate clap to 4.6.1
 - fix: use PCU_ prefix in error messages; document audit ignore policy
 - fix(gen-linkedin): make LinkedIn-Version header configurable; fix rustls-webpki CVEs

## [0.6.18] - 2026-04-09

Summary: Changed[1], Chore[1], Fixed[1]

### Fixed

 - fix(push): rewrite SSH remote to HTTPS when App token is present

### Changed

 - refactor(push): extract make_credential to reduce cognitive complexity

## [0.6.17] - 2026-04-07

Summary: Chore[1], Fixed[1]

### Fixed

 - fix(release): retry get_release_by_tag on 404

## [0.6.16] - 2026-04-06

Summary: Added[8], Changed[3], Chore[1], Fixed[9], Testing[2]

### Added

 - feat(create-issue): add label support with ci-created default
 - feat: add comment-pr subcommand to post a PR comment
 - feat(cli): add create-issue subcommand
 - feat(linkedin): add draft and post subcommands
 - feat: add --from-branch flag to bsky draft
 - feat: add --push flag to bsky draft
 - feat: add pcu trigger --webhook subcommand
 - feat: use kdeets_lib for crate version check

### Fixed

 - fix(deps): update rust crate uuid to 1.23.0
 - fix(deps): update rust crate unicode-segmentation to 1.13.2
 - fix(deps): update rust crate tokio to 1.51.0
 - fix(deps): update rust crate sha2 to 0.11.0
 - fix: sha2 0.11 digest LowerHex removed
 - fix(deps): update rust crate toml to 1.1.2
 - fix(deps): update rust crate link-bridge to 0.2.5
 - fix(deps): update rust crate reqwest to 0.13.2
 - fix(linkedin): use lowercase config keys for access token and author URN

### Changed

 - refactor: extract resolve_owner/resolve_repo to remove duplication
 - refactor: move test module to bottom of cmd_draft
 - refactor: replace curl subprocess with reqwest in attest

## [0.6.15] - 2026-03-28

Summary: Chore[1], Fixed[3]

### Fixed

 - fix: make TagTarget pub to match public trait interface
 - fix(deps): update rust crate env_logger to 0.11.10
 - fix(deps): update rust crate toml to 1.1.0

## [0.6.14] - 2026-03-19

Summary: Chore[1], Fixed[5]

### Fixed

 - fix(deps): update rust crate clap to 4.6.0
 - fix(deps): update rust crate tracing-subscriber to 0.3.23
 - fix(deps): update rust crate toml to 1.0.7
 - fix(deps): update rust crate base62 to 2.2.4
 - fix: GPG-sign workspace version tags

## [0.6.13] - 2026-03-12

Summary: Chore[1], Fixed[2]

### Fixed

 - fix(deps): update rust crate link-bridge to 0.2.4
 - fix(attest): skip upload when assets already exist on GitHub release

## [0.6.12] - 2026-03-11

Summary: Chore[1], Fixed[4]

### Fixed

 - fix(deps): update rust crate tempfile to 3.27.0
 - fix(deps): update rust crate toml to 1.0.6
 - fix(deps): update rust crate openidconnect to 4.0.1
 - fix: make release_package idempotent when GitHub release exists

## [0.6.11] - 2026-03-10

Summary: Chore[1], Fixed[1]

### Fixed

 - fix: pem_to_der extracts only the leaf cert from a chain

## [0.6.10] - 2026-03-09

Summary: Chore[1], Fixed[1]

### Fixed

 - fix: use Fulcio v1 API for SLSA attestation

## [0.6.9] - 2026-03-06

Summary: Added[1], Changed[1], Chore[1], Fixed[8], Testing[4]

### Added

 - feat: add pcu release attest command

### Fixed

 - fix(deps): update rust crate uuid to 1.22.0
 - fix(deps): update rust crate toml to 1.0.4
 - fix(deps): update rust crate sha2 to 0.10.9
 - fix(deps): update rust crate base64 to 0.22.1
 - fix: retry get_pull_request_by_commit when empty
 - fix(862): list external contributors in report
 - fix(144): skip PR link when description is empty
 - fix(813): add --allow-empty flag to bsky draft

### Changed

 - refactor: extract classify helpers to reduce complexity

## [0.6.8] - 2026-03-03

Summary: Chore[1], Fixed[2]

### Fixed

 - fix(deps): update rust crate tokio to 1.50.0
 - fix: add app/renovate to default label authors

## [0.6.7] - 2026-02-28

Summary: Added[1], Changed[1], Chore[1], Fixed[2]

### Added

 - feat: add checkout subcommand

### Fixed

 - fix(deps): update rust crate tempfile to 3.26.0
 - fix(deps): update rust crate chrono to 0.4.44

### Changed

 - refactor: address review comments on checkout subcommand

## [0.6.6] - 2026-02-23

Summary: Added[1], Documentation[1], Fixed[5], Testing[1]

### Added

 - feat(cli): add release utility subcommands

### Fixed

 - fix(deps): update rust crate owo-colors to 4.3.0
 - fix(deps): update rust crate toml to 1.0.3
 - fix(deps): update rust crate clap to 4.5.60
 - fix(deps): update rust crate uuid to 1.21.0
 - fix(cli): move tests module to end of release.rs

## [0.6.5] - 2026-02-23

Summary: Fixed[17]

### Fixed

 - fix(verify): include subkey IDs in GPG trust map
 - fix(auth): warn when falling back to PAT
 - fix: sync pcu version to 0.6.4 (matches crates.io)
 - fix(git): use stored token for HTTPS push auth
 - fix(pr): use git config identity in push error message
 - fix(pr): include push identity in rejection error
 - fix(git): log push URL and credential type
 - fix(pr): detect push rejection via fetch-and-recheck
 - fix: detect silent push rejection via branch-ahead check
 - fix(deps): update rust crate toml to v1
 - fix(deps): update rust crate tempfile to 3.25.0
 - fix(deps): update rust crate toml to 0.9.12
 - fix(deps): update rust crate reqwest to 0.13.2
 - fix(deps): update rust crate regex to 1.12.3
 - fix(deps): update rust crate env_logger to 0.11.9
 - fix(deps): update rust crate clap to 4.5.58
 - fix(deps): bump MSRV to 1.88 for time security fix

## [0.6.4] - 2026-02-01

Summary: Added[1], Changed[1], Fixed[16]

### Added

 - feat: add per-crate release workflow with independent versioning

### Fixed

 - fix(deps): update rust crate uuid to 1.20.0
 - fix(deps): update rust crate gql_client to 1.1.0
 - fix(deps): update rust crate clap to 4.5.56
 - fix(deps): update rust crate tokio to 1.49.0
 - fix(deps): update rust crate url to 2.5.8
 - fix(deps): update rust crate toml to 0.9.11
 - fix(deps): update rust crate thiserror to 2.0.18
 - fix(deps): update rust crate serde_json to 1.0.149
 - fix(deps): update rust crate clap to 4.5.54
 - fix(deps): update rust crate chrono to 0.4.43
 - fix(deps): update rust crate reqwest to 0.13.1
 - fix: use correct authors list in label_next_pr filter
 - fix: use NEW_VERSION env var in release hooks
 - fix: reset pcu version to match crates.io
 - fix: redirect error messages to stderr in release hooks
 - fix: address SonarQube shell script code smells

### Changed

 - ♻️ refactor(client): streamline pull request title update

## [0.6.3] - 2026-01-06

Summary: Added[10], Changed[7], Chore[1], Documentation[2], Fixed[11], Testing[1]

### Added

 - ✨ feat: support read-only token for forked PR verification
 - ✨ feat: update existing PR comments instead of creating duplicates
 - ✨ feat: add PR comment reporting for signature verification
 - ✅ feat: complete verify-signatures integration
 - ✅ feat: add trust_fetcher module for GitHub API integration
 - ✅ feat: add git commit extraction with signature info
 - 🚀 feat: add verify-signatures subcommand scaffolding
 - ✨ feat: add --from-merge flag to support PR log updates on main branch
 - ✨ feat(linkedin): add linkedin share subcommand to pcu
 - ✨ feat: implement commit message signoff with --no-signoff flag

### Fixed

 - fix(deps): update rust crate tempfile to 3.24.0
 - fix(deps): update rust crate uuid to 1.19.0
 - fix(deps): update rust crate toml to 0.9.10
 - fix(deps): update rust crate serde_json to 1.0.147
 - fix(deps): update rust crate reqwest to 0.12.28
 - fix(deps): update rust crate log to 0.4.29
 - fix(deps): update rust crate git2 to 0.20.3
 - 🐛 fix: use octocrate issues API instead of gh CLI for PR comments
 - 🐛 fix: import GPG keys into system keyring for git verification
 - 🐛 fix: return error on verification failure for proper exit code
 - 🐛 fix: handle commits without associated PRs gracefully in from-merge mode

### Changed

 - ♻️ refactor: reduce run_verify cognitive complexity from 19 to <15
 - ♻️ refactor: reduce code duplication in PR comment generation
 - ♻️ refactor: optimize trust_fetcher by passing mutable reference
 - ♻️ refactor: reduce cognitive complexity in verify_commit
 - ♻️ refactor: reduce cognitive complexity of run_pull_request
 - ♻️ refactor(linkedin): factor helpers and integrate --linkedin-share into release flow
 - ♻️ refactor(bsky): use config builder for settings overrides

## [0.6.2] - 2025-11-28

Summary: Chore[1], Fixed[4]

### Fixed

 - fix(deps): update rust crate tokio to 1.48.0
 - fix(deps): update rust crate tempfile to 3.23.0
 - fix(deps): update rust crate rstest to 0.26.1
 - fix(deps): update rust crate clap to 4.5.53

## [0.6.1] - 2025-10-28

Summary: Added[1], Changed[1], Chore[2], Documentation[1], Fixed[15]

### Added

 - ✨ feat(cli): support multiple authors in label command

### Fixed

 - fix(deps): update rust crate regex to 1.12.2
 - fix(deps): update rust crate toml to 0.9.8
 - fix(deps): update rust crate thiserror to 2.0.17
 - fix(deps): update rust crate serde to 1.0.228
 - fix(deps): update rust crate owo-colors to 4.2.3
 - fix(deps): update rust crate clap to 4.5.50
 - fix(deps): update rust crate uuid to 1.18.1
 - fix(deps): update rust crate url to 2.5.7
 - fix(deps): update rust crate tracing-subscriber to 0.3.20
 - fix(deps): update rust crate toml to 0.9.7
 - fix(deps): update rust crate regex to 1.11.2
 - fix(deps): update rust crate log to 0.4.28
 - fix(deps): update rust crate clap to 4.5.48
 - fix(deps): update rust crate chrono to 0.4.42
 - fix(deps): update rust crate base62 to 2.2.3

### Changed

 - ♻️ refactor(git_ops): enhance PR filtering by authors

## [0.6.0] - 

Summary: Added[11], Build[1], Changed[40], Chore[15], Documentation[7], Fixed[148], Security[1]

### Added

 - ✨ feat(cli): save settings to file upon successful build
 - ✨ feat(error): add new PostError variant
 - ✨ feat(cli): enhance site config path handling
 - ✨ feat(cli): add www_src_root parameter to CmdDraft
 - ✨ feat(cli): support multiple paths for CmdDraft
 - ✨ feat(pcu): enable serde feature for url crate
 - ✨ feat(logging): add logging filter for gen_bsky module
 - ✨ feat(cli): add trace logging for key parameters in cmd_draft
 - ✨ feat(cli): add site_config management in bsky draft command
 - ✨ feat(error): add new error variant for front matter
 - ✨ feat(pcu): add Cargo.toml for pcu crate

### Fixed

 - 🐛 fix(cli): correct configuration key for prlog setting
 - 🐛 fix(git_ops): re-enable credential callback for git operations
 - 🐛 fix(make_release): correct options variable name
 - 🐛 fix(update_from_pr): correct changelog variable usage
 - 🐛 fix(make_release): correct error handling in changelog parsing
 - 🐛 fix(cli): correct default log file setting
 - fix(deps): update rust crate tracing-subscriber to v0.3.20 [security]
 - fix(deps): update rust crate toml to 0.9.5
 - fix(deps): update rust crate tokio to 1.47.1
 - fix(deps): update rust crate thiserror to 2.0.16
 - fix(deps): update rust crate serde_json to 1.0.143
 - fix(deps): update rust crate clap-verbosity-flag to 3.0.4
 - fix(deps): update rust crate clap to 4.5.45
 - fix(deps): update rust crate cargo_toml to 0.22.3
 - 🐛 fix(cli): correct date handling in draft command
 - 🐛 fix(cli): handle file read errors gracefully
 - 🐛 fix(cmd_draft): add await for async bluesky post writing
 - 🐛 fix(cli): correct path handling in cmd_draft
 - 🐛 fix(cli): fix borrow issues in command execution
 - fix(deps): update rust crate uuid to 1.17.0
 - fix(deps): update rust crate toml to 0.9.2
 - fix(deps): update rust crate tokio to 1.46.1
 - fix(deps): update rust crate tempfile to 3.20.0
 - fix(deps): update rust crate serde_json to 1.0.141
 - fix(deps): update rust crate config to 0.15.13
 - fix(deps): update rust crate clap to 4.5.41
 - fix(deps): update rust crate url to 2.5.4
 - fix(deps): update rust crate toml to 0.8.23
 - fix(deps): update rust crate owo-colors to 4.2.2
 - fix(deps): update rust crate gql_client to 1.0.8
 - fix(deps): update rust crate git2 to 0.20.2
 - fix(deps): update rust crate color-eyre to 0.6.5
 - fix(deps): update rust crate clap-verbosity-flag to 3.0.3
 - fix(deps): update rust crate clap to 4.5.40
 - fix(deps): update rust crate toml to 0.8.22
 - fix(deps): update rust crate chrono to 0.4.41
 - fix(deps): update rust crate clap to 4.5.37
 - fix(deps): update rust crate tokio to 1.44.2
 - fix(deps): update rust crate clap to 4.5.36
 - fix(deps): update rust crate env_logger to 0.11.8
 - fix(deps): update rust crate clap to 4.5.35
 - fix(deps): update rust crate log to 0.4.27
 - fix(deps): update rust crate log to 0.4.27
 - fix(deps): update rust crate clap to 4.5.34
 - fix(deps): update rust crate tempfile to 3.19.1
 - fix(deps): update rust crate git2 to 0.20.1
 - fix(deps): update rust crate uuid to 1.16.0
 - fix(deps): update rust crate tokio to 1.44.1
 - fix(deps): update rust crate cargo_toml to 0.22.1
 - fix(deps): update rust crate serde to 1.0.219
 - fix(deps): update rust crate config to 0.15.11
 - fix(deps): update rust crate clap to 4.5.32
 - fix(deps): update rust crate env_logger to 0.11.7
 - fix(deps): update rust crate rstest to 0.25.0
 - fix(deps): update rust crate thiserror to 2.0.12
 - fix(deps): update rust crate config to 0.15.9
 - fix(deps): update rust crate uuid to 1.15.1
 - fix(deps): update rust crate owo-colors to 4.2.0
 - fix(deps): update rust crate clap to 4.5.31
 - fix(deps): update rust crate chrono to 0.4.40
 - fix(deps): update rust crate uuid to 1.14.0
 - fix(deps): update rust crate log to 0.4.26
 - fix(deps): update rust crate octocrate to 2.2.0
 - fix(deps): update rust crate serde to 1.0.218
 - fix(deps): update rust crate clap to 4.5.30
 - fix(deps): update rust crate config to 0.15.8
 - fix(deps): update rust crate clap to 4.5.29
 - fix(deps): update rust crate uuid to 1.13.1
 - fix(deps): update rust crate clap to 4.5.28
 - fix(deps): update rust crate uuid to 1.12.1
 - fix(deps): update rust crate config to 0.15.7
 - fix(deps): update rust crate tokio to 1.43.0
 - fix(deps): update rust crate git2 to 0.20.0
 - fix(deps): update rust crate clap to 4.5.27
 - fix(deps): update rust crate thiserror to 2.0.11
 - fix(deps): update rust crate log to 0.4.25
 - fix(deps): update rust crate config to 0.15.6
 - fix(deps): update rust crate clap to 4.5.26
 - fix(deps): update rust crate rstest to 0.24.0
 - fix(deps): update rust crate tokio to 1.42.0
 - fix(deps): update rust crate serde to 1.0.217
 - fix(deps): update rust crate thiserror to 2.0.9
 - fix(deps): update rust crate env_logger to 0.11.6
 - fix(deps): update rust crate config to 0.15.4
 - fix(deps): update rust crate config to 0.15.3
 - fix(deps): update rust crate thiserror to 2.0.8
 - fix(deps): update rust crate clap-verbosity-flag to 3.0.2
 - fix(deps): update rust crate thiserror to 2.0.6
 - fix(deps): update rust crate serde to 1.0.216
 - fix(deps): update rust crate chrono to 0.4.39
 - fix(deps): update rust crate cargo_toml to 0.21.0
 - fix(deps): update rust crate tracing-subscriber to 0.3.19
 - fix(deps): update rust crate thiserror to 2.0.4
 - fix(deps): update rust crate clap to 4.5.23
 - fix(deps): update rust crate tracing to 0.1.41
 - fix(deps): update rust crate clap-verbosity-flag to 3.0.1
 - fix(deps): update rust crate clap-verbosity-flag to v3
 - fix(deps): update rust crate clap-verbosity-flag to 2.2.3
 - fix(deps): update rust crate clap to 4.5.21
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
 - fix(deps): update rust crate regex to 1.11.1
 - fix(deps): update rust crate config to 0.14.1
 - fix(deps): update rust crate thiserror to 1.0.65
 - fix(deps): update rust crate serde to 1.0.213
 - fix(deps): update rust crate tokio to 1.41.0
 - fix(deps): update rust crate serde to 1.0.211
 - fix(deps): update rust crate uuid to 1.11.0
 - fix(deps): update rust crate clap to 4.5.20
 - fix(deps): update rust crate clap to 4.5.19
 - fix(deps): update rust crate regex to 1.11.0
 - fix(deps): update rust crate rstest to 0.23.0
 - fix(deps): update rust crate clap-verbosity-flag to 2.2.2
 - fix(deps): update rust crate clap to 4.5.18
 - fix(deps): update rust crate thiserror to 1.0.64
 - fix(deps): update rust crate tokio to 1.40.0
 - fix(deps): update rust crate serde to 1.0.210
 - fix(deps): update rust crate clap to 4.5.17
 - fix(deps): update rust crate regex to 1.10.6
 - fix(deps): update rust crate env_logger to 0.11.5
 - fix(deps): update rust crate clap-verbosity-flag to 2.2.1
 - fix(deps): update rust crate clap to 4.5.15
 - fix(deps): update rust crate rstest to 0.22.0
 - fix(deps): update rust crate tokio to 1.39.0
 - fix(deps): update rust crate tokio to 1.38.1
 - fix(deps): update rust crate thiserror to 1.0.63
 - fix(deps): update rust crate clap to 4.5.9
 - fix(deps): update rust crate keep-a-changelog to 0.1.4
 - fix(deps): update rust crate uuid to 1.10.0
 - fix(deps): update rust crate uuid to 1.9.1
 - fix(deps): update rust crate url to 2.5.2
 - fix(deps): update rust crate regex to 1.10.5
 - fix(deps): update rust crate tokio to 1.38.0
 - fix(deps): update rust crate clap to 4.5.8
 - fix(deps): update rust crate log to 0.4.22
 - fix(deps): update rust crate clap to v4.5.8
 - fix(deps): update rust crate log to v0.4.22
 - fix(deps): update rust crate url to v2.5.2
 - fix(deps): update rust crate git2 to 0.19.0

### Changed

 - ♻️ refactor(prlog): rename changelog to prlog
 - ♻️ refactor(cli): rename update_changelog to update_prlog
 - ♻️ refactor(update_from_pr): rename changelog to prlog
 - ♻️ refactor(pr_title): rename changelog to prlog
 - ♻️ refactor(make_release): rename changelog to prlog
 - ♻️ refactor(cli): rename changelog to prlog in release process
 - ♻️ refactor(client): rename changelog to prlog
 - ♻️ refactor(cli): rename changelog function to prlog
 - ♻️ refactor(cli): remove unnecessary file write operation
 - ♻️ refactor(git_ops): improve reference handling in GitOps
 - ♻️ refactor(update_from_pr): rename changelog to prlog
 - ♻️ refactor(client): rename changelog fields to prlog
 - ♻️ refactor(cli): add trace log for initial settings
 - ♻️ refactor(tests): rename changelog files to prlog
 - ♻️ refactor(client)!: rename changelog to prlog
 - ♻️ refactor(cmd_draft): streamline post writing logic
 - ♻️ refactor(ops): simplify git_ops module import
 - ♻️ refactor(git_ops): restructure import statements
 - ♻️ refactor(graphql): reorder imports for clarity
 - ♻️ refactor(graphql): reorder imports for clarity
 - ♻️ refactor(graphql): reorder imports for clarity
 - ♻️ refactor(cli): reorder imports for better organization
 - ♻️ refactor(lib): simplify import statements
 - ♻️ refactor(client): reorder imports for clarity
 - ♻️ refactor(cli): reorder imports for clarity
 - ♻️ refactor(cmd_post): streamline post and delete process
 - ♻️ refactor(error): remove unused error variants
 - ♻️ refactor(cli): update post command dependencies
 - ♻️ refactor(post): rename and restructure post module
 - ♻️ refactor(cli): enhance draft command flexibility
 - ♻️ refactor(cli): update site configuration initialization
 - ♻️ refactor(poster): simplify error handling in post creation
 - ♻️ refactor(site_config): use Url type for base_url
 - ♻️ refactor(error): remove unused FrontMatterError variant
 - ♻️ refactor(cli): update method for path handling in draft command
 - ♻️ refactor(cli): enhance draft processing and configuration
 - ♻️ refactor(error): update error enum definitions
 - ♻️ refactor(draft): simplify bluesky record writing process
 - ♻️ refactor(draft): improve post processing logic
 - ♻️ refactor(draft): update front matter module import

### Security

 - chore(deps): update rust crate rstest to 0.21.0

[Unreleased]: https://github.com/jerus-org/pcu/compare/v0.6.31...HEAD
[0.6.31]: https://github.com/jerus-org/pcu/compare/v0.6.30...v0.6.31
[0.6.30]: https://github.com/jerus-org/pcu/compare/v0.6.29...v0.6.30
[0.6.29]: https://github.com/jerus-org/pcu/compare/v0.6.28...v0.6.29
[0.6.28]: https://github.com/jerus-org/pcu/compare/v0.6.27...v0.6.28
[0.6.27]: https://github.com/jerus-org/pcu/compare/v0.6.26...v0.6.27
[0.6.26]: https://github.com/jerus-org/pcu/compare/v0.6.25...v0.6.26
[0.6.25]: https://github.com/jerus-org/pcu/compare/v0.6.24...v0.6.25
[0.6.24]: https://github.com/jerus-org/pcu/compare/v0.6.23...v0.6.24
[0.6.23]: https://github.com/jerus-org/pcu/compare/v0.6.22...v0.6.23
[0.6.22]: https://github.com/jerus-org/pcu/compare/v0.6.21...v0.6.22
[0.6.21]: https://github.com/jerus-org/pcu/compare/v0.6.20...v0.6.21
[0.6.20]: https://github.com/jerus-org/pcu/compare/v0.6.19...v0.6.20
[0.6.19]: https://github.com/jerus-org/pcu/compare/v0.6.18...v0.6.19
[0.6.18]: https://github.com/jerus-org/pcu/compare/v0.6.17...v0.6.18
[0.6.17]: https://github.com/jerus-org/pcu/compare/v0.6.16...v0.6.17
[0.6.16]: https://github.com/jerus-org/pcu/compare/v0.6.15...v0.6.16
[0.6.15]: https://github.com/jerus-org/pcu/compare/v0.6.14...v0.6.15
[0.6.14]: https://github.com/jerus-org/pcu/compare/v0.6.13...v0.6.14
[0.6.13]: https://github.com/jerus-org/pcu/compare/v0.6.12...v0.6.13
[0.6.12]: https://github.com/jerus-org/pcu/compare/v0.6.11...v0.6.12
[0.6.11]: https://github.com/jerus-org/pcu/compare/v0.6.10...v0.6.11
[0.6.10]: https://github.com/jerus-org/pcu/compare/v0.6.9...v0.6.10
[0.6.9]: https://github.com/jerus-org/pcu/compare/v0.6.8...v0.6.9
[0.6.8]: https://github.com/jerus-org/pcu/compare/v0.6.7...v0.6.8
[0.6.7]: https://github.com/jerus-org/pcu/compare/v0.6.6...v0.6.7
[0.6.6]: https://github.com/jerus-org/pcu/compare/v0.6.5...v0.6.6
[0.6.5]: https://github.com/jerus-org/pcu/compare/v0.6.4...v0.6.5
[0.6.4]: https://github.com/jerus-org/pcu/compare/v0.6.3...v0.6.4
[0.6.3]: https://github.com/jerus-org/pcu/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/jerus-org/pcu/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/jerus-org/pcu/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/jerus-org/pcu/releases/tag/v0.6.0

