mod file {
    #[derive(Debug, serde::Deserialize)]
    pub struct Cfg {
        pub debug: Option<bool>,
        pub sock_name: Option<String>,
        pub session: Option<String>,
        pub slots_fifos_dir: Option<String>,
        pub pista: Option<super::Pista>,
    }
}

use std::path::{Path, PathBuf};

use anyhow::{Error, Result};
use expanduser::expanduser;

#[derive(Debug)]
pub struct Cfg {
    pub debug: bool,
    pub sock: String,
    pub session: String,
    pub slots_fifos_dir: PathBuf,
    pub pista: Pista,
}

#[derive(Debug, serde::Deserialize)]
pub struct Pista {
    pub log_level: Option<PistaLogLevel>,
    pub x11: Option<bool>,
    pub interval: Option<f32>,
    pub expiry_character: Option<char>,
    pub pad_left: Option<String>,
    pub pad_right: Option<String>,
    pub separator: Option<String>,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Copy, Clone, serde::Deserialize)]
pub enum PistaLogLevel {
    Nothing = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
}

#[derive(Debug, serde::Deserialize)]
pub struct Slot {
    pub len: usize,
    pub ttl: i32,
    pub cmd: String,
}

impl Cfg {
    pub fn from_file(path: &Path) -> Result<Self> {
        let data: String = std::fs::read_to_string(path).map_err(|e| {
            Error::from(e).context(format!("Failed to read from: {:?}", path))
        })?;
        let file: file::Cfg = toml::from_str(&data).map_err(|e| {
            Error::from(e)
                .context(format!("Failed to parse TOML from: {:?}", path))
        })?;
        let default = Self::default()?;
        let cfg = Self {
            debug: file.debug.unwrap_or(default.debug),
            sock: file.sock_name.unwrap_or(default.sock),
            session: file.session.unwrap_or(default.session),
            slots_fifos_dir: {
                match file.slots_fifos_dir {
                    None => default.slots_fifos_dir,
                    Some(d) => expanduser(d)?,
                }
            },
            pista: file.pista.unwrap_or(default.pista),
        };
        Ok(cfg)
    }

    fn default() -> Result<Self> {
        let name = crate::NAME!();
        Ok(Self {
            debug: false,
            sock: name.to_string(),
            session: name.to_string(),
            slots_fifos_dir: expanduser(format!("~/.{}/slots", name))?,
            pista: Pista {
                interval: None,
                pad_left: None,
                pad_right: None,
                separator: None,
                x11: None,
                log_level: None,
                expiry_character: None,
                slots: vec![],
            },
        })
    }
}

impl Pista {
    pub fn to_arg_str(&self) -> String {
        let Pista {
            interval,
            pad_left,
            pad_right,
            separator: mid_sep,
            x11,
            log_level,
            expiry_character,
            slots: _,
        } = self;
        [
            interval
                .map(|i| vec![format!("-i {}", i)])
                .unwrap_or(vec![]),
            pad_left
                .as_ref()
                .map(|s| vec![format!("-f '{}'", s)])
                .unwrap_or(vec![]),
            mid_sep
                .as_ref()
                .map(|s| vec![format!("-s '{}'", s)])
                .unwrap_or(vec![]),
            pad_right
                .as_ref()
                .map(|s| vec![format!("-r '{}'", s)])
                .unwrap_or(vec![]),
            x11.map(|b| if b { vec!["-x".to_string()] } else { vec![] })
                .unwrap_or(vec![]),
            expiry_character
                .map(|c| vec![format!("-e '{}'", c)])
                .unwrap_or(vec![]),
            log_level
                .map(|l| vec![format!("-l {}", l as u8)])
                .unwrap_or(vec![]),
        ]
        .concat()
        .join(" ")
    }
}
