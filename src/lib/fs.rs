use std::{
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader},
    os::unix,
    path::Path,
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::{Context, Error, Result};

use crate::process;

// ----------------------------------------------------------------------------
// API
// ----------------------------------------------------------------------------

pub fn file_create<P: AsRef<Path> + Debug>(path: P) -> Result<File> {
    File::create(&path)
        .with_context(|| format!("Failed to create file: {:?}", &path))
}

pub fn mkfifo(path: &Path) -> Result<()> {
    let path = path.to_string_lossy();
    process::run("mkfifo", &[&path])
}

pub fn set_permissions(file: &File, perms_sum: u32) -> Result<()> {
    _set_permissions(file, perms_sum).with_context(|| {
        format!("Failed to set permissions ({:?}) for {:?}", perms_sum, file)
    })
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
pub fn head(path: &Path, timeout: Duration) -> Result<Option<String>> {
    let file = File::open(path)?;
    let mut lines = BufReader::new(file).lines();
    let (sender, receiver) = mpsc::channel();
    {
        let path = path.to_owned();
        thread::spawn(move || {
            let line_result_opt = lines.next();
            sender.send(line_result_opt).unwrap_or_else(|err| {
                tracing::error!(
                    "Failure to send line read from {:?}. Error: {:?}",
                    &path,
                    err
                )
            });
        });
    }
    match receiver.recv_timeout(timeout) {
        Ok(Some(Err(e))) => Err(Error::from(e)),
        Ok(Some(Ok(line))) => Ok(Some(line)),
        Ok(None) => {
            tracing::warn!("FIFO empty and did not block: {:?}", path);
            Ok(None)
        }
        Err(_) => {
            tracing::error!("Timed out waiting to read: {:?}", path);
            Ok(None)
        }
    }
}

pub fn read_to_string(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read from path: {:?}", path))
}

pub fn create_dir_all(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("Failed to create dirs for path: {:?}", path))
}

// ----------------------------------------------------------------------------
// Internal
// ----------------------------------------------------------------------------

fn _set_permissions(file: &File, perms_sum: u32) -> Result<()> {
    let mut perms = file.metadata()?.permissions();
    {
        use unix::fs::PermissionsExt;
        perms.set_mode(perms_sum);
    }
    file.set_permissions(perms)?;
    Ok(())
}
