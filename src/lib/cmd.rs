use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{anyhow, Error, Result};

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
    let term = tmux.new_terminal()?;
    tmux.send_text(&term, &pista_cmd)?;
    tmux.send_enter(&term)
}

pub fn stop(cfg: &Cfg, tmux: &Tmux) -> Result<()> {
    if let Err(err) = tmux.kill_session() {
        tracing::error!("Failure in kill session: {:?}", err);
    }
    // TODO Just remove the whole slots directory.
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
    // TODO Maybe just recursively find and delete ALL fifos? Just delete dir?
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

/// Reads first line from a file.
fn head(path: &Path) -> Result<String> {
    match BufReader::new(File::open(path)?).lines().next() {
        None => Err(anyhow!("FIFO empty and did not block: {:?}", path)),
        Some(Err(e)) => Err(Error::from(e)),
        Some(Ok(line)) => Ok(line),
    }
}

fn start_slot(s: &cfg::Slot, d: &Path, tmux: &mut Tmux) -> Result<String> {
    // TODO Write s.cmd to slot_dir/exe script and execute the script file.
    // TODO slot_dir/{cmd,out,err}: executable, its stdout and stderr files.
    //      Logging stdout and stderr can be optional.
    let slot_pipe_path = d.join("out");
    let slot_pipe_str = slot_pipe_path.to_string_lossy();
    let d = d.to_string_lossy();
    process::run("mkdir", &["-p", &d])?;
    process::run("mkfifo", &[&slot_pipe_str])?;
    let cmd = format!(
        // TODO Capture stderr and display in exit notification?
        "cd {} && {} > {}; \
        notify-send -u critical 'pista slot exited!' \"{}\n$?\"",
        d, &s.cmd, slot_pipe_str, &s.cmd,
    );
    let term = tmux.new_terminal()?;
    tmux.send_text(&term, &cmd)?;
    tmux.send_enter(&term)?;
    let slot_len = match s.len {
        Some(len) => {
            tracing::info!(
                "User-defined slot length found: {}, for command: {:?}",
                len,
                &s.cmd
            );
            len
        }
        None => {
            tracing::warn!(
                "User-defined slot length NOT found. \
                Waiting for first line in FIFO: {:?}. \
                From command: {:?}",
                &slot_pipe_path,
                &s.cmd,
            );
            let head: String = head(&slot_pipe_path)?;
            // pista expects length in bytes. String::len already counts bytes,
            // but I just want to be super-explicit:
            let len = head.as_bytes().len();
            tracing::info!(
                "Read slot length: {}. Restarting command: {:?}",
                len,
                &slot_pipe_path
            );
            // XXX Restart just in case the cmd's refresh interval is very long
            //     and the slot will be surprisingly empty.
            tmux.send_interrupt(&term)?;
            tmux.send_text(&term, &cmd)?;
            tmux.send_enter(&term)?;
            len
        }
    };
    let pista_slot_spec = format!("{} {} {}", slot_pipe_str, slot_len, s.ttl);
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
