# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

### Fixed

- trim exclamation point from end of subkey(pr [#105](https://github.com/jerus-org/pcu/pull/105))
- change log level from Debug to Trace in run_update function(pr [#123](https://github.com/jerus-org/pcu/pull/123))
- setup of logging for environment or cli(pr [#124](https://github.com/jerus-org/pcu/pull/124))
- correct typo in setting key for commit_message [#136](https://github.com/jerus-org/pcu/pull/136)

### Security

- Security: adopt new ci bot signature(pr [#95](https://github.com/jerus-org/pcu/pull/95))
