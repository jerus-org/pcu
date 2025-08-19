pub(crate) fn hashtags(tags: Vec<String>) -> Vec<String> {
    let mut hashtags = vec![];
    for tag in tags.into_iter() {
        // convert tag to hashtag by capitalising the first letter of each word,
        // removing the spaces and prefixing with a # if required
        let formatted_tag = tag
            .trim_start_matches('#')
            .split_whitespace()
            .map(|mut word| word.capitalize())
            .collect::<String>();
        let hashtag = format!("#{formatted_tag}");
        hashtags.push(hashtag);
    }

    hashtags
}

trait Capitalize {
    fn capitalize(&mut self) -> String;
}

impl Capitalize for &str {
    /// Capitalizes the first character in s.
    fn capitalize(&mut self) -> String {
        let mut c = self.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }
}
