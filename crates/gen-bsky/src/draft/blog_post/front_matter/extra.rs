use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Extra {
    #[allow(dead_code)]
    bluesky: Option<super::Bluesky>,
}

impl Extra {
    pub(crate) fn bluesky(&self) -> Option<&super::Bluesky> {
        self.bluesky.as_ref()
    }
}
