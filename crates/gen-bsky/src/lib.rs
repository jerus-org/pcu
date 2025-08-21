#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(rustdoc_missing_doc_code_examples))]
#![cfg_attr(docsrs, warn(rustdoc::invalid_codeblock_attributes))]

//! # Gen-bsky - Bluesky Blog Post Generator
#![doc = include_str!("../docs/lib.md")]

/// Default bluesky post directory.
pub const BSKY_DIR: &str = "bluesky";
/// Default referrer directory.
pub const REFERRER_DIR: [&str; 2] = ["static", "s"];
/// Default blog directory.
pub const BLOG_DIR: [&str; 2] = ["content", "blog"];

mod draft;
pub use draft::{Draft, DraftError};
mod post;
pub use post::{Post, PostError};
mod util;

mod capitalise;
pub(crate) use capitalise::Capitalise;
