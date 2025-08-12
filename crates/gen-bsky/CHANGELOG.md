# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.0.1 (2025-08-12)

### Chore

 - <csr-id-30298800431350e9d887c9d5ef70b62ec4d8185f/> remove publish restriction in Cargo.toml
   - remove the publish = false line to allow package publishing
 - <csr-id-b2378a3689201fa704a5260b2699eeb98adc5e0a/> update Cargo.toml for publishing control
   - set publish to false to prevent crate from being published
 - <csr-id-2acbcbb04c8975b0859875cdfb4efb0a7c846c22/> remove unused ToStringSlash trait
   - delete `to_string_slash.rs` as it is no longer needed
   - clean up unused code to improve maintainability
 - <csr-id-bdcdf40d91888ea81e145c743edfb0308b5b866d/> remove unused error module
   - delete error.rs file as it is no longer needed
   - clean up unused error definitions and dependencies
 - <csr-id-9faf38650959d17712f2c393e997d43ad63e58f4/> update cargo package metadata
   - specify license as "MIT or Apache-2.0"
   - update readme file to "README.md"
   - adjust include files to support dual licensing

### Documentation

 - <csr-id-6743fe710499ba8955abf9976089706cfe1798de/> correct typos and update code examples
   - fix several typos in documentation comments
   - update code examples with proper error handling and syntax
 - <csr-id-86b16bd4969c06b95aabf4e17ad5a64353d8e8c1/> update code example with async main function
   - add `tokio::main` attribute for asynchronous execution
   - include use statements for necessary modules
   - provide detailed comments for code clarity
   - ensure consistent error handling with `unwrap`
 - <csr-id-6c2ddc04ba58441eba4566d26af456df02f8f12c/> correct typos and improve clarity
   - fix typos in README.md such as "avaiable" to "available" and "seperate" to "separate"
   - enhance readability by correcting "frontmatter" to "front matter"
   - improve clarity in draft example section by fixing minor spelling errors
 - <csr-id-0039849f4b2b7dc6341a6a04c993a54d8070b5fc/> enhance documentation for gen-bsky module
   - add detailed descriptions for gen-bsky functionality
   - include workflow steps for drafting and posting
   - provide draft example with code snippet
 - <csr-id-45c15078ec583f86ba4a0aae50ad9c6e7d89a40c/> update feature set and add usage example
   - mark "create and save a bluesky post record" as completed
   - add detailed explanation of drafting and posting workflow
   - provide a draft example for better understanding
 - <csr-id-258118d47a34b78beaee08aa8202214f65676458/> update Rust version badge
   - change Rust version badge from 1.81+ to 1.87+ for accuracy
 - <csr-id-48cb3763304358964b0af39f6a701a1737ffcafd/> correct dependency name in Cargo.toml
   - fix hyphenation in unicode-segmentation dependency for consistency with crate names
 - <csr-id-51087d8fd0b17d7a2f240c518c058de36a2bae45/> add README for gen-bsky library
   - introduce gen-bsky library for generating bluesky posts
   - add badges for version, CI, crates.io, documentation, and licenses
   - outline feature set for creating and posting bluesky records
   - provide licensing information under Apache 2.0 or MIT
   - describe contribution guidelines for dual licensing
 - <csr-id-7ec1a44566ce19cc0b471ea412e471c22822fecb/> add MIT license file
   - include MIT license for legal clarity and compliance
 - <csr-id-8a8adc0018c23389d906ac5686fcd5b3450eb563/> add Apache License 2.0 file
   - include LICENSE-APACHE file for legal and distribution purposes
   - define terms for use, reproduction, and distribution of the software

