use anyhow::Result;
use expanduser::expanduser;

use crate::cfg::Cfg;

#[derive(clap::Parser, Debug)]
pub struct Cli {
    /// Path to configuration file
    #[clap(short, long, default_value = concat!("~/.", crate::NAME!(), ".toml"))]
    config: String,

    #[clap(subcommand)]
    pub command: Cmd,
}

impl Cli {
    pub fn to_cfg(&self) -> Result<Cfg> {
        let cfg_file = expanduser(&self.config)?;
        Cfg::from_file(&cfg_file)
    }
}

#[derive(clap::Subcommand, Debug)]
pub enum Cmd {
    Status,
    Start,
    Stop,
    Restart,
    Attach,
}

/// Aborts the process on errors!
pub fn parse() -> Cli {
    use clap::Parser;
    Cli::parse()
}
