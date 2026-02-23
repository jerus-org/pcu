// use std::str::FromStr;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser, Clone)]
pub struct Version {
    /// Specific package to release
    pub version: String,
}

#[derive(Debug, Parser, Clone)]
pub struct Package {
    /// Specific package to release
    pub package: String,
}

#[derive(Debug, Parser, Clone)]
pub struct Current {
    /// Specific package to release
    pub package: Option<String>,
}

/// Check if a specific version is already published to crates.io.
/// Writes SKIP_PUBLISH=true/false to $BASH_ENV.
#[derive(Debug, Parser, Clone)]
pub struct CheckVersionPublished {
    /// Package name on crates.io
    pub package: String,
    /// Version to check (reads $SEMVER or $NEXT_VERSION if not provided)
    #[arg(short, long)]
    pub version: Option<String>,
}

/// Check if the release tag already exists on the remote.
/// Writes SKIP_RELEASE=true/false to $BASH_ENV.
/// Tag is constructed as <package>-v<version>.
#[derive(Debug, Parser, Clone)]
pub struct CheckTag {
    /// Package name (tag constructed as <package>-v<VERSION>)
    pub package: String,
    /// Version (reads $SEMVER or $NEXT_VERSION if not provided)
    #[arg(short, long)]
    pub version: Option<String>,
}

/// Inject a minisign pubkey into Cargo.toml, amend the release commit, and
/// move the signed tag to the amended commit.
#[derive(Debug, Parser, Clone)]
pub struct InjectPubkey {
    /// Package name (locates crates/<package>/Cargo.toml and constructs tag)
    pub package: String,
    /// Version string (reads $SEMVER or $NEXT_VERSION if not provided)
    #[arg(short, long)]
    pub version: Option<String>,
    /// Minisign public key (reads $BINSTALL_SIGNING_PUBKEY if not provided)
    #[arg(long)]
    pub pubkey: Option<String>,
}

/// Upload a binary asset to an existing GitHub release.
#[derive(Debug, Parser, Clone)]
pub struct UploadAsset {
    /// Git tag for the GitHub release
    #[arg(long)]
    pub tag: String,
    /// Path to the asset file to upload
    #[arg(long)]
    pub asset_path: std::path::PathBuf,
    /// Name for the asset in the release (default: filename from asset_path)
    #[arg(long)]
    pub asset_name: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Mode {
    Version(Version),
    Package(Package),
    Workspace,
    Current(Current),
    /// Check if a crate version is already on crates.io
    CheckVersionPublished(CheckVersionPublished),
    /// Check if the release tag already exists on the remote
    CheckTag(CheckTag),
    /// Inject signing pubkey into Cargo.toml and amend the release commit
    InjectPubkey(InjectPubkey),
    /// Upload a binary asset to a GitHub release
    UploadAsset(UploadAsset),
}

// impl FromStr for Mode {
//     type Err = String;
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s {
//             "version" => Ok(Mode::Version),
//             "package" => Ok(Mode::Package(Package::from_str(s)?)),
//             "workspace" => Ok(Mode::Workspace),
//             "current" => Ok(Mode::Current),
//             _ => Err(format!("Invalid release path: {s}")),
//         }
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_mode_from_str_version() {
//         assert!(matches!(Mode::from_str("version"), Ok(Mode::Version)));
//     }

//     #[test]
//     fn test_mode_from_str_package() {
//         assert!(matches!(Mode::from_str("package"), Ok(Mode::Package)));
//     }

//     #[test]
//     fn test_mode_from_str_workspace() {
//         assert!(matches!(Mode::from_str("workspace"), Ok(Mode::Workspace)));
//     }

//     #[test]
//     fn test_mode_from_str_current() {
//         assert!(matches!(Mode::from_str("current"), Ok(Mode::Current)));
//     }

//     #[test]
//     fn test_mode_from_str_invalid() {
//         let result = Mode::from_str("invalid");
//         assert!(result.is_err());
//         assert_eq!(result.unwrap_err(), "Invalid release path: invalid");
//     }

//     #[test]
//     fn test_mode_from_str_empty() {
//         let result = Mode::from_str("");
//         assert!(result.is_err());
//         assert_eq!(result.unwrap_err(), "Invalid release path: ");
//     }

//     #[test]
//     fn test_mode_default() {
//         assert!(matches!(Mode::default(), Mode::Version));
//     }
// }
