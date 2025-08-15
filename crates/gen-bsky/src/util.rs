use std::path::{Path, PathBuf};

use chrono::Utc;

pub(crate) fn random_name() -> String {
    let now = Utc::now();
    let millis = now.timestamp_millis();

    base62::encode(millis as u128)
}

#[allow(dead_code)]
fn walk_directory_for_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    log::debug!("Walking directory from path: {path:?}");

    if path.is_file() {
        log::debug!("Path is a file");
        files.push(PathBuf::from(path));
    } else if path.is_dir() {
        log::debug!("Path `{path:?}` is a directory");
        for entry in path.read_dir().expect("read_dir call failed").flatten() {
            if entry.path().is_dir() {
                let additional_files = walk_directory_for_files(entry.path().as_path());
                files.extend(additional_files);
            } else if entry.path().is_file() {
                files.push(entry.path())
            }
        }
    } else {
        log::error!(
            "Path is neither a file nor a directory: {:?}",
            path.components()
        )
    }

    files
}
