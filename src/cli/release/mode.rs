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
