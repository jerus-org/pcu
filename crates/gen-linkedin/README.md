# gen-linkedin

Minimal LinkedIn API client for CI pipelines. Focuses on creating simple text posts for release announcements.

- Default feature: `posts` (new REST Posts API)
- Optional features: `ugc` (legacy v2 UGC Posts), `oauth2` (helpers for interactive flows)

## Usage (CI)
Provide a bearer token via env var and create a post:

```rust
use gen_linkedin::{auth::EnvTokenProvider, client::Client};
#[cfg(feature = "posts")] use gen_linkedin::posts::{PostsClient, TextPost};

# async fn demo() -> Result<(), gen_linkedin::Error> {
let token = EnvTokenProvider { var: "LINKEDIN_ACCESS_TOKEN".into() };
let li = Client::new(token)?;
let posts = PostsClient::new(li);
let post = TextPost::new("urn:li:person:...", "Hello LinkedIn!");
let _resp = posts.create_text_post(&post).await?;
# Ok(())
# }
```

## Security
- Never log tokens; store in your CI secret manager.
- Uses rustls; no OpenSSL required.
