use std::str::FromStr;

use clap::Subcommand;

#[derive(Debug, Default, Subcommand, Clone)]
pub enum Mode {
    #[default]
    Version,
    Package,
    Workspace,
    Current,
}

impl FromStr for Mode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "version" => Ok(Mode::Version),
            "package" => Ok(Mode::Package),
            "workspace" => Ok(Mode::Workspace),
            "current" => Ok(Mode::Current),
            _ => Err(format!("Invalid release path: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_from_str_version() {
        assert!(matches!(Mode::from_str("version"), Ok(Mode::Version)));
    }

    #[test]
    fn test_mode_from_str_package() {
        assert!(matches!(Mode::from_str("package"), Ok(Mode::Package)));
    }

    #[test]
    fn test_mode_from_str_workspace() {
        assert!(matches!(Mode::from_str("workspace"), Ok(Mode::Workspace)));
    }

    #[test]
    fn test_mode_from_str_current() {
        assert!(matches!(Mode::from_str("current"), Ok(Mode::Current)));
    }

    #[test]
    fn test_mode_from_str_invalid() {
        let result = Mode::from_str("invalid");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid release path: invalid");
    }

    #[test]
    fn test_mode_from_str_empty() {
        let result = Mode::from_str("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid release path: ");
    }

    #[test]
    fn test_mode_default() {
        assert!(matches!(Mode::default(), Mode::Version));
    }
}
