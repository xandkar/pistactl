use std::{io::Read, path::PathBuf, str::FromStr};

use anyhow::{anyhow, bail, Error, Result};

#[derive(Debug)]
pub struct Info {
    pub comm: String,
    pub fg: bool,

    /// Absolute, with prefixed /dev.
    pub tty: Option<PathBuf>,
}

pub fn list() -> Result<Vec<Info>> {
    let dev = PathBuf::from("/dev");
    let out = exec("ps", &["-eo", "stat,tty,comm"])?;
    let mut list = Vec::new();
    for line in out.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        match fields[..] {
            [state_codes, tty, comm, ..] => {
                let tty = match tty {
                    "?" => None,
                    _ => {
                        let tty = PathBuf::from_str(tty)?;
                        Some(dev.join(tty))
                    }
                };
                list.push(Info {
                    comm: comm.to_string(),
                    fg: state_codes.ends_with('+'),
                    tty,
                });
            }
            _ => bail!("Invalid ps output line: {:?}", &line),
        }
    }
    Ok(list)
}

pub fn exec(cmd: &str, args: &[&str]) -> Result<String> {
    let mut child = std::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            Error::from(e).context(format!("Failed to spawn {cmd:?} {args:?}"))
        })?;
    let status = &child.wait()?;

    let mut stdout = child
        .stdout
        .ok_or_else(|| anyhow!("Failed to get stdout of: {:?}", cmd))?;
    let mut stdout_str = String::new();
    stdout.read_to_string(&mut stdout_str)?;

    let mut stderr = child
        .stderr
        .ok_or_else(|| anyhow!("Failed to get stderr of: {:?}", cmd))?;
    let mut stderr_str = String::new();
    stderr.read_to_string(&mut stderr_str)?;

    status.success().then_some(stdout_str).ok_or_else(|| {
        let cmd_with_args = [&[cmd][..], args].concat().join(" ");
        anyhow!(
            "Failed to run: {:?}. Code: {}. Stderr: {:?}",
            &cmd_with_args,
            status.code().map_or("none".to_owned(), |n| n.to_string()),
            stderr_str
        )
    })
}

pub fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let _ = exec(cmd, args)?;
    Ok(())
}
