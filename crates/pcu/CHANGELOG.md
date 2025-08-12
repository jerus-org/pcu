# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.5.0 (2025-08-12)

### Chore

 - <csr-id-cb6618a87fb77f52884e2d5fe10cfc927711cc8f/> reorganize file structure for bsky content
   - rename and relocate config.toml for better organization
   - move blog files to content/blog directory for improved clarity
 - <csr-id-44aa903e21100727580ebea49b2e0d7e2998801f/> add logging for draft command execution
   - log initial posts state after build
   - log state after writing referrers and bluesky posts
 - <csr-id-6256500cf86cf51db1e7b7ce6022cfb51cac32c9/> update license configuration
   - change license to "MIT OR Apache-2.0" for dual licensing
   - remove LICENSE file and update include paths for new license files
 - <csr-id-b3ffd75bf2507b84d86fd9d70486410b5819b697/> remove draft and site_config modules
   - delete draft.rs and site_config.rs to clean up unused code
   - remove Draft struct and related functionality
   - eliminate SiteConfig and configuration handling
 - <csr-id-5e846eee41a77c82822933b6bcdecc3e1530b2f8/> remove unused front matter files
   - delete front_matter.rs and its related modules (bluesky.rs, extra.rs, taxonomies.rs)
   - cleanup codebase by removing unused front matter handling code
 - <csr-id-eb973366638c3b79a358e656d7f85b0001bf3507/> add gen-bsky to workspace dependencies
   - include gen-bsky in the workspace for consistent dependency management across crates
 - <csr-id-291f80bad9dc7a5343e176ad3535eee299b46c5e/> restructure project directory layout
   - move source files from 'src' and 'tests' to 'crates/pcu' directory
   - update project structure for improved modularity and organization
 - <csr-id-4ae1a9f0d60d5fd4f012e76d2d9f903bbf9b9183/> reorganize README file location
   - move README.md to crates/pcu directory for better project structure
 - <csr-id-2a9d677898cb19e80efcdabb9aeffa9f7594115f/> relocate LICENSE file
   - move LICENSE file to crates/pcu directory for better organization

### Documentation

 - <csr-id-f3a0dae411adab71559195561d84fa6a3ff29501/> add description to update and release series
   - add a description field to enhance blog post metadata and improve SEO
 - <csr-id-3ffee146a1f09878d40987907331a525802d6794/> remove "Stuck in the middle" article
   - delete blog post "Stuck in the middle" from the repository
   - the article discussed geopolitical themes and historical parallels
   - content no longer relevant or needed in the current context
 - <csr-id-65a42fc30bb5746633291bd67674af7bcdfc8134/> add Apache and MIT licenses
   - include Apache License 2.0 text for licensing terms
   - add MIT License text for dual licensing options
 - <csr-id-836e01512055a06ffdd4c0549e12439808f71983/> update project title casing
   - change project title from lowercase to capitalized format
 - <csr-id-c6e97e0e81c17f88184bb6129ef87e36d6d6bacd/> update Rust version badge
   - change Rust version badge from 1.81+ to 1.87+ to reflect updated requirements

### New Features

 - <csr-id-e5573393486c48c7c7ff3182ae8ed787d2515b79/> enhance site config path handling
   - add path::Path import for improved path management
   - update SiteConfig::new to accept www_src_root and optional filename
   - log errors with dynamic file path for better debugging
 - <csr-id-64a4bed7557956d7e42021fddfb3e48379aaddac/> add www_src_root parameter to CmdDraft
   - introduce www_src_root parameter for specifying website source root folder
   - update log trace to include www_src_root
   - modify Draft builder to accept www_src_root as an argument
 - <csr-id-79ec58e49a3571a1a1362e1faea1d39cf1b7d9d4/> support multiple paths for CmdDraft
   - change path from Option<String> to Vec<PathBuf> to handle multiple paths
   - update run method to process each path in paths vector
   - ensure DEFAULT_PATH is included when no paths are provided
 - <csr-id-7078da88bba282a8b93d2fe355ed00f54baed1be/> enable serde feature for url crate
   - add serde feature to url crate in Cargo.toml for enhanced serialization support
 - <csr-id-d5cf7697c798b0f5e028f8ea98d147c550487208/> add logging filter for gen_bsky module
   - include gen_bsky module in logging filters
   - ensure consistent logging level configuration across modules
 - <csr-id-86440622331b6ac10debee48dc084fa2791d1a18/> add trace logging for key parameters in cmd_draft
   - introduce trace logging to output key parameters such as base_url, store, and path for improved debugging and transparency
 - <csr-id-7c38aed8bdc4f671256b50fa7ae1095974f94113/> add site_config management in bsky draft command
   - create site_config.rs for site configuration management
   - implement SiteConfig struct with base_url field
   - add new() method to read and parse config.toml file
   - provide base_url() method to access base_url value
 - <csr-id-c9311bc9da88c39c1a0ad9db3c3b1651b161c15a/> add new error variant for front matter
   - introduce FrontMatterError to handle errors from gen-bsky
   - improve error handling by expanding error enum
 - <csr-id-32b0267d9e40aac9a3a1e8007ca0ef99a53d9a5c/> add Cargo.toml for pcu crate
   - create Cargo.toml file for pcu crate
   - define package metadata and dependencies
   - set up bin and lib paths for the crate