### New Features

 - <csr-id-dd0bcb0f283ebab760eb0bb78974196993698f60/> add trace logging for blog paths discovery
   - log the discovered blog paths for better traceability and debugging
 - <csr-id-c4d74e49206068e580f11c54ef18a0822ee57741/> add www_src_root to draft configuration
   - introduce www_src_root field to Draft struct for enhanced path management
   - update builder method to accept www_src_root parameter
   - adjust path handling to include www_src_root in referrer and bluesky post stores
 - <csr-id-1cb158cb7aaddffa264767f395f5c43ba15d1db0/> add base_url to blog post creation
   - introduce base_url parameter to DraftBuilder methods
   - pass base_url to BlogPost::new for enhanced URL handling
 - <csr-id-2d00a003e6873f7d1cb76af833571a5e91ce6580/> add link module for blog post draft
   - create link.rs to handle link structures and errors
   - implement Link struct with local and public URL handling
   - introduce LinkError enum for URL parsing error management
 - <csr-id-e27505311c727f9fad0cb60cf12a742ed19b4a8d/> add BlogPost struct and error handling
   - introduce new BlogPost struct for managing blog posts
   - add error handling with BlogPostError enum for robust processing
   - implement methods for post creation, validation, and writing operations
 - <csr-id-1b053412726104fb81226fc70ba04b8638dd95c7/> add front matter parsing functionality
   - introduce front matter struct for parsing blog post metadata
   - implement error handling for draft and outdated posts
   - add support for taxonomies and extra sections in front matter
   - include tests for various front matter parsing scenarios
 - <csr-id-0f4c4b414b88117d89f2226b7e9a3f6e819131a8/> add draft allowance option to blog post processing
   - introduce `allow_draft` parameter to control inclusion of draft blog posts
   - modify `add_blog_posts` and `get_front_matters` functions to support draft filtering
   - add warnings for skipped drafts and outdated posts
 - <csr-id-7ec52c819a0c391b47182e54e362023267cb0d16/> add getter method for bluesky field
   - add a getter method `bluesky` to access the bluesky field
   - improve encapsulation by providing a read-only reference to bluesky field
 - <csr-id-bfc71c8df9a633c96497689b5e4edd8bed1ec416/> add ToStringSlash trait implementation
   - introduce ToStringSlash trait for converting strings with trailing slash
   - implement to_string_slash method for &str type to ensure ending slash
 - <csr-id-cbec2716732929683740f51f21cb0cd28cce3c4f/> add new modules for string and draft handling
   - introduce to_string_slash module with ToStringSlash for string operations
   - add draft module with Draft and DraftError for draft management
 - <csr-id-37639d00eead7b48d31c6a7f7378540be9382e06/> add draft management for blog posts
   - implement Draft and DraftBuilder structs to manage blog post drafts
   - provide methods for processing and writing Bluesky posts
   - include error handling for various file and path issues
   - add utility functions for file management and front matter extraction
   
   ✅ test(gen-bsky): add tests for draft management functions
   
   - test path and basename extraction for various file structures
   - validate file retrieval for markdown files in different scenarios
   - include tests for error cases like non-existent paths and invalid file types
 - <csr-id-dafea2eb9202a2930882c753853265e23d7fd311/> add error variants and write bluesky record
   - introduce new error variants for unconstructed posts, unset basename, and unset post link
   - add new error variants for IO and serde_json errors
   - implement `write_bluesky_record` to write bluesky records to specified directory with unique naming convention
 - <csr-id-27fcef638c6412748a60fc613aa88aa908d08439/> enhance error handling and logging
   - introduce `FrontMatterError` enum for detailed error management
   - add comprehensive logging for `get_bluesky_record` function
   
   📝 docs(front_matter): update struct documentation
   
   - provide detailed comments for `FrontMatter` fields
   - enhance function documentation for clarity
 - <csr-id-ee9f26146169fc727f039f30592868cc4903cdb5/> add logging dependencies
   - add `log` to dependencies for enhanced logging capabilities
   - add `env_logger` to dev-dependencies for better local logging configuration
 - <csr-id-907c84e8ba964130bf6e6e710b81e82d15d12c6d/> add taxonomies struct with tag handling
   - introduce Taxonomies struct with tags field
   - implement methods to retrieve tags and convert them to hashtags
 - <csr-id-c379dc6e967aba27dd2937f8f5b1f7316e8227ab/> add extra.rs file for additional metadata
   - create extra.rs file in front_matter module
   - define Extra struct with serde support for deserialization
   - add optional bluesky field to accommodate additional metadata
 - <csr-id-1333741006fe70330d062d9fc706e92ad7391d4a/> add Bluesky struct with serialization
   - introduce Bluesky struct with description and tags fields
   - implement methods to retrieve description, tags, and generate hashtags
   - use serde for deserialization of the front matter data
 - <csr-id-ff04f99c432bdc9835f024c5b463773230fc986d/> add error module
   - introduce a new error module in the gen-bsky crate
   - define an Error enum using the thiserror crate for standardized error handling
 - <csr-id-e25f6886a468d115a96d92d9096b592821db1af9/> add front matter parsing functionality
   - introduce `FrontMatter` struct with methods for TOML parsing
   - implement methods to extract description, tags, and dates
   - add test coverage for TOML parsing and date handling
 - <csr-id-12e966a6be7ead491961539bb418f9142306bd34/> add basic addition function
   - implement add function for u64 integers
   - include basic test to verify functionality
 - <csr-id-89e597e20106b33d5d32872b8d66401e423d33f6/> add initial Cargo.toml for gen-bsky crate
   - create Cargo.toml with basic package metadata
   - set up workspace configurations for common attributes
   - include source files and documentation in package

