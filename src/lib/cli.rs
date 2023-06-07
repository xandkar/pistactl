use std::path::PathBuf;

use anyhow::Result;
use expanduser::expanduser;

use crate::cfg::Cfg;

#[derive(clap::Parser, Debug)]
pub struct Cli {
    /// Path to configuration file
    #[clap(short, long, default_value = concat!("~/.", crate::NAME!(), ".toml"))]
    config: String,

    /// Increase logging verbosity
    #[clap(short, long, default_value_t = false)]
    debug: bool,

    /// Name of tmux sock. Corresponds to tmux -L
    #[clap(short = 'L', long)]
    sock_name: Option<String>,

    /// Name of tmux session
    #[clap(short, long)]
    session: Option<String>,

    /// Directory where slot subdirectories with FIFOs will be created
    #[clap(long)]
    dir: Option<PathBuf>,

    #[clap(subcommand)]
    pub command: Cmd,
}

impl Cli {
    pub fn to_cfg(&self) -> Result<Cfg> {
        let cfg_file = expanduser(&self.config)?;
        let mut cfg = Cfg::from_file(&cfg_file)?;
        if self.debug {
            cfg.debug = true;
        }
        if let Some(ref sock) = self.sock_name {
            cfg.sock = sock.clone();
        }
        if let Some(ref session) = self.session {
            cfg.session = session.clone();
        }
        if let Some(ref dir) = self.dir {
            cfg.slots_fifos_dir = dir.clone();
        }
        Ok(cfg)
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