### Bug Fixes

 - <csr-id-a861ea3780009a52f713e2a38f53d77f83e0e775/> handle file read errors gracefully
   - add error handling for reading config.toml
   - log error message when file reading fails
 - <csr-id-c762ea65d4e27088096d6dce928c11b41404aa4e/> add await for async bluesky post writing
   - ensure asynchronous execution by adding await to write_bluesky_posts
   - fix potential runtime issue by properly handling async function
 - <csr-id-730b37bc2b4137a6632779687bce0693936a2215/> correct path handling in cmd_draft
   - fix condition to correctly handle empty paths scenario
   - ensure default path is added when no paths are provided
 - <csr-id-453d1fd385e862e3fdef1d4ed6e5d9d81de67fd6/> fix borrow issues in command execution
   - make draft_args mutable to resolve borrow checker issues with the run method

### Other

 - <csr-id-83f923d58dcf91cf5fa32f85d95832abd11e9994/> update gen-bsky dependency configuration
   - specify version "0.0.0" for gen-bsky dependency
   - ensure consistent dependency management within workspace

### Refactor

 - <csr-id-0a474b2a952b142107015d43aa19b863306c6bdb/> enhance draft command flexibility
   - modify Draft::builder to accept an optional root path
   - make minimum date filter conditional based on provided date
 - <csr-id-8ebbab8bc12ece4eee314ab24cf36f3da1106587/> update site configuration initialization
   - modify `SiteConfig::new` to accept `www_src_root` and optional parameters for flexibility
 - <csr-id-9c8b7b709545bfd2d9e775aee94ec9255e75ce98/> simplify error handling in post creation
   - use `if let` syntax for concise error checking and logging
 - <csr-id-d9854c789811a3d36393a31415e0a1edd00d0639/> use Url type for base_url
   - change base_url field type from String to Url for better URL handling
   - update base_url method return type to Url for consistency
 - <csr-id-9cdbffdb488d78959be12e578180a831d1227f1d/> remove unused FrontMatterError variant
   - eliminate FrontMatterError from Error enum
   - improve code maintainability by cleaning up unused variants
 - <csr-id-9cfea848606607e78e3b18b770dd07bb792055f3/> update method for path handling in draft command
   - rename method from add_blog_posts to add_path_or_file for clarity and flexibility
 - <csr-id-90523fec790e734855ec64d1421ac6e00b06ba81/> enhance draft processing and configuration
   - replace module `draft` with `site_config` for improved configuration handling
   - remove unused imports and streamline code
   - integrate `gen_bsky::Draft` for better post management
   - optimize blog post processing with new `Draft` builder pattern
   
   ✅ test(cli): remove redundant tests and update for new logic
   
   - remove tests related to old `draft` module
   - update tests to align with new `Draft` processing logic
 - <csr-id-0abe468f237b0eb7d341b566f9e6c1779419e166/> update error enum definitions
   - remove unused FutureCapacityTooLarge error variant
   - update FrontMatterError message for clarity
   - add new DraftError variant for better error handling
 - <csr-id-f0b167d2d2ff6a8ea43ab502d072b63b113718ae/> simplify bluesky record writing process
   - remove manual file creation and serialization code
   - use write_bluesky_record method to handle file operations
   - improve error handling and logging for clearer diagnostics
 - <csr-id-5d450a3ecbd4b7184fcc9d8aa012b4b6fa6e7d36/> improve post processing logic
   - remove inline record data creation for cleaner code
   - utilize get_bluesky_record method for post processing
   
   🐛 fix(logging): correct log messages for file operations
   
   - update logging from "Post filename" to "Write filename"
   - change "Post file" to "Write file" for clarity
 - <csr-id-c1d0106fa0ecba35d6ba4efd579f5d5c9c2a601b/> update front matter module import
   - remove direct import of front_matter module
   - use gen_bsky for importing FrontMatter

### Style

 - <csr-id-aee56b3d229f3fc2f716415dee4531c75b4a3b4c/> update metadata field name
   - change 'bluesky' to 'bluesky.description' for clarity and consistency
 - <csr-id-52d7fa3b5a9166a21685e065cbd702c0811f966f/> correct logging format in cmd_draft
   - adjust parameter labels to maintain consistent case and clarity
   - replace duplicate "Base" with "root" for accurate description
 - <csr-id-1854e3a7225138ee79c21dcb539c083d41a8b2d4/> remove obsolete comments
   - delete outdated TODO comments related to blog processing and Bluesky posting
 - <csr-id-1f691e1a0eba1266c2aba5e2fff73f21bde090c5/> add hidden false positives for ltex
   - update ltex.hiddenFalsePositives.en-GB.txt with new rules for better false positive management
   
   💄 style(blog): fix typos and improve clarity in blog posts
   
   - correct spelling and punctuation in influences-jan-2025.md and influences-original.md
   - improve sentence clarity in introduction.md
 - <csr-id-8a1007e21a8d12c408dcd7cf8af930a5e2b77551/> fix typo in blog content
   - correct "by" to "be" for grammatical accuracy
   - add apostrophe in "it's" for proper contraction usage
 - <csr-id-96114728db66eca78afc49d3e797bee26d981df5/> update file path in log messages
   - add './' prefix to config.toml path to clarify file location in log messages
 - <csr-id-95466ab2763b626b08edea91f858302842a6bec2/> simplify log message format
   - remove redundant 'url' from log message for base_url parameter

### Chore (BREAKING)

 - <csr-id-15bc8782413641a435cc6af8102cc874b0d75fcf/> simplify test configuration file
   - remove unnecessary configuration settings for streamlined testing
   - retain only essential base_url setting in config.toml

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 62 commits contributed to the release over the course of 12 calendar days.
 - 47 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

