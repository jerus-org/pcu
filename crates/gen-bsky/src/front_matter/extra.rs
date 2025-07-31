use serde::Deserialize;

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Extra {
    #[allow(dead_code)]
    pub bluesky: Option<super::Bluesky>,
}