### Bug Fixes

 - <csr-id-a057a9689be1536914d74a476ddc4da7b9b33666/> add trace logging for directory walk
   - introduce trace level logging to monitor directory traversal
   - enhance debugging by displaying the path being walked
 - <csr-id-41e2fa2f94fe4bfd37aab336405b2fdda6b70319/> handle path stripping error in blog path collection
   - add error handling when stripping prefix from entry paths
   - log warning if stripping fails and continue processing other entries
 - <csr-id-7b026d5815e137990dde27154e95ca1181e4abc9/> correct file path handling in get_files function
   - ensure proper conversion of file paths to string
   - fix tuple formation for markdown files
 - <csr-id-1ed10210c256bfc5f9ffa95faa11587be4fe10f1/> uncomment error variants for better error handling
   - uncomment PostTooManyCharacters and PostTooManyGraphemes for validation
   - uncomment Toml, RedirectorError for improved error reporting

### Other

 - <csr-id-c86395a0f8dc821d93cfeaaf5ea4703304c2c2b0/> add new dependencies to workspace
   - add walkdir and url as workspace dependencies in Cargo.toml
 - <csr-id-216f05ef688ad9778060da2e273e8b843ed234ca/> add new dependencies
   - add chrono to dependencies for date and time handling
   - add tokio and tempfile to dev dependencies for async runtime and temporary file management
 - <csr-id-2c6f76dcf56d069116630f7b265ac46ff94c8bc3/> update dependencies in Cargo.toml
   - add base62 and serde_json as workspace dependencies
   - ensure consistent dependency management within the workspace
 - <csr-id-e021a2ce40d73b90307ca1b3c462d57dc30759c1/> update dependencies in Cargo.toml
   - add workspace dependencies for better package management

