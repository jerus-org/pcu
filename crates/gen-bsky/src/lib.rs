#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(rustdoc_missing_doc_code_examples))]
#![cfg_attr(docsrs, warn(rustdoc::invalid_codeblock_attributes))]

//! # Gen-bsky
//!
//! Drafts and posts bluesky feed posts for a markdown blog files. The details
//! for the posts are generated from the frontmatter metadata in the blog post.
//! To maximize the characters avaiable for post title, description and tags a
//! short-name referrer can be generated and hosted on the same website.
//!
//! Drafting and posting are two seperate steps to allow for the following
//! workflow:
//!
//! 1. Draft the bluesky post when building the website from the markdown files.
//! - Generate the short cut referrer and write to short cut store
//! - Generate the text for the bluesky post and save to a store in the repo.
//! 2. Post the bluesky post when publishing the website
//! - For each post saved in the store post to bluesky
//! - Delete posts that have been succesfully sent
//!
//! ## Draft Example
//!
//! The following sample builds the draft structure and then write the reffer
//! and the bluesky posts. As the referrer has been written when the bluesky
//! post is generated using the shorter link to the referrer.
//! (e.g. https://www.example.com/s/A4t5rb.html instead
//! of https://www.example.com/blog/gen-bsky-release-version-1.3.0/).
//!
//! ```
//!     let mut builder = Draft::builder(base_url);
//!    
//!     // Add the paths specified at the command line.
//!     for path in self.paths.iter() {
//!         builder.add_path_or_file(path)?;
//!     }
//!    
//!     // Set the filters to qualify the blog posts
//!     builder
//!     .with_minimum_date(self.date)?
//!     .with_allow_draft(self.allow_draft);
//!    
//!     let mut posts = builder.build().await?;
//!    
//!     posts.write_referrers(None)?;
//!     posts.write_bluesky_posts(None)?;
//! ```
//!

mod draft;
pub use draft::{Draft, DraftError};
