use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use expanduser::expanduser;

use pistactl::{cfg::Cfg, cmd, logger, tmux::Tmux};

#[derive(Parser, Debug)]
pub struct Cli {
    /// Path to configuration file
    #[clap(short, long, default_value = concat!("~/.", pistactl::NAME!(), ".toml"))]
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = cli.to_cfg()?;
    logger::init(cfg.debug)?;
    tracing::debug!("cfg: {:#?}", &cfg);
    let mut tmux = Tmux::new(&cfg.sock, &cfg.session);
    match &cli.command {
        Cmd::Status => cmd::status(&cfg, &tmux),
        Cmd::Attach => cmd::attach(&tmux),
        Cmd::Start => cmd::start(&cfg, &mut tmux),
        Cmd::Stop => cmd::stop(&cfg, &tmux),
        Cmd::Restart => cmd::restart(&cfg, &mut tmux),
    }
}
