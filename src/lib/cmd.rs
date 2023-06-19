use std::{
    collections::HashSet,
    fs::{self, File},
    io::Write,
    iter::zip,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::{
    cfg::{self, Cfg},
    process, scripts,
    tmux::{self, Tmux},
};

const PERM_OWNER_RWX: u32 = 0o100 + 0o200 + 0o400;

pub fn status(cfg: &Cfg, tmux: &Tmux) -> Result<()> {
    let name_run = "run"; // TODO top level const
    let name_err = "err"; // TODO top level const
    let dir = &cfg.slots_fifos_dir;
    let fg: HashSet<PathBuf> = process::list()?
        .into_iter()
        .filter_map(|proc| match proc.tty {
            Some(tty) if proc.fg && proc.comm == name_run => Some(tty),
            _ => None,
        })
        .collect();
    let mut panes = tmux.list_panes()?;
    panes.sort_by(|a, b| a.window_id.cmp(&b.window_id));
    println!("POSITION NAME RUNNING? LOG_LINES");
    for tmux::PaneInfo {
        window_id,
        window_name,
        tty,
        pane_id,
    } in panes
    {
        assert_eq!(window_id, pane_id);
        let log_file = match window_id {
            0 => dir.join(name_err),
            _ => dir
                .join(slot_dir_name(window_id, &window_name))
                .join(name_err),
        };
        // TODO Per log level? How to not assume log format?
        let log_lines = fs::read_to_string(log_file)?.lines().count();
        let is_running = fg.get(&tty).map_or("NO", |_| "YES");
        println!(
            "{} {} {} {}",
            &window_id, &window_name, is_running, log_lines
        );
        if window_id == 0 {
            assert_eq!("pista", window_name);
        }
    }
    Ok(())
}

pub fn attach(tmux: &Tmux) -> Result<()> {
    tmux.attach()
}

pub fn restart(cfg: &Cfg, tmux: &mut Tmux) -> Result<()> {
    let _ = stop(cfg, tmux);
    start(cfg, tmux)
}

pub fn start(cfg: &Cfg, tmux: &mut Tmux) -> Result<()> {
    let name_run = "run";
    let name_out = "out";
    let name_err = "err";
    let dir = &cfg.slots_fifos_dir;
    fs::create_dir_all(dir)?;
    tmux.new_session(dir)?;
    let pista_slot_specs = start_slots(cfg, tmux)?;
    {
        let mut run = File::create(dir.join(name_run))?;
        writeln!(run, "#! /bin/bash")?;
        writeln!(
            run,
            "pista {} {} >> ./{} 2>> ./{};",
            &cfg.pista.to_arg_str(),
            pista_slot_specs.join(" "),
            name_out,
            name_err
        )?;
        writeln!(run, "code=$?")?;
        writeln!(
            run,
            "log=$({})",
            &scripts::tail_log(
                name_err,
                cfg.notifications.log_lines_limit,
                &cfg.notifications.indent,
                cfg.notifications.width_limit
            )
        )?;
        writeln!(run, "body=\"code: $code\nlog:\n$log\"")?;
        writeln!(
            run,
            "{}",
            scripts::notify_send_critical("'pista exited!'", "\"$body\"",)
        )?;
        crate::fs::set_permissions(&run, PERM_OWNER_RWX)?;
        run.sync_all()?;
    }
    let term = tmux.zeroth_terminal("pista")?;
    tmux.send_text(&term, &format!("./{}", name_run))?;
    tmux.send_enter(&term)
}

pub fn stop(cfg: &Cfg, tmux: &Tmux) -> Result<()> {
    if let Err(err) = tmux.kill_session() {
        tracing::error!("Failure in kill session: {:?}", err);
    }
    if let Err(err) = fs::remove_dir_all(&cfg.slots_fifos_dir) {
        tracing::error!(
            "Failure in removal of slot directory: {:?}. Error: {:?}",
            &cfg.slots_fifos_dir,
            err
        );
    }
    Ok(())
}

fn start_slot(
    notif: &cfg::Notifications,
    slot: &cfg::Slot,
    slot_dir: &Path,
    slot_name: &str,
    tmux: &mut Tmux,
) -> Result<String> {
    let name_cmd = "cmd";
    let name_run = "run";
    let name_out = "out";
    let name_err = "err";
    std::fs::create_dir_all(slot_dir)?;
    let slot_pipe = slot_dir.join(name_out);
    crate::fs::mkfifo(&slot_pipe)?;
    {
        let mut cmd = File::create(slot_dir.join(name_cmd))?;
        writeln!(cmd, "#! {}", slot.interpreter.display())?;
        writeln!(cmd, "{}", slot.cmd)?;
        crate::fs::set_permissions(&cmd, PERM_OWNER_RWX)?;
        cmd.sync_all()?;
    }
    {
        let mut run = File::create(slot_dir.join(name_run))?;
        writeln!(run, "#! /bin/bash")?;
        writeln!(run, "# This script wraps the user-provided script,")?;
        writeln!(run, "# which was written to ./{},", name_cmd)?;
        writeln!(run, "# adding output redirection and")?;
        writeln!(run, "# a notification in case of an unexpected exit.")?;
        writeln!(
            run,
            "cd {:?} && ./{} > ./{} 2>> ./{};",
            slot_dir, name_cmd, name_out, name_err
        )?;
        writeln!(run, "code=$?")?;
        writeln!(run, "slot_name={}", slot_name)?;
        writeln!(
            run,
            "log=$({})",
            &scripts::tail_log(
                name_err,
                notif.log_lines_limit,
                &notif.indent,
                notif.width_limit
            )
        )?;
        writeln!(run, "body=\"slot: $slot_name\ncode: $code\nlog:\n$log\"")?;
        writeln!(
            run,
            "{}",
            scripts::notify_send_critical("'pista feed exited!'", "\"$body\"",)
        )?;
        crate::fs::set_permissions(&run, PERM_OWNER_RWX)?;
        run.sync_all()?;
    }
    let term = tmux.new_terminal(slot_dir, slot_name)?;
    let dot_slash_run = format!("./{}", name_run);
    tmux.send_text(&term, &dot_slash_run)?;
    tmux.send_enter(&term)?;
    let slot_len = match slot.len {
        Some(len) => {
            tracing::info!(
                "User-defined slot length found: {}, for command: {:?}",
                len,
                &slot.cmd
            );
            len
        }
        None => {
            tracing::warn!(
                "User-defined slot length NOT found. \
                Waiting for first line in FIFO: {:?}. \
                From command: {:?}",
                &slot_pipe,
                &slot.cmd,
            );
            let head: String = crate::fs::head(&slot_pipe)?;
            // pista expects length in bytes. String::len already counts bytes,
            // but I just want to be super-explicit:
            let len = head.as_bytes().len();
            tracing::info!(
                "Read slot length: {}. Restarting command: {:?}",
                len,
                &slot.cmd
            );
            // XXX Restart just in case the cmd's refresh interval is very long
            //     and the slot will be surprisingly empty.
            tmux.send_interrupt(&term)?;
            tmux.send_text(&term, &dot_slash_run)?;
            tmux.send_enter(&term)?;
            len
        }
    };
    let pista_slot_spec = format!("{:?} {} {}", slot_pipe, slot_len, slot.ttl);
    Ok(pista_slot_spec)
}

fn start_slots(cfg: &Cfg, tmux: &mut Tmux) -> Result<Vec<String>> {
    let mut pista_slot_specs = Vec::new();
    for (i, s) in zip(1.., cfg.pista.slots.iter()) {
        let slot_name = match s.name {
            None => i.to_string(),
            Some(ref name) => name.to_string(),
        };
        let slot_dir = cfg.slots_fifos_dir.join(slot_dir_name(i, &slot_name));
        let slot_spec =
            start_slot(&cfg.notifications, s, &slot_dir, &slot_name, tmux)?;
        pista_slot_specs.push(slot_spec);
    }
    Ok(pista_slot_specs)
}

fn slot_dir_name(position: usize, name: &str) -> String {
    format!("{}-{}", position, name)
}
