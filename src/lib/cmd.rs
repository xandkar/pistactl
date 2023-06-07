use std::{fs, path::Path};

use anyhow::{Error, Result};

use crate::{
    cfg::{self, Cfg},
    process,
    tmux::Tmux,
};

pub fn status(tmux: &Tmux) -> Result<()> {
    tmux.status()
}

pub fn attach(tmux: &Tmux) -> Result<()> {
    tmux.attach()
}

pub fn restart(cfg: &Cfg, tmux: &mut Tmux) -> Result<()> {
    let _ = stop(cfg, tmux);
    start(cfg, tmux)
}

pub fn start(cfg: &Cfg, tmux: &mut Tmux) -> Result<()> {
    tmux.new_session()?;
    let pista_slot_specs = start_slots(cfg, tmux)?;
    let pista_cmd = format!(
        "pista {} {}; notify-send -u critical 'pista exited!' \"$?\"",
        &cfg.pista.to_arg_str(),
        pista_slot_specs.join(" ")
    );
    tmux.launch_cmd(&pista_cmd)
}

pub fn stop(cfg: &Cfg, tmux: &Tmux) -> Result<()> {
    if let Err(err) = tmux.kill_session() {
        tracing::error!("Failure in kill session: {:?}", err);
    }
    if let Err(err) = remove_slot_pipes(cfg) {
        tracing::error!("Failure in removal of slot pipes: {:?}", err);
    }
    Ok(())
}

fn is_fifo(path: &Path) -> Result<bool> {
    use std::os::unix::prelude::FileTypeExt;
    let metadata = fs::metadata(path)?;
    Ok(metadata.file_type().is_fifo())
}

fn remove_slot_pipes(cfg: &Cfg) -> Result<()> {
    // TODO Maybe just recursively find and delete ALL fifos?
    let dir = cfg.slots_fifos_dir.clone();
    let entries = fs::read_dir(&dir).map_err(|e| {
        Error::from(e).context(format!("Failed to list directory {:?}", &dir))
    })?;
    for entry_result in entries {
        let entry = entry_result?;
        let slot_dir = entry.path();
        if slot_dir.is_dir() {
            let slot_fifo = slot_dir.join("out");
            if slot_fifo.exists() {
                if is_fifo(&slot_fifo)? {
                    tracing::info!("removing: {:?}", &slot_fifo);
                    if let Err(err) = fs::remove_file(&slot_fifo) {
                        tracing::error!(
                            "Failed to remove slot FIFO file: {:?}. Error: {:?}",
                            slot_fifo,
                            err
                        );
                    }
                } else {
                    tracing::warn!("not a fifo: {:?}", &slot_fifo);
                }
            } else {
                tracing::debug!("not found: {:?}", &slot_fifo);
            }
        }
    }
    Ok(())
}

fn start_slot(s: &cfg::Slot, d: &Path, tmux: &mut Tmux) -> Result<String> {
    process::run("mkdir", &["-p", &d.to_string_lossy().to_string()])?;
    let slot_pipe = d.join("out").to_string_lossy().to_string();
    process::run("mkfifo", &[&slot_pipe])?;
    let cmd = format!(
        // TODO Capture stderr and display in exit notification?
        "cd {} && {} > {}; \
        notify-send -u critical 'pista slot exited!' \"{}\n$?\"",
        d.to_string_lossy().to_string(),
        &s.cmd,
        slot_pipe,
        &s.cmd,
    );
    tmux.launch_cmd(&cmd)?;
    let pista_slot_spec = format!("{} {} {}", slot_pipe, s.len, s.ttl);
    Ok(pista_slot_spec)
}

fn start_slots(cfg: &Cfg, tmux: &mut Tmux) -> Result<Vec<String>> {
    let mut pista_slot_specs = Vec::new();
    for (i, s) in std::iter::zip(1.., cfg.pista.slots.iter()) {
        let slot_dir = cfg.slots_fifos_dir.join(i.to_string());
        let slot_spec = start_slot(s, &slot_dir, tmux)?;
        pista_slot_specs.push(slot_spec);
    }
    Ok(pista_slot_specs)
}
