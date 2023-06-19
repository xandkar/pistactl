use std::{
    fs::File,
    io::{BufRead, BufReader},
    os::unix,
    path::Path,
};

use anyhow::{anyhow, Error, Result};

use crate::process;

pub fn mkfifo(path: &Path) -> Result<()> {
    let path = path.to_string_lossy();
    process::run("mkfifo", &[&path])
}

pub fn set_permissions(file: &File, perms_sum: u32) -> Result<()> {
    let mut perms = file.metadata()?.permissions();
    {
        use unix::fs::PermissionsExt;
        perms.set_mode(perms_sum);
    }
    file.set_permissions(perms)?;
    Ok(())
}

pub fn is_fifo(path: &Path) -> Result<bool> {
    let metadata = std::fs::metadata(path)?;
    let is_fifo = {
        use unix::fs::FileTypeExt;
        metadata.file_type().is_fifo()
    };
    Ok(is_fifo)
}

/// Reads first line from a file.
pub fn head(path: &Path) -> Result<String> {
    match BufReader::new(File::open(path)?).lines().next() {
        None => Err(anyhow!("FIFO empty and did not block: {:?}", path)),
        Some(Err(e)) => Err(Error::from(e)),
        Some(Ok(line)) => Ok(line),
    }
}
