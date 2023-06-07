use std::io::Read;

use anyhow::{anyhow, Error, Result};

pub fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let mut child = std::process::Command::new(cmd)
        .args(args)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            Error::from(e)
                .context(format!("Failed to spawn {cmd:?} {args:?}"))
        })?;
    let status = &child.wait()?;
    let mut stderr = child
        .stderr
        .ok_or_else(|| anyhow!("Failed to get stderr of: {:?}", cmd))?;
    let mut stderr_str = String::new();
    stderr.read_to_string(&mut stderr_str)?;
    status.success().then_some(()).ok_or_else(|| {
        let cmd_with_args = [&[cmd][..], args].concat().join(" ");
        anyhow!(
            "Failed to run: {:?}. Code: {}. Stderr: {:?}",
            &cmd_with_args,
            status.code().map_or("none".to_owned(), |n| n.to_string()),
            stderr_str
        )
    })
}
