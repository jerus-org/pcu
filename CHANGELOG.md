# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- ✨ add initial Cargo.toml for gen-bsky crate(pr [#634])
- ✨ add front matter parsing functionality(pr [#638])
- ✨ add error variants and write bluesky record(pr [#639])
- ✨ add draft management for blog posts(pr [#640])
- ✨ add getter method for bluesky field(pr [#643])
- ✨ add draft allowance option to blog post processing(pr [#648])
- ✨ add link module for blog post draft(pr [#652])

### Changed

- 🔧 BREAKING: chore(workspace)-restructure Cargo.toml for workspace management(pr [#633])
- 🔧 chore(gen-bsky)-update cargo package metadata(pr [#635])
- 💄 style(gen-bsky)-correct license format in Cargo.toml(pr [#636])
- 👷 ci(circleci)-update workflow branch filter(pr [#637])
- ♻️ refactor(front_matter)-update path handling and remove basename usage(pr [#641])
- ♻️ refactor(bluesky)-update field access in Bluesky struct(pr [#642])
- ♻️ refactor(taxonomies)-improve tag handling in Taxonomies struct(pr [#644])
- ♻️ refactor(draft)-improve path handling in DraftBuilder(pr [#645])
- 👷 ci(circleci)-enhance check_last_commit workflow(pr [#646])
- 👷 ci(config)-update CircleCI workflow filters(pr [#647])
- 📝 docs(README)-update project title casing(pr [#649])
- Simplify-and-improve-draft-structure(pr [#650])
- Rename-and-refactor-path-handling(pr [#653])
- ✅ test(front_matter)-add test for incorrect bluesky field(pr [#654])
- Add-code-coverage-generation(pr [#655])
- 🔧 BREAKING: chore(config)-simplify test configuration file(pr [#656])

### Fixed

- 🐛 cli: correct path handling in cmd_draft(pr [#651])

## [0.4.56] - 2025-07-30

### Changed

- 💄 style(client)-adjust log level for settings output and disable pcu-app(pr [#631])

## [0.4.55] - 2025-07-30

### Changed

- 👷 ci(circleci)-update context in release workflow(pr [#630])

## [0.4.54] - 2025-07-30

### Changed

- 👷 ci(circleci)-add conditional step for PCU update(pr [#629])

## [0.4.53] - 2025-07-30

### Changed

- 💄 style(client)-change log level to info(pr [#628])

## [0.4.52] - 2025-07-30

### Changed

- 🔧 chore(package)-bump version to 0.4.51(pr [#627])

## [0.4.51] - 2025-07-29

### Added

- ✨ add check for nothing to push in push command(pr [#623])
- ✨ enhance logging with branch status and commit info(pr [#625])

### Changed

- ♻️ refactor(cmd_draft)-rename methods and variables for clarity(pr [#621])
- 💄 style(cmd_draft)-update log level for front matter(pr [#622])
- ♻️ refactor(cli)-modify changelog commit logic(pr [#626])

### Fixed

- 🐛 git_ops: handle empty file lists during staging and committing(pr [#624])

## [0.4.50] - 2025-07-28

### Added

- ✨ enhance post text with extra field(pr [#586])
- ✨ add hashtag formatting to taxonomies(pr [#587])
- ✨ add build_post_text method for post generation(pr [#589])
- ✨ add automatic push after commit in cmd_post(pr [#590])
- ✨ add redirect functionality to draft command(pr [#591])
- ✨ add logging for post details(pr [#592])
- ✨ enhance bluesky integration(pr [#593])
- ✨ add date handling and comparison(pr [#594])
- ✨ add short link store support(pr [#597])
- ✨ add default date filtering for draft command(pr [#598])
- ✨ enhance postname generation logic(pr [#614])
- ✨ add release flag to cmd_post(pr [#616])

### Changed

- 🔧 chore(config)-add host rules for CircleCI in renovate(pr [#580])
- 🔧 chore(config)-update renovate configuration to inherit settings(pr [#581])
- 🔧 chore(renovate)-update configuration to extend from remote(pr [#582])
- 🔧 chore(config)-update renovate configuration path(pr [#584])
- 🔧 chore(renovate)-update renovate configuration(pr [#585])
- ♻️ refactor(front_matter)-modularize and streamline code structure(pr [#596])
- 🔧 chore(cli)-add debug log for site configuration(pr [#599])
- 📝 docs(front_matter)-add debug logs for redirect process(pr [#601])
- ♻️ refactor(front_matter)-consolidate link generation methods(pr [#602])
- 📦 build(dependencies)-update link-bridge version(pr [#615])
- ♻️ refactor(cmd_draft)-enhance file change handling logic(pr [#620])

### Fixed

- 🐛 draft: remove hashtag from post text format(pr [#588])
- 🐛 front_matter: correct post link formatting(pr [#595])
- 🐛 cli: enhance logging for draft command(pr [#600])
- deps: update rust crate clap to 4.5.41(pr [#603])
- deps: update rust crate config to 0.15.13(pr [#604])
- deps: update rust crate named-colour to 0.3.22(pr [#605])
- deps: update rust crate serde_json to 1.0.141(pr [#606])
- deps: update dependency toolkit to v2.12.1(pr [#607])
- deps: update rust crate tempfile to 3.20.0(pr [#608])
- deps: update rust crate tokio to 1.46.1(pr [#609])
- deps: update rust crate toml to 0.9.2(pr [#610])
- deps: update rust crate uuid to 1.17.0(pr [#611])
- 🐛 front_matter: correct short link formatting(pr [#613])
- 🐛 front_matter: correct error handling for post length(pr [#617])
- 🐛 front_matter: correct method call for tag retrieval(pr [#618])
- 🐛 cli: correct basehead format in draft command(pr [#619])

## [0.4.49] - 2025-07-10

### Changed

- ♻️ refactor(cli)-improve error handling for pull request(pr [#579])

### Fixed

- 🐛 cli: change default for allow_no_pull_request to true(pr [#578])

## [0.4.48] - 2025-07-08

### Changed

- Simplify release workflow configuration(pr [#571])
- ♻️ refactor(cli)-rename option for pull request handling(pr [#572])
- 🔧 chore(ci)-update release workflow(pr [#573])
- 👷 ci(circleci)-fix ssh directory listing command(pr [#574])
- 🔧 chore(ci)-update release.yml configuration(pr [#575])
- 📝 docs(CHANGELOG)-update changelog for unreleased changes(pr [#576])

### Fixed

- 🐛 cli: correct error handling for pull request(pr [#577])

## [0.4.45] - 2025-06-28

### Added

- ✨ add hide_no_pull_request option to Pr(pr [#570])

### Changed

- Upgrade-toolkit-orb-to-version-2.11.0-for-latest-features-and-fixes(pr [#568])
- ♻️ refactor(logging)-use format strings for logging(pr [#569])

## [0.4.45] - 2025-06-28

### Fixed

- deps: update rust crate bsky-sdk to 0.1.20(pr [#559])
- deps: update rust crate clap to 4.5.40(pr [#560])
- deps: update rust crate clap-verbosity-flag to 3.0.3(pr [#561])
- deps: update rust crate color-eyre to 0.6.5(pr [#562])
- deps: update rust crate git2 to 0.20.2(pr [#563])
- deps: update rust crate gql_client to 1.0.8(pr [#564])
- deps: update rust crate owo-colors to 4.2.2(pr [#565])
- deps: update rust crate toml to 0.8.23(pr [#566])
- deps: update rust crate url to 2.5.4(pr [#567])

## [0.4.44] - 2025-05-28

### Changed

- 🔧 chore(config)-update renovate schedule(pr [#556])
- 🔧 chore(config)-update renovate schedule for flexibility(pr [#558])

## [0.4.43] - 2025-05-06

### Changed

- 👷 ci(circleci)-simplify release workflow configuration(pr [#554])
- 👷 ci(circleci)-update circleci-toolkit orb version(pr [#555])

### Fixed

- deps: update rust crate bsky-sdk to 0.1.19(pr [#550])
- deps: update rust crate chrono to 0.4.41(pr [#551])
- deps: update rust crate toml to 0.8.22(pr [#552])

## [0.4.42] - 2025-05-01

### Added

- ✨ BREAKING: add mode-based release management(pr [#546])

### Changed

- ♻️ refactor(cli)-simplify release mode handling(pr [#547])
- 👷 ci(circleci)-add remove_ssh_key parameter to release workflow(pr [#548])
- 👷 ci(circleci)-update ssh key removal setting(pr [#549])

### Fixed

- 🐛 release: add semver requirement check(pr [#545])

## [0.4.41] - 2025-04-26

### Added

- ✨ add get_tag functionality(pr [#538])
- ✨ enhance get_tag with commit details(pr [#539])
- ✨ enhance logging configuration(pr [#541])
- ✨ update committed_date to DateTime<Utc>(pr [#544])

### Changed

- ♻️ refactor(logging)-replace tracing with log(pr [#542])
- 💄 style(graphql)-fix indentation in get_tag query(pr [#543])

### Fixed

- deps: update rust crate clap to 4.5.37(pr [#540])

## [0.4.40] - 2025-04-22

### Added

- ✨ add fail_on_missing option to CmdPost(pr [#530])
- ✨ add bot user name for push_commit(pr [#531])
- ✨ add trace logging for credential usage(pr [#532])

### Changed

- 🔧 chore(ci)-remove unused version checks in CircleCI config(pr [#525])
- ♻️ refactor(cli)-replace CI check with testing check(pr [#529])
- 📦 build(dependencies)-update Cargo dependencies(pr [#534])
- 👷 ci(circleci)-separate release workflow into its own config(pr [#535])
- Revert 👷 ci(circleci)- separate release workflow into its own config(pr [#537])

### Fixed

- deps: update rust crate clap to 4.5.36(pr [#526])
- deps: update rust crate tokio to 1.44.2(pr [#527])
- deps: update dependency toolkit to v2.8.1(pr [#528])
- 🐛 git_ops: add logging for ssh key credential status(pr [#533])

## [0.4.39] - 2025-04-12

### Changed

- 👷 ci(circleci)-add tools job to CI configuration(pr [#523])
- 👷 ci(circleci)-update workflow conditions(pr [#524])

## [0.4.38] - 2025-04-05

### Fixed

- deps: update rust crate log to 0.4.27(pr [#517])
- deps: update rust crate bsky-sdk to 0.1.18(pr [#518])
- deps: update rust crate clap to 4.5.35(pr [#519])
- deps: update rust crate env_logger to 0.11.8(pr [#520])

## [0.4.37] - 2025-03-29

### Fixed

- deps: update rust crate clap to 4.5.34(pr [#516])

## [0.4.36] - 2025-03-22

### Added

- ✨ add storage directory option for Bluesky posts(pr [#512])

### Changed

- 👷 ci(circleci)-update toolkit orb and streamline config(pr [#510])
- ♻️ refactor(cmd_draft)-clean up get_files_from_path function(pr [#511])

### Fixed

- deps: update rust crate git2 to 0.20.1(pr [#513])
- deps: update rust crate tempfile to 3.19.1(pr [#514])
- deps: update rust crate named-colour to 0.3.19(pr [#515])

## [0.4.35] - 2025-03-15

### Fixed

- deps: update rust crate env_logger to 0.11.7(pr [#505])
- deps: update rust crate clap to 4.5.32(pr [#503])
- deps: update rust crate config to 0.15.11(pr [#504])
- deps: update rust crate serde to 1.0.219(pr [#506])
- deps: update rust crate cargo_toml to 0.22.1(pr [#507])
- deps: update rust crate tokio to 1.44.1(pr [#508])
- deps: update rust crate uuid to 1.16.0(pr [#509])

## [0.4.34] - 2025-03-08

### Added

- ✨ add Bsky command for posting to Bluesky(pr [#479])

### Changed

- 🔧 chore(structure)-reorganize directory structure(pr [#485])
- ♻️ refactor(cli)-modularize pull request handling(pr [#486])
- ♻️ refactor(cli)-modularize commit command handling(pr [#487])
- ♻️ refactor(cli)-modularize push command handling(pr [#488])
- ♻️ refactor(cli)-modularize label command handling(pr [#489])
- ♻️ refactor(cli)-modularize release logic into separate module(pr [#490])
- ♻️ refactor(cli)-centralize push command implementation(pr [#491])
- ♻️ refactor(cli)-move commit command logic to cli/commit.rs(pr [#492])
- ♻️ refactor(cli)-enhance label command structure(pr [#493])
- ♻️ refactor(release)-restructure release command handling(pr [#494])
- ♻️ refactor(cli)-migrate `Pr` struct and method to `pull_request.rs`(pr [#495])
- ♻️ refactor(cli)-refactor client acquisition method(pr [#496])
- ♻️ refactor(cli)-remove duplicate push_committed function(pr [#497])
- ♻️ refactor(cli)-remove duplicate commit function(pr [#498])

### Fixed

- deps: update rust crate config to 0.15.9(pr [#499])
- deps: update rust crate named-colour to 0.3.18(pr [#500])
- deps: update rust crate thiserror to 2.0.12(pr [#501])
- deps: update rust crate rstest to 0.25.0(pr [#502])

## [0.4.33] - 2025-03-01

### Changed

- 💄 style(issue template)-improve formatting consistency(pr [#478])

### Fixed

- deps: update dependency toolkit to v2.0.13(pr [#480])
- deps: update rust crate chrono to 0.4.40(pr [#481])
- deps: update rust crate clap to 4.5.31(pr [#482])
- deps: update rust crate owo-colors to 4.2.0(pr [#483])
- deps: update rust crate uuid to 1.15.1(pr [#484])

## [0.4.32] - 2025-02-25

### Added

- ✨ add commit emoji support(pr [#477])

## [0.4.31] - 2025-02-24

### Changed

- ✨ feat(cli): add allow_push_fail option(pr [#476])

## [0.4.30] - 2025-02-22

### Fixed

- deps: update rust crate clap to 4.5.30(pr [#470])
- deps: update rust crate named-colour to 0.3.16(pr [#471])
- deps: update rust crate serde to 1.0.218(pr [#472])
- deps: update rust crate octocrate to 2.2.0(pr [#473])
- deps: update rust crate log to 0.4.26(pr [#474])
- deps: update rust crate uuid to 1.14.0(pr [#475])

## [0.4.29] - 2025-02-15

### Fixed

- deps: update rust crate clap to 4.5.29(pr [#468])
- deps: update rust crate config to 0.15.8(pr [#469])

## [0.4.28] - 2025-02-08

### Fixed

- deps: update rust crate clap to 4.5.28(pr [#466])
- deps: update rust crate uuid to 1.13.1(pr [#467])

### Security

- Dependencies: bump openssl from 0.10.68 to 0.10.70 in the cargo group across 1 directory(pr [#465])

## [0.4.27] - 2025-02-01

### Changed

- 👷 ci(circleci): enhance CircleCI config with new job dependencies and trigger(pr [#464])

### Fixed

- deps: update rust crate config to 0.15.7(pr [#460])
- deps: update rust crate uuid to 1.12.1(pr [#461])
- deps: update rust crate named-colour to 0.3.14(pr [#462])
- deps: update dependency toolkit to v2(pr [#463])

## [0.4.26] - 2025-01-25

### Fixed

- deps: update rust crate clap to 4.5.27(pr [#455])
- deps: update rust crate named-colour to 0.3.13(pr [#456])
- deps: update rust crate git2 to 0.20.0(pr [#457])
- deps: update rust crate tokio to 1.43.0(pr [#458])
- config: migrate renovate config(pr [#459])

## [0.4.25] - 2025-01-18

### Fixed

- deps: update rust crate config to 0.15.6(pr [#451])
- deps: update rust crate log to 0.4.25(pr [#452])
- deps: update rust crate named-colour to 0.3.12(pr [#453])
- deps: update rust crate thiserror to 2.0.11(pr [#454])

## [0.4.24] - 2025-01-11

### Fixed

- deps: update rust crate clap to 4.5.26(pr [#449])
- deps: update rust crate named-colour to 0.3.11(pr [#450])

## [0.4.23] - 2025-01-04

### Fixed

- deps: update rust crate serde to 1.0.217(pr [#445])
- deps: update rust crate tokio to 1.42.0(pr [#446])
- deps: update dependency toolkit to v1.23.0(pr [#447])
- deps: update rust crate rstest to 0.24.0(pr [#448])

## [0.4.22] - 2024-12-28

### Fixed

- deps: update rust crate config to 0.15.4(pr [#441])
- deps: update rust crate env_logger to 0.11.6(pr [#442])
- deps: update rust crate named-colour to 0.3.10(pr [#443])
- deps: update rust crate thiserror to 2.0.9(pr [#444])

## [0.4.21] - 2024-12-21

### Changed

- chore-update CircleCI toolkit orb to version 1.21.0(pr [#440])

### Fixed

- deps: update rust crate clap-verbosity-flag to 3.0.2(pr [#436])
- deps: update rust crate thiserror to 2.0.8(pr [#437])
- deps: update rust crate config to 0.15.3(pr [#439])
- deps: update dependency toolkit to v1.20.2(pr [#438])

## [0.4.20] - 2024-12-14

### Fixed

- deps: update rust crate chrono to 0.4.39(pr [#432])
- deps: update rust crate named-colour to 0.3.8(pr [#433])
- deps: update rust crate serde to 1.0.216(pr [#434])
- deps: update rust crate thiserror to 2.0.6(pr [#435])

## [0.4.19] - 2024-12-07

### Fixed

- deps: update rust crate clap to 4.5.23(pr [#428])
- deps: update rust crate thiserror to 2.0.4(pr [#429])
- deps: update rust crate tracing-subscriber to 0.3.19(pr [#430])
- deps: update rust crate cargo_toml to 0.21.0(pr [#431])

## [0.4.18] - 2024-11-30

### Fixed

- deps: update rust crate clap-verbosity-flag to 3.0.1(pr [#426])
- deps: update rust crate tracing to 0.1.41(pr [#427])

### Security

- Dependencies: bump rustls from 0.23.16 to 0.23.18 in the cargo group across 1 directory(pr [#425])

## [0.4.17] - 2024-11-23

### Fixed

- deps: update rust crate clap-verbosity-flag to 2.2.3(pr [#422])
- deps: update rust crate named-colour to 0.3.6(pr [#423])
- deps: update rust crate clap-verbosity-flag to v3(pr [#424])

## [0.4.16] - 2024-11-16

### Fixed

- deps: update rust crate thiserror to 2.0.2(pr [#418])
- deps: update rust crate serde to 1.0.215(pr [#420])
- deps: update rust crate thiserror to 2.0.3(pr [#419])
- deps: update rust crate clap to 4.5.21(pr [#421])

## [0.4.15] - 2024-11-09

### Fixed

- deps: update rust crate named-colour to 0.3.5(pr [#408])
- deps: update rust crate thiserror to 1.0.66(pr [#409])
- deps: update rust crate thiserror to 1.0.67(pr [#410])
- deps: update rust crate url to 2.5.3(pr [#411])
- deps: update rust crate thiserror to 1.0.68(pr [#412])
- deps: update rust crate thiserror to v2(pr [#413])
- deps: update rust crate tokio to 1.41.1(pr [#415])
- deps: update dependency toolkit to v1.18.0(pr [#414])
- deps: update dependency toolkit to v1.19.0(pr [#416])
- deps: update rust crate thiserror to 2.0.1(pr [#417])

## [0.4.14] - 2024-11-02

### Fixed

- deps: update rust crate serde to 1.0.214(pr [#406])
- deps: update dependency toolkit to v1.15.0(pr [#407])

## [0.4.13] - 2024-10-26

### Added

- add workspace flag to process packages in the workspace(pr [#395])
- implement pagination for listing tags in GitOps(pr [#401])
- add option to release specific workspace package(pr [#402])

### Changed

- refactor(cli)-make semver optional and handle missing value in release process(pr [#398])
- refactor(circleci)-simplify config by removing unused commands and jobs(pr [#404])

### Fixed

- deps: update rust crate serde to 1.0.211(pr [#393])
- deps: update rust crate tokio to 1.41.0(pr [#394])
- deps: update rust crate serde to 1.0.213(pr [#396])
- deps: update rust crate thiserror to 1.0.65(pr [#397])
- cli: correct tag existence check and release creation logic(pr [#399])
- deps: update rust crate config to 0.14.1(pr [#400])
- deps: update rust crate regex to 1.11.1(pr [#403])
- cli: correct typo in release command description(pr [#405])

## [0.4.12] - 2024-10-19

### Fixed

- deps: update rust crate uuid to 1.11.0(pr [#392])

## [0.4.11] - 2024-10-12

### Fixed

- deps: update rust crate named-colour to 0.3.4(pr [#390])
- deps: update rust crate clap to 4.5.20(pr [#391])

## [0.4.10] - 2024-10-05

### Fixed

- deps: update rust crate named-colour to 0.3.3(pr [#386])
- deps: update rust crate rstest to 0.23.0(pr [#387])
- deps: update rust crate regex to 1.11.0(pr [#388])
- deps: update rust crate clap to 4.5.19(pr [#389])

## [0.4.9] - 2024-09-28

### Added

- add host rules for CircleCI token in renovate.json(pr [#378])
- add sourceUrl for circleci-toolkit orb(pr [#379])
- add authType configuration for CircleCI host in renovate.json(pr [#382])

### Fixed

- enable CircleCI toolkit package in renovate configuration(pr [#377])
- update token reference in renovate.json for CircleCI(pr [#380])
- update sourceUrl for circleci-toolkit in renovate.json(pr [#381])
- remove hostRules configuration from renovate.json(pr [#384])
- deps: update dependency toolkit to v1.11.0(pr [#383])
- deps: update rust crate clap-verbosity-flag to 2.2.2(pr [#385])

## [0.4.8] - 2024-09-24

### Added

- add trace logs for push_commit function(pr [#373])
- add prefix option for version tags in CLI commands(pr [#375])

### Changed

- ci(circleci)-update toolkit orb to version 1.9.2 and add security context to workflows(pr [#374])
- refactor!(cli): rename PullRequest to Pr for brevity(pr [#376])

### Fixed

- deps: update rust crate thiserror to 1.0.64(pr [#372])
- deps: update rust crate clap to 4.5.18(pr [#371])

## [0.4.7] - 2024-09-21

### Fixed

- deps: update rust crate named-colour to 0.3.2(pr [#369])

## [0.4.6] - 2024-09-14

### Added

- add package grouping and regex manager for rust-toolchain(pr [#357])

### Changed

- chore-remove semanticCommitType from renovate.json(pr [#353])
- chore-remove semanticCommits setting from renovate.json(pr [#354])
- chore-remove versioning configuration from renovate.json(pr [#355])
- chore-remove rangeStrategy from renovate configuration(pr [#356])
- chore-update rangeStrategy to update-lockfile in renovate.json(pr [#358])
- ci-update CircleCI config to use toolkit 1.5.0 and add label_pr job(pr [#363])
- chore(circleci)-update toolkit orb to version 1.6.1(pr [#367])

### Fixed

- change rangeStrategy in renovate.json from update-lockfile to replace(pr [#359])
- update rangeStrategy in renovate.json from replace to bump(pr [#360])
- deps: update rust crate clap to 4.5.17(pr [#361])
- deps: update rust crate named-colour to 0.3.1(pr [#362])
- deps: update rust crate serde to 1.0.210(pr [#364])
- deps: update rust crate tokio to 1.40.0(pr [#365])
- cli: pass version to push_committed function in run_release(pr [#366])
- cli: pass version to commit_changed_files function in run_release(pr [#368])

## [0.4.5] - 2024-09-07

### Added

- update renovate.json with new configuration options(pr [#350])

### Changed

- refactor(client)-simplify branch_or_main method using map_or(pr [#347])
- chore-rename renovate.json to do-not-renovate.json(pr [#348])
- chore-Configure Renovate(pr [#349])
- refactor-remove extends key from renovate.json(pr [#352])

### Fixed

- update rangeStrategy to auto in renovate.json(pr [#351])

## [0.4.4] - 2024-09-05

### Fixed

- deps: update rust crate named-colour to 0.3.0(pr [#346])

## [0.4.3] - 2024-09-05

### Fixed

- deps: update rust crate named-colour to 0.2.0(pr [#344])

### Security

- Dependencies: bump quinn-proto from 0.11.6 to 0.11.8 in the cargo group across 1 directory(pr [#345])

## [0.4.2] - 2024-08-30

### Added

- use commit and push logic for changelog update(pr [#301])
- add Rebase command to CLI(pr [#304])
- add graphql method to get open pull requests(pr [#306])
- add PrItem struct and return Vec<PrItem> instead of Vec<Edge>(pr [#307])
- add early return if no open pull requests are found(pr [#308])
- run rebase only when check run on main(pr [#311])
- add label support for pull requests(pr [#314])
- integrate tracing library for enhanced logging(pr [#318])
- add tracing-subscriber for enhanced logging(pr [#319])
- add repository ID retrieval and improve label handling(pr [#320])
- add create_label function and update label_pr to use it(pr [#323])
- add new GraphQL operations for labels, PRs, and repository(pr [#324])
- add filtering by login in rebase_next_pr(pr [#335])
- add login option to rebase command(pr [#337])
- add label to pull request during rebase(pr [#338])
- add label option for rebase command(pr [#339])
- add description option for rebase label(pr [#340])
- add colour option for labels in rebase command(pr [#342])

### Changed

- chore-remove commented out debug logs and unused code(pr [#302])
- remove-remove customised commit and push for changelogs(pr [#303])
- ci-integrate rebase command into change update job(pr [#305])
- ci-add check for pull request before running cl update(pr [#312])
- ci-modify conditional logic for result variable and API call(pr [#313])
- refactor(label)-update GetLabelID structure and related types(pr [#316])
- fix(: skip deserializing owner and name fields in Repository struct(pr [#322])
- refactor(label_pr)-remove debug trace statement from GraphQLLabelPR implementation(pr [#334])
- refactor-rename 'rebase' command to 'label' in CI configuration and CLI(pr [#343])

### Fixed

- add condition to execute pcu rebase only if CIRCLE_PULL_REQUEST is set(pr [#309])
- rename GetPullRequestTitle to GetLabelID and remove redundant log trace(pr [#315])
- comment out serde rename attribute for label field(pr [#317])
- add owner and name fields to repository query(pr [#321])
- label_pr: simplify GraphQL mutation parameters for labeling PRs(pr [#325])
- remove unused fields 'name' and 'color' from Label struct in label_pr.rs(pr [#326])
- graphql: correct trace log message from vars to mutation in label_pr.rs(pr [#327])
- label_pr: replace ID with name in Label struct and GraphQL query(pr [#328])
- graphql: correct label_id type in mutation from String to ID(pr [#329])
- correct typo in GraphQL mutation query in label_pr.rs(pr [#330])
- correct typos in GraphQL mutation for adding labels to PR(pr [#331])
- graphql: correct typo in 'labelable' struct field name in label_pr.rs(pr [#332])
- graphql: rename GetPullRequestTitle struct to Data and enhance Data struct in label_pr.rs(pr [#333])
- git_ops: add early exit for no open PRs scenario following filter to login(pr [#336])
- update test label in CircleCI config and add description parameter to GraphQL mutation(pr [#341])

## [0.4.1] - 2024-08-24

### Added

- add push_commit method and refactor branch method to branch_or_main(pr [#286])
- add commit and reduce to push only(pr [#288])
- add semver option to Push struct and tag_opt method(pr [#289])
- fallback to graphql client for fetching pull request title(pr [#291])
- better organisation of code(pr [#294])
- add support for gql_client with headers in get_github_apis(pr [#295])
- use GraphQL for pull request title retrieval(pr [#299])

### Changed

- chore-add logging for pull request response(pr [#290])
- refactor-remove unused github_graphql variable and related code in pull_request.rs(pr [#300])

### Fixed

- integrate get_authenticated_remote into push_commit method(pr [#287])
- add headers and refine error handling(pr [#293])
- add owner and name fields to repository query(pr [#296])
- add skip_deserializing attribute to owner and name fields in Repository struct(pr [#297])
- remove unnecessary owner and name fields from get_pull_request_title query(pr [#298])

## [0.4.0] - 2024-08-17

### Added

- add commit_staged function to GitOps trait(pr [#285])

## [0.3.0] - 2024-08-17

### Added

- BREAKING: add push command to CLI to push any changes to the remote repository(pr [#282])
- list the unstaged files(pr [#283])
- add stage_files function to GitOps trait and implement it in Client(pr [#284])

### Changed

- refactor-restructure project directories and update paths in Cargo.toml(pr [#281])

## [0.2.0] - 2024-08-15

### Added

- replace octocrab with octocrate in client and pull_request modules(pr [#264])
- BREAKING: add GitHub App authentication support(pr [#272])
- add pcu-app to context in workflows configuration(pr [#273])
- add line limit parameter to print_changelog function(pr [#275])
- add config for line limit(pr [#277])
- use ANSI_term for styled console output(pr [#279])

### Changed

- refactor-replace octocrab with octocrate in various modules(pr [#265])
- ci-upgrade jerus-org/circleci-toolkit orb version from 0.25.0 to 1.0.0(pr [#268])
- refactor-remove settings field from Client struct, add git_api, default_branch, commit_message fields(pr [#271])
- ci-increase pcu_verbosity level from -vvv to -vvvv(pr [#276])
- refactor-change print_changelog function to return string instead of printing directly(pr [#280])

### Fixed

- deps: update rust crate clap to 4.5.15(pr [#266])
- deps: update rust crate clap-verbosity-flag to 2.2.1(pr [#267])
- deps: update rust crate env_logger to 0.11.5(pr [#269])
- deps: update rust crate regex to 1.10.6(pr [#270])
- remove redundant GitHubAPI instantiation in new_pull_request_opt function(pr [#274])
- limit lines from file in pr_title(pr [#278])

## [0.1.26] - 2024-08-10

### Changed

- chore-track get commitish(pr [#263])

### Fixed

- deps: update rust crate rstest to 0.22.0(pr [#262])

## [0.1.25] - 2024-08-03

### Added

- extract git operations to a separate module(pr [#243])
- add new module ops(pr [#253])
- add new module and ReleaseUnreleased trait and implementation(pr [#254])
- add ChangelogParseOptions to Client struct and default value for version_prefix(pr [#255])

### Changed

- chore-add logging for tag retrieval in get_commitish_for_tag function(pr [#233])
- refactor-simplify version tag reference formatting and logging(pr [#238])
- chore-add trace log for .git/refs directory content(pr [#239])
- chore-list the tags in the git repo(pr [#240])
- chore-more precise look at ref and remove from check(pr [#244])
- chore-restore tag push(pr [#246])
- chore-add trace logs for debugging in list_tags function(pr [#247])
- chore-check if file is empty or error in read(pr [#248])
- chore-handle file read errors gracefully in list_tags function(pr [#249])
- chore-correct filename logging in list_tags function(pr [#250])
- chore-replace dynamic filename generation with static filename in list_tags function(pr [#251])
- refactor-run_release function in main.rs and simplify make_release function(pr [#256])
- refactor-remove unused import and code, refactor tag_ref assignment and logging(pr [#258])

### Fixed

- handle case were we are pushing from the main branch(pr [#231])
- default branch name to 'main' if branch is not specified(pr [#232])
- push the version tag to the repo(pr [#234])
- add optional tag parameter to commit_changelog and commit_changelog_gpg functions(pr [#235])
- change commit_changelog_gpg method to mutable(pr [#236])
- modify version_tag reference format in git_repo.find_reference method(pr [#237])
- prepend 'v' to version_tag before tagging commit(pr [#241])
- replace version parameter with tag in get_commitish_for_tag method(pr [#245])
- correct format of version_tag in tag_ref string(pr [#252])
- pass version to commit_changelog_gpg and commit_changelog methods(pr [#257])
- pass version to push_changelog function instead of None(pr [#259])
- replace 'svs_root' and 'scs_root' with 'dev_platform' in client/mod.rs and main.rs(pr [#260])
- change head reference from 'main' to 'HEAD' in ChangelogParseOptions(pr [#261])

### Security

- Dependencies: update rust crate octocrab to 0.39.0(pr [#242])

## [0.1.24] - 2024-07-25

### Changed

- ci-adopt revised toolkit(pr [#230])

## [0.1.23] - 2024-07-25

### Changed

- refactor-extract repeated code into commands set_semver, make_cargo_release, make_github_release(pr [#229])

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
- add early_exit flag for signalling an early exit(pr [#177])

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
- chore-remove cargo release comment replacements(pr [#147](https://github.com/jerus-org/pcu/pull/147))
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
- refactor-migrate const to settings(pr [#135](https://github.com/jerus-org/pcu/pull/135))
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
[#231]: https://github.com/jerus-org/pcu/pull/231
[#232]: https://github.com/jerus-org/pcu/pull/232
[#233]: https://github.com/jerus-org/pcu/pull/233
[#234]: https://github.com/jerus-org/pcu/pull/234
[#235]: https://github.com/jerus-org/pcu/pull/235
[#236]: https://github.com/jerus-org/pcu/pull/236
[#237]: https://github.com/jerus-org/pcu/pull/237
[#238]: https://github.com/jerus-org/pcu/pull/238
[#239]: https://github.com/jerus-org/pcu/pull/239
[#240]: https://github.com/jerus-org/pcu/pull/240
[#241]: https://github.com/jerus-org/pcu/pull/241
[#242]: https://github.com/jerus-org/pcu/pull/242
[#243]: https://github.com/jerus-org/pcu/pull/243
[#244]: https://github.com/jerus-org/pcu/pull/244
[#245]: https://github.com/jerus-org/pcu/pull/245
[#246]: https://github.com/jerus-org/pcu/pull/246
[#247]: https://github.com/jerus-org/pcu/pull/247
[#248]: https://github.com/jerus-org/pcu/pull/248
[#249]: https://github.com/jerus-org/pcu/pull/249
[#250]: https://github.com/jerus-org/pcu/pull/250
[#251]: https://github.com/jerus-org/pcu/pull/251
[#252]: https://github.com/jerus-org/pcu/pull/252
[#253]: https://github.com/jerus-org/pcu/pull/253
[#254]: https://github.com/jerus-org/pcu/pull/254
[#255]: https://github.com/jerus-org/pcu/pull/255
[#256]: https://github.com/jerus-org/pcu/pull/256
[#257]: https://github.com/jerus-org/pcu/pull/257
[#258]: https://github.com/jerus-org/pcu/pull/258
[#259]: https://github.com/jerus-org/pcu/pull/259
[#260]: https://github.com/jerus-org/pcu/pull/260
[#261]: https://github.com/jerus-org/pcu/pull/261
[#262]: https://github.com/jerus-org/pcu/pull/262
[#263]: https://github.com/jerus-org/pcu/pull/263
[#264]: https://github.com/jerus-org/pcu/pull/264
[#265]: https://github.com/jerus-org/pcu/pull/265
[#266]: https://github.com/jerus-org/pcu/pull/266
[#267]: https://github.com/jerus-org/pcu/pull/267
[#268]: https://github.com/jerus-org/pcu/pull/268
[#269]: https://github.com/jerus-org/pcu/pull/269
[#270]: https://github.com/jerus-org/pcu/pull/270
[#271]: https://github.com/jerus-org/pcu/pull/271
[#272]: https://github.com/jerus-org/pcu/pull/272
[#273]: https://github.com/jerus-org/pcu/pull/273
[#274]: https://github.com/jerus-org/pcu/pull/274
[#275]: https://github.com/jerus-org/pcu/pull/275
[#276]: https://github.com/jerus-org/pcu/pull/276
[#277]: https://github.com/jerus-org/pcu/pull/277
[#278]: https://github.com/jerus-org/pcu/pull/278
[#279]: https://github.com/jerus-org/pcu/pull/279
[#280]: https://github.com/jerus-org/pcu/pull/280
[#281]: https://github.com/jerus-org/pcu/pull/281
[#282]: https://github.com/jerus-org/pcu/pull/282
[#283]: https://github.com/jerus-org/pcu/pull/283
[#284]: https://github.com/jerus-org/pcu/pull/284
[#285]: https://github.com/jerus-org/pcu/pull/285
[#286]: https://github.com/jerus-org/pcu/pull/286
[#287]: https://github.com/jerus-org/pcu/pull/287
[#288]: https://github.com/jerus-org/pcu/pull/288
[#289]: https://github.com/jerus-org/pcu/pull/289
[#290]: https://github.com/jerus-org/pcu/pull/290
[#291]: https://github.com/jerus-org/pcu/pull/291
[#293]: https://github.com/jerus-org/pcu/pull/293
[#294]: https://github.com/jerus-org/pcu/pull/294
[#295]: https://github.com/jerus-org/pcu/pull/295
[#296]: https://github.com/jerus-org/pcu/pull/296
[#297]: https://github.com/jerus-org/pcu/pull/297
[#298]: https://github.com/jerus-org/pcu/pull/298
[#299]: https://github.com/jerus-org/pcu/pull/299
[#300]: https://github.com/jerus-org/pcu/pull/300
[#301]: https://github.com/jerus-org/pcu/pull/301
[#302]: https://github.com/jerus-org/pcu/pull/302
[#303]: https://github.com/jerus-org/pcu/pull/303
[#304]: https://github.com/jerus-org/pcu/pull/304
[#305]: https://github.com/jerus-org/pcu/pull/305
[#306]: https://github.com/jerus-org/pcu/pull/306
[#307]: https://github.com/jerus-org/pcu/pull/307
[#308]: https://github.com/jerus-org/pcu/pull/308
[#309]: https://github.com/jerus-org/pcu/pull/309
[#311]: https://github.com/jerus-org/pcu/pull/311
[#312]: https://github.com/jerus-org/pcu/pull/312
[#313]: https://github.com/jerus-org/pcu/pull/313
[#314]: https://github.com/jerus-org/pcu/pull/314
[#315]: https://github.com/jerus-org/pcu/pull/315
[#316]: https://github.com/jerus-org/pcu/pull/316
[#317]: https://github.com/jerus-org/pcu/pull/317
[#318]: https://github.com/jerus-org/pcu/pull/318
[#319]: https://github.com/jerus-org/pcu/pull/319
[#320]: https://github.com/jerus-org/pcu/pull/320
[#321]: https://github.com/jerus-org/pcu/pull/321
[#322]: https://github.com/jerus-org/pcu/pull/322
[#323]: https://github.com/jerus-org/pcu/pull/323
[#324]: https://github.com/jerus-org/pcu/pull/324
[#325]: https://github.com/jerus-org/pcu/pull/325
[#326]: https://github.com/jerus-org/pcu/pull/326
[#327]: https://github.com/jerus-org/pcu/pull/327
[#328]: https://github.com/jerus-org/pcu/pull/328
[#329]: https://github.com/jerus-org/pcu/pull/329
[#330]: https://github.com/jerus-org/pcu/pull/330
[#331]: https://github.com/jerus-org/pcu/pull/331
[#332]: https://github.com/jerus-org/pcu/pull/332
[#333]: https://github.com/jerus-org/pcu/pull/333
[#334]: https://github.com/jerus-org/pcu/pull/334
[#335]: https://github.com/jerus-org/pcu/pull/335
[#336]: https://github.com/jerus-org/pcu/pull/336
[#337]: https://github.com/jerus-org/pcu/pull/337
[#338]: https://github.com/jerus-org/pcu/pull/338
[#339]: https://github.com/jerus-org/pcu/pull/339
[#340]: https://github.com/jerus-org/pcu/pull/340
[#341]: https://github.com/jerus-org/pcu/pull/341
[#342]: https://github.com/jerus-org/pcu/pull/342
[#343]: https://github.com/jerus-org/pcu/pull/343
[#344]: https://github.com/jerus-org/pcu/pull/344
[#345]: https://github.com/jerus-org/pcu/pull/345
[#346]: https://github.com/jerus-org/pcu/pull/346
[#347]: https://github.com/jerus-org/pcu/pull/347
[#348]: https://github.com/jerus-org/pcu/pull/348
[#349]: https://github.com/jerus-org/pcu/pull/349
[#350]: https://github.com/jerus-org/pcu/pull/350
[#351]: https://github.com/jerus-org/pcu/pull/351
[#352]: https://github.com/jerus-org/pcu/pull/352
[#353]: https://github.com/jerus-org/pcu/pull/353
[#354]: https://github.com/jerus-org/pcu/pull/354
[#355]: https://github.com/jerus-org/pcu/pull/355
[#356]: https://github.com/jerus-org/pcu/pull/356
[#357]: https://github.com/jerus-org/pcu/pull/357
[#358]: https://github.com/jerus-org/pcu/pull/358
[#359]: https://github.com/jerus-org/pcu/pull/359
[#360]: https://github.com/jerus-org/pcu/pull/360
[#361]: https://github.com/jerus-org/pcu/pull/361
[#363]: https://github.com/jerus-org/pcu/pull/363
[#362]: https://github.com/jerus-org/pcu/pull/362
[#364]: https://github.com/jerus-org/pcu/pull/364
[#365]: https://github.com/jerus-org/pcu/pull/365
[#366]: https://github.com/jerus-org/pcu/pull/366
[#367]: https://github.com/jerus-org/pcu/pull/367
[#368]: https://github.com/jerus-org/pcu/pull/368
[#369]: https://github.com/jerus-org/pcu/pull/369
[#372]: https://github.com/jerus-org/pcu/pull/372
[#371]: https://github.com/jerus-org/pcu/pull/371
[#373]: https://github.com/jerus-org/pcu/pull/373
[#374]: https://github.com/jerus-org/pcu/pull/374
[#375]: https://github.com/jerus-org/pcu/pull/375
[#376]: https://github.com/jerus-org/pcu/pull/376
[#377]: https://github.com/jerus-org/pcu/pull/377
[#378]: https://github.com/jerus-org/pcu/pull/378
[#379]: https://github.com/jerus-org/pcu/pull/379
[#380]: https://github.com/jerus-org/pcu/pull/380
[#381]: https://github.com/jerus-org/pcu/pull/381
[#382]: https://github.com/jerus-org/pcu/pull/382
[#384]: https://github.com/jerus-org/pcu/pull/384
[#383]: https://github.com/jerus-org/pcu/pull/383
[#385]: https://github.com/jerus-org/pcu/pull/385
[#386]: https://github.com/jerus-org/pcu/pull/386
[#387]: https://github.com/jerus-org/pcu/pull/387
[#388]: https://github.com/jerus-org/pcu/pull/388
[#389]: https://github.com/jerus-org/pcu/pull/389
[#390]: https://github.com/jerus-org/pcu/pull/390
[#391]: https://github.com/jerus-org/pcu/pull/391
[#392]: https://github.com/jerus-org/pcu/pull/392
[#393]: https://github.com/jerus-org/pcu/pull/393
[#394]: https://github.com/jerus-org/pcu/pull/394
[#396]: https://github.com/jerus-org/pcu/pull/396
[#397]: https://github.com/jerus-org/pcu/pull/397
[#398]: https://github.com/jerus-org/pcu/pull/398
[#399]: https://github.com/jerus-org/pcu/pull/399
[#400]: https://github.com/jerus-org/pcu/pull/400
[#401]: https://github.com/jerus-org/pcu/pull/401
[#402]: https://github.com/jerus-org/pcu/pull/402
[#404]: https://github.com/jerus-org/pcu/pull/404
[#403]: https://github.com/jerus-org/pcu/pull/403
[#405]: https://github.com/jerus-org/pcu/pull/405
[#406]: https://github.com/jerus-org/pcu/pull/406
[#407]: https://github.com/jerus-org/pcu/pull/407
[#408]: https://github.com/jerus-org/pcu/pull/408
[#409]: https://github.com/jerus-org/pcu/pull/409
[#410]: https://github.com/jerus-org/pcu/pull/410
[#411]: https://github.com/jerus-org/pcu/pull/411
[#412]: https://github.com/jerus-org/pcu/pull/412
[#413]: https://github.com/jerus-org/pcu/pull/413
[#415]: https://github.com/jerus-org/pcu/pull/415
[#414]: https://github.com/jerus-org/pcu/pull/414
[#416]: https://github.com/jerus-org/pcu/pull/416
[#417]: https://github.com/jerus-org/pcu/pull/417
[#418]: https://github.com/jerus-org/pcu/pull/418
[#420]: https://github.com/jerus-org/pcu/pull/420
[#419]: https://github.com/jerus-org/pcu/pull/419
[#421]: https://github.com/jerus-org/pcu/pull/421
[#422]: https://github.com/jerus-org/pcu/pull/422
[#423]: https://github.com/jerus-org/pcu/pull/423
[#424]: https://github.com/jerus-org/pcu/pull/424
[#425]: https://github.com/jerus-org/pcu/pull/425
[#426]: https://github.com/jerus-org/pcu/pull/426
[#427]: https://github.com/jerus-org/pcu/pull/427
[#428]: https://github.com/jerus-org/pcu/pull/428
[#429]: https://github.com/jerus-org/pcu/pull/429
[#430]: https://github.com/jerus-org/pcu/pull/430
[#431]: https://github.com/jerus-org/pcu/pull/431
[#432]: https://github.com/jerus-org/pcu/pull/432
[#433]: https://github.com/jerus-org/pcu/pull/433
[#434]: https://github.com/jerus-org/pcu/pull/434
[#435]: https://github.com/jerus-org/pcu/pull/435
[#436]: https://github.com/jerus-org/pcu/pull/436
[#437]: https://github.com/jerus-org/pcu/pull/437
[#439]: https://github.com/jerus-org/pcu/pull/439
[#438]: https://github.com/jerus-org/pcu/pull/438
[#440]: https://github.com/jerus-org/pcu/pull/440
[#441]: https://github.com/jerus-org/pcu/pull/441
[#442]: https://github.com/jerus-org/pcu/pull/442
[#443]: https://github.com/jerus-org/pcu/pull/443
[#444]: https://github.com/jerus-org/pcu/pull/444
[#445]: https://github.com/jerus-org/pcu/pull/445
[#446]: https://github.com/jerus-org/pcu/pull/446
[#447]: https://github.com/jerus-org/pcu/pull/447
[#448]: https://github.com/jerus-org/pcu/pull/448
[#449]: https://github.com/jerus-org/pcu/pull/449
[#450]: https://github.com/jerus-org/pcu/pull/450
[#451]: https://github.com/jerus-org/pcu/pull/451
[#452]: https://github.com/jerus-org/pcu/pull/452
[#453]: https://github.com/jerus-org/pcu/pull/453
[#454]: https://github.com/jerus-org/pcu/pull/454
[#455]: https://github.com/jerus-org/pcu/pull/455
[#456]: https://github.com/jerus-org/pcu/pull/456
[#457]: https://github.com/jerus-org/pcu/pull/457
[#458]: https://github.com/jerus-org/pcu/pull/458
[#459]: https://github.com/jerus-org/pcu/pull/459
[#460]: https://github.com/jerus-org/pcu/pull/460
[#461]: https://github.com/jerus-org/pcu/pull/461
[#462]: https://github.com/jerus-org/pcu/pull/462
[#463]: https://github.com/jerus-org/pcu/pull/463
[#464]: https://github.com/jerus-org/pcu/pull/464
[#465]: https://github.com/jerus-org/pcu/pull/465
[#466]: https://github.com/jerus-org/pcu/pull/466
[#467]: https://github.com/jerus-org/pcu/pull/467
[#468]: https://github.com/jerus-org/pcu/pull/468
[#469]: https://github.com/jerus-org/pcu/pull/469
[#470]: https://github.com/jerus-org/pcu/pull/470
[#471]: https://github.com/jerus-org/pcu/pull/471
[#472]: https://github.com/jerus-org/pcu/pull/472
[#473]: https://github.com/jerus-org/pcu/pull/473
[#474]: https://github.com/jerus-org/pcu/pull/474
[#475]: https://github.com/jerus-org/pcu/pull/475
[#476]: https://github.com/jerus-org/pcu/pull/476
[#477]: https://github.com/jerus-org/pcu/pull/477
[#478]: https://github.com/jerus-org/pcu/pull/478
[#480]: https://github.com/jerus-org/pcu/pull/480
[#481]: https://github.com/jerus-org/pcu/pull/481
[#482]: https://github.com/jerus-org/pcu/pull/482
[#483]: https://github.com/jerus-org/pcu/pull/483
[#484]: https://github.com/jerus-org/pcu/pull/484
[#485]: https://github.com/jerus-org/pcu/pull/485
[#486]: https://github.com/jerus-org/pcu/pull/486
[#487]: https://github.com/jerus-org/pcu/pull/487
[#488]: https://github.com/jerus-org/pcu/pull/488
[#489]: https://github.com/jerus-org/pcu/pull/489
[#490]: https://github.com/jerus-org/pcu/pull/490
[#491]: https://github.com/jerus-org/pcu/pull/491
[#492]: https://github.com/jerus-org/pcu/pull/492
[#493]: https://github.com/jerus-org/pcu/pull/493
[#494]: https://github.com/jerus-org/pcu/pull/494
[#495]: https://github.com/jerus-org/pcu/pull/495
[#496]: https://github.com/jerus-org/pcu/pull/496
[#497]: https://github.com/jerus-org/pcu/pull/497
[#498]: https://github.com/jerus-org/pcu/pull/498
[#499]: https://github.com/jerus-org/pcu/pull/499
[#500]: https://github.com/jerus-org/pcu/pull/500
[#501]: https://github.com/jerus-org/pcu/pull/501
[#502]: https://github.com/jerus-org/pcu/pull/502
[#505]: https://github.com/jerus-org/pcu/pull/505
[#503]: https://github.com/jerus-org/pcu/pull/503
[#504]: https://github.com/jerus-org/pcu/pull/504
[#506]: https://github.com/jerus-org/pcu/pull/506
[#507]: https://github.com/jerus-org/pcu/pull/507
[#508]: https://github.com/jerus-org/pcu/pull/508
[#509]: https://github.com/jerus-org/pcu/pull/509
[#510]: https://github.com/jerus-org/pcu/pull/510
[#511]: https://github.com/jerus-org/pcu/pull/511
[#512]: https://github.com/jerus-org/pcu/pull/512
[#513]: https://github.com/jerus-org/pcu/pull/513
[#514]: https://github.com/jerus-org/pcu/pull/514
[#515]: https://github.com/jerus-org/pcu/pull/515
[#516]: https://github.com/jerus-org/pcu/pull/516
[#517]: https://github.com/jerus-org/pcu/pull/517
[#518]: https://github.com/jerus-org/pcu/pull/518
[#519]: https://github.com/jerus-org/pcu/pull/519
[#520]: https://github.com/jerus-org/pcu/pull/520
[#523]: https://github.com/jerus-org/pcu/pull/523
[#524]: https://github.com/jerus-org/pcu/pull/524
[#525]: https://github.com/jerus-org/pcu/pull/525
[#526]: https://github.com/jerus-org/pcu/pull/526
[#527]: https://github.com/jerus-org/pcu/pull/527
[#528]: https://github.com/jerus-org/pcu/pull/528
[#529]: https://github.com/jerus-org/pcu/pull/529
[#530]: https://github.com/jerus-org/pcu/pull/530
[#531]: https://github.com/jerus-org/pcu/pull/531
[#532]: https://github.com/jerus-org/pcu/pull/532
[#533]: https://github.com/jerus-org/pcu/pull/533
[#534]: https://github.com/jerus-org/pcu/pull/534
[#535]: https://github.com/jerus-org/pcu/pull/535
[#537]: https://github.com/jerus-org/pcu/pull/537
[#538]: https://github.com/jerus-org/pcu/pull/538
[#539]: https://github.com/jerus-org/pcu/pull/539
[#540]: https://github.com/jerus-org/pcu/pull/540
[#541]: https://github.com/jerus-org/pcu/pull/541
[#542]: https://github.com/jerus-org/pcu/pull/542
[#543]: https://github.com/jerus-org/pcu/pull/543
[#544]: https://github.com/jerus-org/pcu/pull/544
[#545]: https://github.com/jerus-org/pcu/pull/545
[#546]: https://github.com/jerus-org/pcu/pull/546
[#547]: https://github.com/jerus-org/pcu/pull/547
[#548]: https://github.com/jerus-org/pcu/pull/548
[#549]: https://github.com/jerus-org/pcu/pull/549
[#554]: https://github.com/jerus-org/pcu/pull/554
[#550]: https://github.com/jerus-org/pcu/pull/550
[#551]: https://github.com/jerus-org/pcu/pull/551
[#552]: https://github.com/jerus-org/pcu/pull/552
[#555]: https://github.com/jerus-org/pcu/pull/555
[#556]: https://github.com/jerus-org/pcu/pull/556
[#558]: https://github.com/jerus-org/pcu/pull/558
[#559]: https://github.com/jerus-org/pcu/pull/559
[#560]: https://github.com/jerus-org/pcu/pull/560
[#561]: https://github.com/jerus-org/pcu/pull/561
[#562]: https://github.com/jerus-org/pcu/pull/562
[#563]: https://github.com/jerus-org/pcu/pull/563
[#564]: https://github.com/jerus-org/pcu/pull/564
[#565]: https://github.com/jerus-org/pcu/pull/565
[#566]: https://github.com/jerus-org/pcu/pull/566
[#567]: https://github.com/jerus-org/pcu/pull/567
[#568]: https://github.com/jerus-org/pcu/pull/568
[#569]: https://github.com/jerus-org/pcu/pull/569
[#570]: https://github.com/jerus-org/pcu/pull/570
[#571]: https://github.com/jerus-org/pcu/pull/571
[#572]: https://github.com/jerus-org/pcu/pull/572
[#573]: https://github.com/jerus-org/pcu/pull/573
[#574]: https://github.com/jerus-org/pcu/pull/574
[#575]: https://github.com/jerus-org/pcu/pull/575
[#576]: https://github.com/jerus-org/pcu/pull/576
[#577]: https://github.com/jerus-org/pcu/pull/577
[#578]: https://github.com/jerus-org/pcu/pull/578
[#579]: https://github.com/jerus-org/pcu/pull/579
[#580]: https://github.com/jerus-org/pcu/pull/580
[#581]: https://github.com/jerus-org/pcu/pull/581
[#582]: https://github.com/jerus-org/pcu/pull/582
[#584]: https://github.com/jerus-org/pcu/pull/584
[#585]: https://github.com/jerus-org/pcu/pull/585
[#586]: https://github.com/jerus-org/pcu/pull/586
[#587]: https://github.com/jerus-org/pcu/pull/587
[#588]: https://github.com/jerus-org/pcu/pull/588
[#589]: https://github.com/jerus-org/pcu/pull/589
[#590]: https://github.com/jerus-org/pcu/pull/590
[#591]: https://github.com/jerus-org/pcu/pull/591
[#592]: https://github.com/jerus-org/pcu/pull/592
[#593]: https://github.com/jerus-org/pcu/pull/593
[#594]: https://github.com/jerus-org/pcu/pull/594
[#595]: https://github.com/jerus-org/pcu/pull/595
[#596]: https://github.com/jerus-org/pcu/pull/596
[#597]: https://github.com/jerus-org/pcu/pull/597
[#598]: https://github.com/jerus-org/pcu/pull/598
[#599]: https://github.com/jerus-org/pcu/pull/599
[#600]: https://github.com/jerus-org/pcu/pull/600
[#601]: https://github.com/jerus-org/pcu/pull/601
[#602]: https://github.com/jerus-org/pcu/pull/602
[#603]: https://github.com/jerus-org/pcu/pull/603
[#604]: https://github.com/jerus-org/pcu/pull/604
[#605]: https://github.com/jerus-org/pcu/pull/605
[#606]: https://github.com/jerus-org/pcu/pull/606
[#607]: https://github.com/jerus-org/pcu/pull/607
[#608]: https://github.com/jerus-org/pcu/pull/608
[#609]: https://github.com/jerus-org/pcu/pull/609
[#610]: https://github.com/jerus-org/pcu/pull/610
[#611]: https://github.com/jerus-org/pcu/pull/611
[#613]: https://github.com/jerus-org/pcu/pull/613
[#613]: https://github.com/jerus-org/pcu/pull/613
[#614]: https://github.com/jerus-org/pcu/pull/614
[#615]: https://github.com/jerus-org/pcu/pull/615
[#616]: https://github.com/jerus-org/pcu/pull/616
[#617]: https://github.com/jerus-org/pcu/pull/617
[#618]: https://github.com/jerus-org/pcu/pull/618
[#619]: https://github.com/jerus-org/pcu/pull/619
[#620]: https://github.com/jerus-org/pcu/pull/620
[#621]: https://github.com/jerus-org/pcu/pull/621
[#622]: https://github.com/jerus-org/pcu/pull/622
[#623]: https://github.com/jerus-org/pcu/pull/623
[#624]: https://github.com/jerus-org/pcu/pull/624
[#625]: https://github.com/jerus-org/pcu/pull/625
[#626]: https://github.com/jerus-org/pcu/pull/626
[#627]: https://github.com/jerus-org/pcu/pull/627
[#628]: https://github.com/jerus-org/pcu/pull/628
[#629]: https://github.com/jerus-org/pcu/pull/629
[#630]: https://github.com/jerus-org/pcu/pull/630
[#631]: https://github.com/jerus-org/pcu/pull/631
[#633]: https://github.com/jerus-org/pcu/pull/633
[#634]: https://github.com/jerus-org/pcu/pull/634
[#635]: https://github.com/jerus-org/pcu/pull/635
[#636]: https://github.com/jerus-org/pcu/pull/636
[#637]: https://github.com/jerus-org/pcu/pull/637
[#638]: https://github.com/jerus-org/pcu/pull/638
[#639]: https://github.com/jerus-org/pcu/pull/639
[#640]: https://github.com/jerus-org/pcu/pull/640
[#641]: https://github.com/jerus-org/pcu/pull/641
[#642]: https://github.com/jerus-org/pcu/pull/642
[#643]: https://github.com/jerus-org/pcu/pull/643
[#644]: https://github.com/jerus-org/pcu/pull/644
[#645]: https://github.com/jerus-org/pcu/pull/645
[#646]: https://github.com/jerus-org/pcu/pull/646
[#647]: https://github.com/jerus-org/pcu/pull/647
[#648]: https://github.com/jerus-org/pcu/pull/648
[#649]: https://github.com/jerus-org/pcu/pull/649
[#650]: https://github.com/jerus-org/pcu/pull/650
[#651]: https://github.com/jerus-org/pcu/pull/651
[#653]: https://github.com/jerus-org/pcu/pull/653
[#654]: https://github.com/jerus-org/pcu/pull/654
[#655]: https://github.com/jerus-org/pcu/pull/655
[#656]: https://github.com/jerus-org/pcu/pull/656
[Unreleased]: https://github.com/jerus-org/pcu/compare/v0.4.56...HEAD
[0.4.56]: https://github.com/jerus-org/pcu/compare/v0.4.55...v0.4.56
[0.4.55]: https://github.com/jerus-org/pcu/compare/v0.4.54...v0.4.55
[0.4.54]: https://github.com/jerus-org/pcu/compare/v0.4.53...v0.4.54
[0.4.53]: https://github.com/jerus-org/pcu/compare/v0.4.52...v0.4.53
[0.4.52]: https://github.com/jerus-org/pcu/compare/v0.4.51...v0.4.52
[0.4.51]: https://github.com/jerus-org/pcu/compare/v0.4.50...v0.4.51
[0.4.50]: https://github.com/jerus-org/pcu/compare/v0.4.49...v0.4.50
[0.4.49]: https://github.com/jerus-org/pcu/compare/v0.4.48...v0.4.49
[0.4.48]: https://github.com/jerus-org/pcu/compare/v0.4.45...v0.4.48
[0.4.45]: https://github.com/jerus-org/pcu/compare/v0.4.45...v0.4.45
[0.4.45]: https://github.com/jerus-org/pcu/compare/v0.4.44...v0.4.45
[0.4.44]: https://github.com/jerus-org/pcu/compare/v0.4.43...v0.4.44
[0.4.43]: https://github.com/jerus-org/pcu/compare/v0.4.42...v0.4.43
[0.4.42]: https://github.com/jerus-org/pcu/compare/v0.4.41...v0.4.42
[0.4.41]: https://github.com/jerus-org/pcu/compare/v0.4.40...v0.4.41
[0.4.40]: https://github.com/jerus-org/pcu/compare/v0.4.39...v0.4.40
[0.4.39]: https://github.com/jerus-org/pcu/compare/v0.4.38...v0.4.39
[0.4.38]: https://github.com/jerus-org/pcu/compare/v0.4.37...v0.4.38
[0.4.37]: https://github.com/jerus-org/pcu/compare/v0.4.36...v0.4.37
[0.4.36]: https://github.com/jerus-org/pcu/compare/v0.4.35...v0.4.36
[0.4.35]: https://github.com/jerus-org/pcu/compare/v0.4.34...v0.4.35
[0.4.34]: https://github.com/jerus-org/pcu/compare/v0.4.33...v0.4.34
[0.4.33]: https://github.com/jerus-org/pcu/compare/v0.4.32...v0.4.33
[0.4.32]: https://github.com/jerus-org/pcu/compare/v0.4.31...v0.4.32
[0.4.31]: https://github.com/jerus-org/pcu/compare/v0.4.30...v0.4.31
[0.4.30]: https://github.com/jerus-org/pcu/compare/v0.4.29...v0.4.30
[0.4.29]: https://github.com/jerus-org/pcu/compare/v0.4.28...v0.4.29
[0.4.28]: https://github.com/jerus-org/pcu/compare/v0.4.27...v0.4.28
[0.4.27]: https://github.com/jerus-org/pcu/compare/v0.4.26...v0.4.27
[0.4.26]: https://github.com/jerus-org/pcu/compare/v0.4.25...v0.4.26
[0.4.25]: https://github.com/jerus-org/pcu/compare/v0.4.24...v0.4.25
[0.4.24]: https://github.com/jerus-org/pcu/compare/v0.4.23...v0.4.24
[0.4.23]: https://github.com/jerus-org/pcu/compare/v0.4.22...v0.4.23
[0.4.22]: https://github.com/jerus-org/pcu/compare/v0.4.21...v0.4.22
[0.4.21]: https://github.com/jerus-org/pcu/compare/v0.4.20...v0.4.21
[0.4.20]: https://github.com/jerus-org/pcu/compare/v0.4.19...v0.4.20
[0.4.19]: https://github.com/jerus-org/pcu/compare/v0.4.18...v0.4.19
[0.4.18]: https://github.com/jerus-org/pcu/compare/v0.4.17...v0.4.18
[0.4.17]: https://github.com/jerus-org/pcu/compare/v0.4.16...v0.4.17
[0.4.16]: https://github.com/jerus-org/pcu/compare/v0.4.15...v0.4.16
[0.4.15]: https://github.com/jerus-org/pcu/compare/v0.4.14...v0.4.15
[0.4.14]: https://github.com/jerus-org/pcu/compare/v0.4.13...v0.4.14
[0.4.13]: https://github.com/jerus-org/pcu/compare/v0.4.12...v0.4.13
[0.4.12]: https://github.com/jerus-org/pcu/compare/v0.4.11...v0.4.12
[0.4.11]: https://github.com/jerus-org/pcu/compare/v0.4.10...v0.4.11
[0.4.10]: https://github.com/jerus-org/pcu/compare/v0.4.9...v0.4.10
[0.4.9]: https://github.com/jerus-org/pcu/compare/v0.4.8...v0.4.9
[0.4.8]: https://github.com/jerus-org/pcu/compare/v0.4.7...v0.4.8
[0.4.7]: https://github.com/jerus-org/pcu/compare/v0.4.6...v0.4.7
[0.4.6]: https://github.com/jerus-org/pcu/compare/v0.4.5...v0.4.6
[0.4.5]: https://github.com/jerus-org/pcu/compare/v0.4.4...v0.4.5
[0.4.4]: https://github.com/jerus-org/pcu/compare/v0.4.3...v0.4.4
[0.4.3]: https://github.com/jerus-org/pcu/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/jerus-org/pcu/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/jerus-org/pcu/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/jerus-org/pcu/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/jerus-org/pcu/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/jerus-org/pcu/compare/v0.1.26...v0.2.0
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
