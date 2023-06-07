use anyhow::Result;

use pistactl::{cli, cmd, logger, tmux::Tmux};

fn main() -> Result<()> {
    let cli = cli::parse();
    let cfg = cli.to_cfg()?;
    logger::init(cfg.debug)?;
    tracing::debug!("cfg: {:#?}", &cfg);
    let mut tmux = Tmux::new(&cfg.sock, &cfg.session);
    match &cli.command {
        cli::Cmd::Status => cmd::status(&tmux),
        cli::Cmd::Attach => cmd::attach(&tmux),
        cli::Cmd::Start => cmd::start(&cfg, &mut tmux),
        cli::Cmd::Stop => cmd::stop(&cfg, &tmux),
        cli::Cmd::Restart => cmd::restart(&cfg, &mut tmux),
    }
}
