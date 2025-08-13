use std::path::PathBuf;

use bsky_sdk::api::app::bsky::feed::post::RecordData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BskyPostState {
    Read,
    Posted,
    Deleted,
}

#[derive(Debug, Clone)]
pub(crate) struct BskyPost {
    post: RecordData,
    file_path: PathBuf,
    state: BskyPostState,
}

impl BskyPost {
    pub(crate) fn new(post: RecordData, file_path: PathBuf) -> Self {
        BskyPost {
            post,
            file_path,
            state: BskyPostState::Read,
        }
    }

    pub(crate) fn post(&self) -> &RecordData {
        &self.post
    }

    pub(crate) fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    pub(crate) fn state(&self) -> &BskyPostState {
        &self.state
    }

    pub(crate) fn set_state(&mut self, new_state: BskyPostState) {
        self.state = new_state
    }
}
