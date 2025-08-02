#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(rustdoc_missing_doc_code_examples))]
#![cfg_attr(docsrs, warn(rustdoc::invalid_codeblock_attributes))]

//! Gen-bsky
//!

mod to_string_slash;
pub(crate) use to_string_slash::ToStringSlash;

mod front_matter;
pub use front_matter::{FrontMatter, FrontMatterError};

mod draft;
pub use draft::{Draft, DraftError};
