#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]
#![warn(clippy::all, rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

//! gen-linkedin: LinkedIn draft generation and post publishing for CI usage.
//!
//! This crate provides two main capabilities:
//!
//! 1. **Draft generation** — scan Zola/Hugo blog posts for a `[linkedin]`
//!    frontmatter section, build staged `.linkedin` JSON files, and stamp the
//!    source frontmatter with a creation date (idempotent).
//!
//! 2. **Post publishing** — read staged `.linkedin` files, publish each as a
//!    LinkedIn text post via the Posts API, write a `published` date back into
//!    the source frontmatter, and delete the draft file.
//!
//! # Frontmatter convention
//!
//! A blog post opts in by adding a `[linkedin]` section to its TOML frontmatter:
//!
//! ```toml
//! [linkedin]
//! description_file = "linkedin.md"   # preferred for long posts
//! # description = "Short inline text is also accepted."
//! ```
//!
//! The post text must be **author-written** and is not derived from any other
//! frontmatter field. LinkedIn posts are validated against
//! [`LINKEDIN_POST_MAX_CHARS`] (including the appended post link).
//!
//! # Authentication
//!
//! Callers supply a LinkedIn access token and author URN directly. Token
//! acquisition (OAuth 2.0) is outside the scope of this crate. See
//! [`auth::StaticTokenProvider`] for the simple bearer-token provider used
//! in CI contexts.

/// Token providers for bearer tokens used to authenticate with LinkedIn.
pub mod auth;
/// Base HTTP client wrapper shared by feature modules.
pub mod client;
/// LinkedIn draft generation from blog post frontmatter.
pub mod draft;
/// Error types returned by this crate.
pub mod error;
/// Serde-based frontmatter types for the `[linkedin]` TOML section.
pub mod frontmatter;
mod frontmatter_writeback;
/// LinkedIn post publishing from staged draft files.
pub mod post;
/// LinkedIn Posts API (REST) support.
#[cfg(feature = "posts")]
pub mod posts;

pub use crate::draft::{Draft, DraftError, LinkedinFile, LINKEDIN_POST_MAX_CHARS};
pub use crate::error::Error;
pub use crate::frontmatter::{FrontMatter, LinkedIn};
pub use crate::post::{Post, PostError};
