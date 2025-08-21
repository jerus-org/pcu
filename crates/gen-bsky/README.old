# Gen-bsky

A library to generate a bluesky feed post from the front matter metadata in markdown source file for a web document.

[![Rust 1.87+][version-badge]][version-url]
[![circleci-badge]][circleci-url]
[![Crates.io][crates-badge]][crates-url]
[![Docs][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]
[![APACHE licensed][apache-badge]][apache-url]
[![BuyMeaCoffee][bmac-badge]][bmac-url]
[![GitHubSponsors][ghub-badge]][ghub-url]

[crates-badge]: https://img.shields.io/crates/v/gen-bsky.svg
[crates-url]: https://crates.io/crates/gen-bsky
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/jerusdp/gen-bsky/blob/main/LICENSE-MIT
[apache-badge]: https://img.shields.io/badge/license-APACHE-blue.svg
[apache-url]: https://github.com/jerusdp/gen-bsky/blob/main/LICENSE-MIT
[circleci-badge]: https://dl.circleci.com/status-badge/img/gh/jerus-org/pcu/tree/main.svg?style=svg
[circleci-url]: https://dl.circleci.com/status-badge/redirect/gh/jerus-org/pcu/tree/main
[version-badge]: https://img.shields.io/badge/rust-1.81+-orange.svg
[version-url]: https://www.rust-lang.org
[docs-badge]:  https://docs.rs/gen-bsky/badge.svg
[docs-url]:  https://docs.rs/gen-bsky
[bmac-badge]: https://badgen.net/badge/icon/buymeacoffee?color=yellow&icon=buymeacoffee&label
[bmac-url]: https://buymeacoffee.com/jerusdp
[ghub-badge]: https://img.shields.io/badge/sponsor-30363D?logo=GitHub-Sponsors&logoColor=#white
[ghub-url]: https://github.com/sponsors/jerusdp

## Feature set

- [X] Create and save a bluesky post record
- [X] Login and post the record to a bluesky account
Automatically creates and publishes Bluesky posts for your markdown blog
articles using front matter metadata. This tool maximizes character usage by
generating short URLs, leaving more space for compelling post content.

## Two-Step Workflow

The process separates drafting from publishing to integrate seamlessly with
your website build and deployment pipeline:

### 1. Draft Phase (During Website Build)

When processing your markdown blog files:
- **Generate short URLs**: Creates compact referrer links and saves them to
  your short URL store
- **Compose posts**: Extracts metadata from front matter to craft Bluesky
  post text
- **Queue for publishing**: Saves draft posts to a repository store for
  later posting

### 2. Publishing Phase (During Website Deployment)

When your website goes live:
- **Batch publish**: Posts all queued drafts to Bluesky
- **Clean up**: Removes successfully posted drafts from the store

## Preparing Blog Post Front Matter

Bluesky posts are automatically generated from blog post metadata including:
- Title
- Description  
- Tags
- Fully qualified URL or short link (if available)

### Character Limits

Bluesky posts are limited to 300 characters (graphemes). If the generated
post exceeds this limit, an error will be logged as a warning and processing
will continue without posting to Bluesky.

### Custom Bluesky Content

When the default title, description, and tags produce a post that's too long,
you can override the content by adding a `[bluesky]` section to your front matter:

```toml
[bluesky]
description = "Custom shorter description for Bluesky"
tags = ["rust", "web"]
```

**Note:** The post title and link URL cannot be customized and will always use
the original blog post values.

### Example front matter with bluesky section:

```
+++
title = "Used as the header in the Bluesky blog post"
description = """
The descriptions of the blog post can be as necessary for the \
publication of the website and as long as you need it to be. \
The goals of publication on the website should be primary \
ones driving the composition of this element. The description \
will be used for the text on the Bluesky blog post and if it \
is very long result in a Bluesky post that exceeds the size \
allowed by the protocol."""
[taxonomies]
tags = ["Tags",
"Should be",
"Generated as",
"Appropriate to the ",
"Requirements of ",
"The post on",
"The website",
"And can be",
"As extensive as",
"Required.",
"They will be",
"Converted to",
"Hashtags"
"And contribute to"
"The size of the post."
]
[bluesky]
description = """\
This description will be preferred allowing an edited version \
of the description to ensure the Bluesky post can be kept \
within the limits of the protocol."""
tags = ["Likewise",
"this tags section",
"will be preferred"]
+++
```

## Benefits of Short URLs

By generating compact referrer URLs (like `https://www.example.com/s/A4t5rb.html`
instead of `https://www.example.com/blog/gen-bsky-release-version-1.3.0/`), you gain
valuable characters for:
- Engaging post titles
- Descriptive content summaries
- Relevant hashtags and mentions

## Draft Example

The following example demonstrates the complete drafting workflow—from
building the post structure to generating both the short URL referrer and
the final Bluesky post content.

```rust should_panic
# use std::path::PathBuf;
#
# use url::Url;
# use toml::value::Datetime;
#
# use gen_bsky::{Draft, DraftError};
#
# #[tokio::main]
# async fn main() -> Result<(), DraftError> {
    let base_url = Url::parse("https://www.example.com/")?;
    let paths = vec!["content/blog".to_string()];
    let date = Datetime {
                  date: Some(toml::value::Date{
                              year: 2025,
                              month: 8,
                              day: 4}),
                  time: None,
                  offset: None};
    let allow_draft = false;
    let mut posts = get_post_drafts(
                        base_url,
                        paths,
                        date,
                        allow_draft).await?;
   
    posts.write_referrers(None)?;
    posts.write_bluesky_posts(None).await?;
    Ok(())
 }
 async fn get_post_drafts(
            base_url: Url,
            paths: Vec<String>,
            date: Datetime,
            allow_draft: bool) -> Result<Draft, DraftError>
{
    let post_store = PathBuf::new().join("bluesky_post_store");
    let referrer_store = PathBuf::new().join("static").join("s");
    let mut builder = Draft::builder(base_url, None);
   
    // Add the paths specified at the command line.
    for path in paths.iter() {
        builder.add_path_or_file(path)?;
    }
   
    // Set the filters for blog posts
    builder
    .with_minimum_date(date)?
    .with_allow_draft(allow_draft);
    // Create the `Draft` structure to write the files
    let mut drafter = builder.build().await?;
    drafter.write_referrers(Some(referrer_store))?
           .write_bluesky_posts(Some(post_store))?;
 }
```

## Post Processing Example

The post files generated in the previous example are processed through the
following workflow:
1. **Read**: Retrieve posts from the local store
2. **Publish**: Submit each post to the Bluesky account using the provided
  credentials (ID and password)
3. **Clean up**: Remove successfully published posts from the store
Posts that fail to publish remain in the store for retry or manual review.

```rust should_panic
# use gen_bsky::{Post, PostError};
#
# #[tokio::main]
# async fn main() -> Result<(), PostError> {
    let id = "bluesky_identifier";
    let pw = "bluesky_password";
    let store = "bluesky_post_store";
    let mut poster = Post::new(id, pw)?;
    let deleted = poster
        .load(store)?
        .post_to_bluesky()
        .await?
        .delete_posted_posts()?
        .count_deleted();
    println!("{deleted} post sent to bluesky and deleted from the {store}");
    Ok(())
# }   
```
## License

 Licensed under either of

- Apache License, Version 2.0 (LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license (LICENSE-MIT or <http://opensource.org/licenses/MIT>)
 at your option.

## Contribution

 Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
