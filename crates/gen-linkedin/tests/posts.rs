use gen_linkedin::{
    auth::StaticTokenProvider,
    client::Client,
    posts::{PostsClient, TextPost},
};
use url::Url;

#[tokio::test]
async fn create_text_post_sends_expected_request() {
    let server = httpmock::MockServer::start_async().await;

    let m = server.mock(|when, then| {
        when.method("POST")
            .path("/rest/posts")
            .header("authorization", "Bearer TOKEN")
            .header("x-restli-protocol-version", "2.0.0")
            .header("content-type", "application/json");
        then.status(201)
            .header("x-restli-id", "urn:li:activity:123")
            .body("");
    });

    let token = StaticTokenProvider("TOKEN".to_string());
    let client = Client::new(token)
        .unwrap()
        .with_base(Url::parse(&server.base_url()).unwrap());
    let posts = PostsClient::new(client);

    let resp = posts
        .create_text_post(&TextPost::new("urn:li:person:abc", "Hello LinkedIn"))
        .await
        .unwrap();
    assert_eq!(resp.id, "urn:li:activity:123");

    m.assert();
}
