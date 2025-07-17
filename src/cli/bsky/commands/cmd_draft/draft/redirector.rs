use std::fmt;

pub(crate) struct Redirector(pub String);

impl Redirector {
    pub fn new(post_link: &str) -> Self {
        Redirector(post_link.to_string())
    }
}

impl fmt::Display for Redirector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"
<!DOCTYPE HTML>
<html lang="en-US">

<head>
    <meta charset="UTF-8">
    <meta http-equiv="refresh" content="0; url={}">
    <script type="text/javascript">
        window.location.href = "{}";
    </script>
    <title>Page Redirection</title>
</head>

<body>
    <!-- Note: don't tell people to `click` the link, just tell them that it is a link. -->
    If you are not redirected automatically, follow this <a href='{}'>link to example</a>.
</body>

</html>
"#,
            self.0, self.0, self.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redirector_new() {
        let url = "https://example.com";
        let redirector = Redirector::new(url);
        assert_eq!(redirector.0, url);
    }

    #[test]
    fn test_redirector_display() {
        let url = "https://example.com";
        let redirector = Redirector::new(url);
        let output = redirector.to_string();

        assert!(output.contains("<!DOCTYPE HTML>"));
        assert!(output.contains(&format!("url={url}")));
        assert!(output.contains(&format!("window.location.href = \"{url}\";")));
        assert!(output.contains(&format!("<a href='{url}'")));
    }
}