### Refactor

 - <csr-id-cebdaa3a80c2c329f01520d2dcf4952739f3ec81/> encapsulate title field
   - remove pub(super) from title field for encapsulation
   - provide title getter method for accessing title field
 - <csr-id-faf39069682d8f07f232e1fae0ffd6ffa2e79d96/> use method for title access
   - change title field access to method call for consistency
   - replace clone with to_string for title conversion
 - <csr-id-b731a1d8713e36b6c766f57404e9eb4bc300f66b/> rename and refactor path handling
   - rename `www_src_root` to `root` for clarity
   - change `builder` function to accept an optional `root` parameter
   - set default root path to current directory if not provided
   - simplify `with_minimum_date` by removing optional parameter handling
 - <csr-id-6893f4c0737a05c25b53f42ec38ba8c81e9f5342/> simplify error handling in path stripping
   - replace match statement with let-else syntax for cleaner error handling
   - use println for error output instead of log::warn
 - <csr-id-017d9bf1b15b6bcd15bd04512ee71192c34baffd/> update directory path handling
   - modify WalkDir path join to use www_src_root for consistency and correctness
 - <csr-id-4f2c40eb9c8270bcb393388cbfef708f2d845203/> modify blog post path handling
   - add www_src_root parameter to construct full blog file path
   - update frontmatter initialization to use the constructed path
 - <csr-id-930de8376027bc89caa1e10b205d06b21027c06e/> rename variable for clarity
   - change variable name from 'blog_path' to 'blog_file' for better understanding of its purpose
   - update debug log and file opening logic accordingly
 - <csr-id-ccf578c438662d07adb58d405ef45630c56966ef/> simplify link handling in BlogPost
   - remove custom Link module and use Url directly for post_link
   - adjust methods to work with Url type for better maintainability
 - <csr-id-6850abc896abc2ce1d734286c0a1ab54e121abac/> remove unused bluesky_post field
   - eliminate the bluesky_post field from BlogPost struct
   - update functions to handle RecordData directly
   
   ✨ feat(blog_post): enhance bluesky record handling
   
   - modify get_bluesky_record to return RecordData
   - update write_bluesky_record_to to asynchronously create and use RecordData
 - <csr-id-be40425c84445600297cda1b92b66a6d93cef9ea/> make write_bluesky_posts asynchronous
   - update write_bluesky_posts function to be async for non-blocking execution
   - remove unnecessary asynchronous get_bluesky_record call in DraftBuilder
 - <csr-id-03f376dac22f6a8b224e89329c43f0f4a9f53ab7/> replace String with Link for post_link
   - change post_link type from String to Link for improved link handling
   - remove referrer_written flag as it's no longer needed with Link type
   
   ✨ feat(blog_post): integrate Link module for URL management
   
   - introduce new Link module to manage blog post URLs
   - add base_url parameter to BlogPost constructor for Link initialization
 - <csr-id-05e09836f4fe0256f6a1b43079b18174ba8aedd8/> update visibility and structure
   - change structs and methods to use `pub(crate)` for internal visibility
   - rename files and update directory structure for better organization
 - <csr-id-487ad377d9a6399d3ab631aa033165bd5255954d/> simplify and improve draft structure
   - replace `FrontMatter` with `BlogPost` for blog post handling
   - refactor `DraftBuilder` to take `Url` for base_url
   - remove redundant code for file handling and front matter processing
   - streamline directory handling with `walkdir` for markdown files
   
   ✅ test(draft): remove outdated tests for file handling
   
   - eliminate tests related to removed `get_files` and `get_front_matters` functions
   - update structure to align with new draft logic
   - ensure tests reflect refactored components and logic changes
 - <csr-id-3d184be565ae65b6987c488f9bc4c0fa9ea85362/> improve path handling in DraftBuilder
   - replace blog_posts with path_or_file to allow more flexible input
   - update add_blog_posts method to process PathBuf and filter by date
   - modify get_files to accept &PathBuf instead of &str for consistency
   
   🐛 fix(draft): correct path handling in get_files function
   
   - ensure get_files handles PathBuf correctly across all uses
   - update test cases to pass PathBuf instead of strings to get_files
 - <csr-id-53fc49774054412fa26923ff3e03d5b5baae1021/> update tag handling with method call
   - replace direct tag access with tags() method for improved encapsulation
   - update test cases to use tags() method for consistency
   - refactor Taxonomies initialization to use new constructor method
 - <csr-id-eeb77c4bf9b515b8966b94d512c2077345c360c8/> improve tag handling in Taxonomies struct
   - remove public visibility from tags field for encapsulation
   - add new constructor method for creating Taxonomies instances
   - modify tags method to return a reference to the tags vector
 - <csr-id-fc9a43d9669ed3f49c5f71ec714b19ad828ad8ef/> simplify bluesky access
   - replace direct field access with method calls for bluesky
   - improve code readability and maintainability by reducing repetitive unwraps
 - <csr-id-52002e65ec6a5166b44b94de2ac76a6494ff8566/> update method calls for bluesky
   - replace direct field access with method calls for description and tags
   - ensure consistency in accessing optional fields in tests
 - <csr-id-c3c2c09b935fa748545040831d0192cb74960ec3/> update field access in Bluesky struct
   - change `description` and `tags` fields to private
   - update `tags` method to be public
 - <csr-id-98fbc14ed54364d621720a4d704e235da7a68371/> re-enable and update link building
   - uncomment `Redirector` import and its usage for link handling
   - change `get_links` method to handle file paths and update logic
   - simplify `build_post_text` by removing unused parameter
   - retain detailed debug logging for tracing link building steps
 - <csr-id-af0be9bbefab52925cb84a81bb448a34b0b7e717/> simplify file handling with PathBuf
   - change file handling logic to use PathBuf for improved readability and simplicity
   - remove redundant functions for path and basename extraction
   - update function signatures and tests to reflect changes in file handling logic
 - <csr-id-eb7f9e0d982337ec3162f9790a872c1ebaea58cf/> update path handling and remove basename usage
   - replace `basename` with `path` using `PathBuf` for more robust path handling
   - remove unused `basename` field and its related logic
   - comment out `get_links` function as it's not currently in use
   
   ✅ test(front_matter): adjust tests to reflect path handling changes
   
   - remove test for `basename` and `path` as separate fields
   - update tests to validate new `PathBuf` usage for `path`
 - <csr-id-5c0a4aba417283e1789e0c719eefc5b1749e31ca/> rename function for clarity
   - change function name from write_bluesky_record to write_bluesky_record_to
   - improve readability and clarity of function purpose
 - <csr-id-94a81476c225d9f08fb536c43957a9554412dabe/> streamline error handling
   - remove separate error module and integrate FrontMatterError within front_matter module for consolidated error management
 - <csr-id-88892b06d7d9b7a84425199922a8a4afe38d1efb/> restructure gen-bsky module
   - remove unused add function and associated tests
   - add documentation and linting attributes
   - introduce error and front_matter modules
   - re-export Error and FrontMatter for external use

### Style

 - <csr-id-2c0ea3dadf56d8d502bb082f17c27b3a1129ef2d/> correct license format in Cargo.toml
   - change "MIT or Apache-2.0" to "MIT OR Apache-2.0" for consistency with SPDX format
 - <csr-id-d55f8d977e6f2d57e4078ea97a830bcbc57b5d7c/> update copyright symbol
   - replace (c) with © for copyright notice consistency

### Test

 - <csr-id-f4afabff2f362fc87f19ea71c378abe40942e273/> add test for incorrect bluesky field
   - add a new test to check for incorrect data types in the bluesky field
   - ensure error handling works as expected when bluesky field is not structured correctly
   
   💄 style(front_matter): improve logging configuration in tests
   
   - add #[cfg(test)] to logging calls for test-specific logging control

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 90 commits contributed to the release over the course of 12 calendar days.
 - 72 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

