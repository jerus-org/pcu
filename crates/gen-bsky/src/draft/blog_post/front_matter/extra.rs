use serde::Deserialize;

#[derive(Default, Debug, Clone, Deserialize)]
pub(crate) struct Extra {
    #[allow(dead_code)]
    bluesky: Option<super::Bluesky>,
}

impl Extra {
    pub(crate) fn bluesky(&self) -> Option<&super::Bluesky> {
        self.bluesky.as_ref()
    }
}
