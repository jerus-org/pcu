#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]
#![warn(clippy::all, rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

//! gen-linkedin: Minimal LinkedIn API client for CI usage
//!
//! This crate exposes a small surface area focused on creating text posts for
//! release announcements. HTTP and auth details are encapsulated so callers only
//! need to provide an access token and author URN.

/// Token providers for bearer tokens used to authenticate with LinkedIn.
pub mod auth;
/// Base HTTP client wrapper shared by feature modules.
pub mod client;
/// Error types returned by this crate.
pub mod error;
/// LinkedIn Posts API (REST) support.
#[cfg(feature = "posts")]
pub mod posts;

pub use crate::error::Error;
