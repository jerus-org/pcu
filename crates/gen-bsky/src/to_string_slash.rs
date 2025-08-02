pub(crate) trait ToStringSlash {
    fn to_string_slash(&self) -> String;
}

impl ToStringSlash for &str {
    fn to_string_slash(&self) -> String {
        let mut new_string = self.to_string();
        if !new_string.ends_with('/') {
            new_string.push('/')
        }
        new_string
    }
}
