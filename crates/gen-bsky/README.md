# Gen-bsky

A library to generate a bluesky post.

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

- [x] Create and save a bluesky post record
- [ ] Login and post the record to a bluesky account

Drafts and posts bluesky feed posts for a markdown blog files. The details
for the posts are generated from the frontmatter metadata in the blog post.
To maximize the characters avaiable for post title, description and tags a
short-name referrer can be generated and hosted on the same website.
Drafting and posting are two seperate steps to allow for the following
workflow:
1. Draft the bluesky post when building the website from the markdown files.
- Generate the short cut referrer and write to short cut store
- Generate the text for the bluesky post and save to a store in the repo.
2. Post the bluesky post when publishing the website
- For each post saved in the store post to bluesky
- Delete posts that have been succesfully sent
## Draft Example
The following sample builds the draft structure and then write the reffer
and the bluesky posts. As the referrer has been written when the bluesky
post is generated using the shorter link to the referrer.
(e.g. https://www.example.com/s/A4t5rb.html instead
of https://www.example.com/blog/gen-bsky-release-version-1.3.0/).
```
   let mut builder = Draft::builder(base_url);
   
   // Add the path to the markdown files 
    builder.add_path_or_file("content/blog")?;
    
    // Set the filters to qualify the blog posts
    builder
        .with_minimum_date(self.date)?
        .with_allow_draft(self.allow_draft);
   
    let mut posts = builder.build().await?;
   
    posts.write_referrers(None)?;
    posts.write_bluesky_posts(None)?;
```

## License

 Licensed under either of

- Apache License, Version 2.0 (LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license (LICENSE-MIT or <http://opensource.org/licenses/MIT>)
 at your option.

## Contribution

 Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
