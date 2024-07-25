# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.24] - 2024-07-25

### Changed

- ci-adopt revised toolkit(pr [#230])

## [0.1.23] - 2024-07-25

### Changed

- refactor-extract repeated code into commands set_senmver, make_cargo_release, make_github_release(pr [#229])

## [0.1.22] - 2024-07-25

### Fixed

- remove cat release_notes.md command(pr [#228])

## [0.1.21] - 2024-07-25

### Changed

- ci-release production conditional on SEMVER value existing(pr [#227])

## [0.1.20] - 2024-07-25

### Added

- add support for GITHUB_TOKEN environment variable(pr [#226])

## [0.1.19] - 2024-07-25

### Added

- add release notes trait to generate release notes from changelog(pr [#225])

## [0.1.18] - 2024-07-24

### Fixed

- format version string with 'v' prefix in generate_release_notes method(pr [#224])

## [0.1.17] - 2024-07-24

### Fixed

- use local octocrab instance add bot-check context to make_release workflow(pr [#223])

## [0.1.16] - 2024-07-24

### Changed

- chore-generate release notes as changelog change may not have run.(pr [#222])

## [0.1.15] - 2024-07-24

### Fixed

- add owner and repo to the client structure so that is available to all commands(pr [#221])

## [0.1.14] - 2024-07-24

### Changed

- chore-add logging for octocrab creation(pr [#220])

## [0.1.13] - 2024-07-24

### Changed

- chore-increase logging and replace octocrab initialisation(pr [#219])

## [0.1.12] - 2024-07-24

### Added

- add condition to update changelog in run_release function and refactor cli to module(pr [#215])
- add get_commitish_for_tag and refactor make_release function(pr [#217])
- add change release for pcu release(pr [#218])

### Security

- Dependencies: update rust crate tokio to 1.39.0(pr [#216])

## [0.1.11] - 2024-07-22

### Added

- add release function to move changelog unreleased section to new version(pr [#201])
- write release notes(pr [#202])
- add branch check to prevent actions on main or master branches(pr [#210])
- add make_release method to create a new release on GitHub(pr [#213])
- add make_release job to CircleCI configuration to use pcu(pr [#214])

### Changed

- ci-upgrade jerus-org/circleci-toolkit orb from 0.20.0 to 0.23.0(pr [#200])
- refactor-into PullRequest and Release commands and structure app accordingly(pr [#206])
- ci-change order of pcu command parameters to correctly apply --early-exit flag(pr [#207])
- refactor-extract pull request handling from client to separate struct(pr [#208])
- ci-add branch specification to cargo install command in CircleCI config(pr [#209])
- chore:tidy up logging(pr [#211])
- chore-move branch check for main earlier in logic(pr [#212])

### Security

- Dependencies: update rust crate clap to 4.5.9(pr [#203])
- Dependencies: update rust crate thiserror to 1.0.63(pr [#204])
- Dependencies: update rust crate tokio to 1.38.1(pr [#205])

## [0.1.10] - 2024-07-13

### Added

- adopt update changelog from toolkit(pr [#191])

### Changed

- ci-adopt updated toolkit and set install_from_github flag(pr [#182])
- chore-update renovate configuration with new rules and settings(pr [#184])
- ci-update toolkit version and adopt choose_pipeline for bot commit check(pr [#187])
- ci-adopt end_success from toolkit(pr [#188])
- chore-restore crates.io version of keep-a-changelog and update toolkit version(pr [#195])
- chore-trace config settings(pr [#198])
- chore-add trace log for pcu_pull_request variable(pr [#199])

### Fixed

- parameterise pipeline flags(pr [#183])
- move settings initialization after logging initialization(pr [#197])

### Security

- Dependencies: update rust crate log to 0.4.22(pr [#186])
- Dependencies: update rust crate clap to 4.5.8(pr [#185])
- Dependencies: update rust crate tokio to 1.38.0(pr [#190])
- Dependencies: update rust crate regex to 1.10.5(pr [#189])
- Dependencies: update rust crate url to 2.5.2(pr [#192])
- Dependencies: update rust crate uuid to 1.9.1(pr [#193])
- Dependencies: update rust crate uuid to 1.10.0(pr [#194])
- Dependencies: update rust crate keep-a-changelog to 0.1.4(pr [#196])

## [0.1.9] - 2024-07-06

### Added

- add categories field with development-tools::build-utils and command-line-utilities values(pr [#165](https://github.com/jerus-org/pcu/pull/165))
- add reference link for PR in title(pr [#166](https://github.com/jerus-org/pcu/pull/166))
- add support for generating repository URL from PR URL(pr [#167])
- add support for parsing changelog with repository URL in ChangelogParseOptions(pr [#168])
- print message when changelog updated(pr [#176])
- add early_exit flag for signaling an early exit(pr [#177])

### Changed

- ci-restore build of pcu package from main branch(pr [#171])
- chore-update pcu installation command description(pr [#178])
- ci-add --early-exit flag to pcu command in Update changelog job(pr [#179])
- ci-rename update changelog step and add early exit flag to pcu command(pr [#180])
- ci-adopt toolkit and configure to use setup config with validation and success(pr [#181])

### Fixed

- fix method signature requirements(pr [#170](https://github.com/jerus-org/pcu/pull/170))
- pr title elided (pr [#173])

### Security

- Dependencies: update rust crate log to v0.4.22(pr [#174])
- Dependencies: update rust crate clap to v4.5.8(pr [#175])

## [0.1.8] - 2024-06-19

### Changed

- refactor-improve logging for Octocrab instance and pull request handler acquisition(pr [#164](https://github.com/jerus-org/pcu/pull/164))

## [0.1.7] - 2024-06-19

### Changed

- refactor-improve error handling and add debug logs for octocrab client(pr [#163](https://github.com/jerus-org/pcu/pull/163))

## [0.1.6] - 2024-06-19

### Added

- add octocrab authentication by personal access token(pr [#162](https://github.com/jerus-org/pcu/pull/162))

### Security

- Dependencies: update rust crate url to v2.5.2(pr [#161](https://github.com/jerus-org/pcu/pull/161))

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

[#167]: https://github.com/jerus-org/pcu/pull/167
[#168]: https://github.com/jerus-org/pcu/pull/168
[#171]: https://github.com/jerus-org/pcu/pull/171
[#173]: https://github.com/jerus-org/pcu/pull/173
[#174]: https://github.com/jerus-org/pcu/pull/174
[#175]: https://github.com/jerus-org/pcu/pull/175
[#176]: https://github.com/jerus-org/pcu/pull/176
[#177]: https://github.com/jerus-org/pcu/pull/177
[#178]: https://github.com/jerus-org/pcu/pull/178
[#179]: https://github.com/jerus-org/pcu/pull/179
[#180]: https://github.com/jerus-org/pcu/pull/180
[#181]: https://github.com/jerus-org/pcu/pull/181
[#182]: https://github.com/jerus-org/pcu/pull/182
[#183]: https://github.com/jerus-org/pcu/pull/183
[#184]: https://github.com/jerus-org/pcu/pull/184
[#187]: https://github.com/jerus-org/pcu/pull/187
[#186]: https://github.com/jerus-org/pcu/pull/186
[#188]: https://github.com/jerus-org/pcu/pull/188
[#185]: https://github.com/jerus-org/pcu/pull/185
[#190]: https://github.com/jerus-org/pcu/pull/190
[#189]: https://github.com/jerus-org/pcu/pull/189
[#191]: https://github.com/jerus-org/pcu/pull/191
[#192]: https://github.com/jerus-org/pcu/pull/192
[#193]: https://github.com/jerus-org/pcu/pull/193
[#194]: https://github.com/jerus-org/pcu/pull/194
[#195]: https://github.com/jerus-org/pcu/pull/195
[#196]: https://github.com/jerus-org/pcu/pull/196
[#197]: https://github.com/jerus-org/pcu/pull/197
[#198]: https://github.com/jerus-org/pcu/pull/198
[#199]: https://github.com/jerus-org/pcu/pull/199
[#200]: https://github.com/jerus-org/pcu/pull/200
[#201]: https://github.com/jerus-org/pcu/pull/201
[#202]: https://github.com/jerus-org/pcu/pull/202
[#203]: https://github.com/jerus-org/pcu/pull/203
[#204]: https://github.com/jerus-org/pcu/pull/204
[#206]: https://github.com/jerus-org/pcu/pull/206
[#207]: https://github.com/jerus-org/pcu/pull/207
[#205]: https://github.com/jerus-org/pcu/pull/205
[#208]: https://github.com/jerus-org/pcu/pull/208
[#209]: https://github.com/jerus-org/pcu/pull/209
[#210]: https://github.com/jerus-org/pcu/pull/210
[#211]: https://github.com/jerus-org/pcu/pull/211
[#212]: https://github.com/jerus-org/pcu/pull/212
[#213]: https://github.com/jerus-org/pcu/pull/213
[#214]: https://github.com/jerus-org/pcu/pull/214
[#215]: https://github.com/jerus-org/pcu/pull/215
[#216]: https://github.com/jerus-org/pcu/pull/216
[#217]: https://github.com/jerus-org/pcu/pull/217
[#218]: https://github.com/jerus-org/pcu/pull/218
[#219]: https://github.com/jerus-org/pcu/pull/219
[#220]: https://github.com/jerus-org/pcu/pull/220
[#221]: https://github.com/jerus-org/pcu/pull/221
[#222]: https://github.com/jerus-org/pcu/pull/222
[#223]: https://github.com/jerus-org/pcu/pull/223
[#224]: https://github.com/jerus-org/pcu/pull/224
[#225]: https://github.com/jerus-org/pcu/pull/225
[#226]: https://github.com/jerus-org/pcu/pull/226
[#227]: https://github.com/jerus-org/pcu/pull/227
[#228]: https://github.com/jerus-org/pcu/pull/228
[#229]: https://github.com/jerus-org/pcu/pull/229
[#230]: https://github.com/jerus-org/pcu/pull/230
[0.1.24]: https://github.com/jerus-org/pcu/compare/0.1.23...v0.1.24
[0.1.23]: https://github.com/jerus-org/pcu/compare/0.1.22...0.1.23
[0.1.22]: https://github.com/jerus-org/pcu/compare/0.1.21...0.1.22
[0.1.21]: https://github.com/jerus-org/pcu/compare/0.1.20...0.1.21
[0.1.20]: https://github.com/jerus-org/pcu/compare/0.1.19...0.1.20
[0.1.19]: https://github.com/jerus-org/pcu/compare/0.1.18...0.1.19
[0.1.18]: https://github.com/jerus-org/pcu/compare/0.1.17...0.1.18
[0.1.17]: https://github.com/jerus-org/pcu/compare/0.1.16...0.1.17
[0.1.16]: https://github.com/jerus-org/pcu/compare/0.1.15...0.1.16
[0.1.15]: https://github.com/jerus-org/pcu/compare/0.1.14...0.1.15
[0.1.14]: https://github.com/jerus-org/pcu/compare/0.1.13...0.1.14
[0.1.13]: https://github.com/jerus-org/pcu/compare/0.1.12...0.1.13
[0.1.12]: https://github.com/jerus-org/pcu/compare/0.1.11...0.1.12
[0.1.11]: https://github.com/jerus-org/pcu/compare/0.1.10...0.1.11
[0.1.10]: https://github.com/jerus-org/pcu/compare/0.1.9...0.1.10
[0.1.9]: https://github.com/jerus-org/pcu/compare/0.1.8...0.1.9
[0.1.8]: https://github.com/jerus-org/pcu/compare/0.1.7...0.1.8
[0.1.7]: https://github.com/jerus-org/pcu/compare/0.1.6...0.1.7
[0.1.6]: https://github.com/jerus-org/pcu/compare/0.1.5...0.1.6
[0.1.5]: https://github.com/jerus-org/pcu/compare/0.1.4...0.1.5
[0.1.4]: https://github.com/jerus-org/pcu/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/jerus-org/pcu/compare/0.1.2...0.1.3
[0.1.2]: https://github.com/jerus-org/pcu/compare/0.1.1...0.1.2
[0.1.1]: https://github.com/jerus-org/pcu/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/jerus-org/pcu/releases/tag/0.1.0
