# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.5] - 2024-06-18

### Changed

- chore-additional debug info for octocrab instance setup(pr [#160](https://github.com/jerus-org/pcu/pull/160))

## [0.1.4] - 2024-06-18

### Changed

- refactor-remove unwrap from string conversion of OsStr value(pr [#159](https://github.com/jerus-org/pcu/pull/159))

## [0.1.3] - 2024-06-15

### Changed

- chore-test for value of CARGO_HOME(pr [#152](https://github.com/jerus-org/pcu/pull/152))
- chore-user cargo home bin version of pcu(pr [#153](https://github.com/jerus-org/pcu/pull/153))
- chore-update eyre to color-eyre(pr [#158](https://github.com/jerus-org/pcu/pull/158))

### Fixed

- ensure that final error state fails pcu application(pr [#157](https://github.com/jerus-org/pcu/pull/157))

### Security

- Dependencies: update rust crate git2 to 0.19.0(pr [#155](https://github.com/jerus-org/pcu/pull/155))
- Dependencies: update rust crate rstest to 0.21.0(pr [#154](https://github.com/jerus-org/pcu/pull/154))

## [0.1.2] - 2024-06-12

### Changed

- chore-add exactly 1 to replace(pr [#150](https://github.com/jerus-org/pcu/pull/150))
- chore-add search that yields only one response(pr [#151](https://github.com/jerus-org/pcu/pull/151))

## [0.1.1] - 2024-06-12

### Added

- create unreleased section if it does not exist(pr [#146](https://github.com/jerus-org/pcu/pull/146))

### Changed

- ci: add step to remove original SSH key from agent(pr [#142](https://github.com/jerus-org/pcu/pull/142))
- chore-add pre-release configuration for versioning and changelog updates(pr [#143](https://github.com/jerus-org/pcu/pull/143))
- docs-update changelog with new unreleased changes(pr [#145](https://github.com/jerus-org/pcu/pull/145))
- chore-remove  cargo release  comment replacements(pr [#147](https://github.com/jerus-org/pcu/pull/147))
- chore-update unreleased with version and date(pr [#148](https://github.com/jerus-org/pcu/pull/148))
- chore-remove failing replacements(pr [#149](https://github.com/jerus-org/pcu/pull/149))

## [0.1.0] - 2024-06-10

### Added

- Return early if the changelog has been updated already(pr [#93](https://github.com/jerus-org/pcu/pull/93))
- logger and logging (pr [#102](https://github.com/jerus-org/pcu/pull/102))
- sign the commit using gpg(pr [#107](https://github.com/jerus-org/pcu/pull/107))
- add CLI flag to set verbosity of logs(pr [#117](https://github.com/jerus-org/pcu/pull/117))
- allow log environment variables to override command line(pr [#118](https://github.com/jerus-org/pcu/pull/118))
- add Sign enum and sign field to Cli struct(pr [#121](https://github.com/jerus-org/pcu/pull/121))
- add match for sign types in run_update function(pr [#122](https://github.com/jerus-org/pcu/pull/122))
- add config crate to Cargo.toml and implement get_settings function in main.rs(pr [#127](https://github.com/jerus-org/pcu/pull/127))
- add error handling for settings(pr [#129](https://github.com/jerus-org/pcu/pull/129))
- set defaults for settings(pr [#131](https://github.com/jerus-org/pcu/pull/131))
- add support for using settings to configure client(pr [#134](https://github.com/jerus-org/pcu/pull/134))

### Changed

- ci-tidy up and clarify removal of original ssh key(pr [#92](https://github.com/jerus-org/pcu/pull/92))
- chore-add debug print statement to repo_status method(pr [#94](https://github.com/jerus-org/pcu/pull/94))
- chore-remove redundant code in main(pr [#96](https://github.com/jerus-org/pcu/pull/96))
- ci-reorder ssh and gpg and git steps(pr [#97](https://github.com/jerus-org/pcu/pull/97))
- chore-update branch message to remove personal pronoun(pr [#98](https://github.com/jerus-org/pcu/pull/98))
- refactor-remove commented out code(pr [#99](https://github.com/jerus-org/pcu/pull/99))
- chore-return to using none in commit_signed(pr [#100](https://github.com/jerus-org/pcu/pull/100))
- refactor-align var and restore simple commit without signing(pr [#101](https://github.com/jerus-org/pcu/pull/101))
- refactor-remove redundant imports and functions(pr [#103](https://github.com/jerus-org/pcu/pull/103))
- chore-replace printlns with log macros(pr [#104](https://github.com/jerus-org/pcu/pull/104))
- refactor-main update to use logs instead of println(pr [#106](https://github.com/jerus-org/pcu/pull/106))
- chore-move all but current println to logging(pr [#108](https://github.com/jerus-org/pcu/pull/108))
- chore-decorate signing with log messages(pr [#109](https://github.com/jerus-org/pcu/pull/109))
- chore-add logging for input before writing to stdin(pr [#110](https://github.com/jerus-org/pcu/pull/110))
- docs-tidy up changelog(pr [#111](https://github.com/jerus-org/pcu/pull/111))
- chore-catch up the changelog(pr [#112](https://github.com/jerus-org/pcu/pull/112))
- chore-remove blank lines(pr [#113](https://github.com/jerus-org/pcu/pull/113))
- chore-remove blanks(pr [#114](https://github.com/jerus-org/pcu/pull/114))
- chore-display messages using logging(pr [#115](https://github.com/jerus-org/pcu/pull/115))
- refactor-replace println with logging(pr [#116](https://github.com/jerus-org/pcu/pull/116))
- Update issue templates(pr [#119](https://github.com/jerus-org/pcu/pull/119))
- chore-add pull request template to .github directory(pr [#120](https://github.com/jerus-org/pcu/pull/120))
- chore-add log level check before updating changelog(pr [#125](https://github.com/jerus-org/pcu/pull/125))
- chore-add default values required by client module(pr [#133](https://github.com/jerus-org/pcu/pull/133))
- refactor-migrate const to  settings(pr [#135](https://github.com/jerus-org/pcu/pull/135))
- ci-customise release to make the first release v0.1.0(pr [#137](https://github.com/jerus-org/pcu/pull/137))
- chore-manual update of changelog waiting on fix to message(pr [#139](https://github.com/jerus-org/pcu/pull/139))
- chore-prepare for release by committing lock file (pr [#140](https://github.com/jerus-org/pcu/pull/140))
- docs-update readme for first release(pr [#141](https://github.com/jerus-org/pcu/pull/141))

### Fixed

- trim exclamation point from end of subkey(pr [#105](https://github.com/jerus-org/pcu/pull/105))
- change log level from Debug to Trace in run_update function(pr [#123](https://github.com/jerus-org/pcu/pull/123))
- setup of logging for environment or cli(pr [#124](https://github.com/jerus-org/pcu/pull/124))
- correct typo in setting key for commit_message(pr [#136](https://github.com/jerus-org/pcu/pull/136))
- fix type annotation for msg variable to String(pr [#138](https://github.com/jerus-org/pcu/pull/138))

### Security

- Security: adopt new ci bot signature(pr [#95](https://github.com/jerus-org/pcu/pull/95))

[0.1.5]: https://github.com/jerus-org/pcu/compare/0.1.4...v0.1.5
[0.1.4]: https://github.com/jerus-org/pcu/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/jerus-org/pcu/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/jerus-org/pcu/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/jerus-org/pcu/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/jerus-org/pcu/releases/tag/0.1.0
