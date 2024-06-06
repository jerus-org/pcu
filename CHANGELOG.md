# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Return early if the changelog has been updated already(pr [#93](https://github.com/jerus-org/pcu/pull/93))
- logger and logging (pr [#102](https://github.com/jerus-org/pcu/pull/102))
- sign the commit using gpg(pr [#107](https://github.com/jerus-org/pcu/pull/107))

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

### Fixed

- trim exclamation point from end of subkey(pr [#105](https://github.com/jerus-org/pcu/pull/105))

### Security

- Security: adopt new ci bot signature(pr [#95](https://github.com/jerus-org/pcu/pull/95))
