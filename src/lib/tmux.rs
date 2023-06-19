use std::path::{Path, PathBuf};

use anyhow::{anyhow, Error, Result};

#[derive(Debug)]
pub struct Terminal {
    session: String,
    window_id: usize,
    pane_id: usize,
}

impl std::fmt::Display for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}.{}", self.session, self.window_id, self.pane_id)
    }
}

#[derive(Debug)]
pub struct PaneInfo {
    pub window_id: usize,
    pub window_name: String,
    pub tty: PathBuf,
    pub pane_id: usize,
}

impl std::str::FromStr for PaneInfo {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let fields: Vec<&str> = s.split_whitespace().collect();
        match &fields[..] {
            [window_id, window_name, tty, pane_id] => {
                let window_id = window_id
                    .strip_prefix('@')
                    .ok_or_else(|| {
                        anyhow!(
                            "Invalid window_id - missing prefix: {:?}",
                            window_id
                        )
                    })?
                    .parse()?;
                let pane_id = pane_id
                    .strip_prefix('%')
                    .ok_or_else(|| {
                        anyhow!(
                            "Invalid pane_id - missing prefix: {:?}",
                            pane_id
                        )
                    })?
                    .parse()?;
                let pane_info = PaneInfo {
                    pane_id,
                    window_id,
                    window_name: window_name.to_string(),
                    tty: PathBuf::from_str(tty)?,
                };
                Ok(pane_info)
            }
            _ => Err(anyhow!("Invalid PaneInfo string: {:?}", s)),
        }
    }
}

#[derive(Debug)]
pub struct Tmux {
    cur_window: usize,
    sock: String,
    session: String,
}

impl Tmux {
    pub fn new(sock: &str, session: &str) -> Self {
        Self {
            cur_window: 0,
            sock: sock.to_owned(),
            session: session.to_owned(),
        }
    }

    pub fn zeroth_terminal(&mut self, name: &str) -> Result<Terminal> {
        let window = 0;
        let term = Terminal {
            session: self.session.clone(),
            window_id: window,
            pane_id: 0,
        };
        self.rename_window(window, name)?;
        Ok(term)
    }

    pub fn new_terminal(
        &mut self,
        working_directory: &Path,
        name: &str,
    ) -> Result<Terminal> {
        self.new_window(working_directory, name)?;
        self.cur_window += 1;
        let term = Terminal {
            session: self.session.clone(),
            window_id: self.cur_window,
            pane_id: 0,
        };
        tracing::debug!("Allocated terminal: {:?}", term);
        Ok(term)
    }

    pub fn list_panes(&self) -> Result<Vec<PaneInfo>> {
        let out = self.exec(&[
            "list-panes",
            "-s",
            "-t",
            &self.session,
            "-F",
            "#{window_id} #{window_name} #{pane_tty} #{pane_id}",
        ])?;
        let mut panes = Vec::new();
        for line in out.lines() {
            panes.push(line.parse()?)
        }
        Ok(panes)
    }

    #[rustfmt::skip] // I want each option-value pair on the same line.
    pub fn new_session(&self, working_directory: &Path) -> Result<()> {
        let working_directory = working_directory.to_string_lossy();
        self.run(&[
            "new-session",
            "-d", // Start detached from the current terminal.
            "-c", &working_directory,
            "-s", &self.session,
        ])?;
        let s = self.session.as_str();

        // Prevent window names from dynamically changing:
        self.run(&["set-option", "-g", "-t", s, "allow-rename", "off"])?;

        Ok(())
    }

    pub fn kill_session(&self) -> Result<()> {
        self.run(&["kill-session", "-t", &self.session])
    }

    pub fn attach(&self) -> Result<()> {
        self.run(&["attach", "-t", &self.session])
    }

    /// For safety, launch commands in 2 steps:
    /// 1. send_text (key lookup disabled);
    /// 2. send_enter.
    pub fn send_text(&self, term: &Terminal, text: &str) -> Result<()> {
        tracing::debug!(
            "Sending text. Terminal: {:?}. Text: {:?}",
            term,
            text
        );
        // > The -l flag disables key name lookup and processes the keys as
        // > literal UTF-8 characters.
        self.run(&["send-keys", "-t", &term.to_string(), "-l", text])
    }

    pub fn send_enter(&self, term: &Terminal) -> Result<()> {
        self.run(&["send-keys", "-t", &term.to_string(), "ENTER"])
    }

    pub fn send_interrupt(&self, term: &Terminal) -> Result<()> {
        tracing::debug!("Sending interrupt. Terminal: {:?}", term);
        self.run(&["send-keys", "-t", &term.to_string(), "^C"])
    }

    #[rustfmt::skip] // I want each option-value pair on the same line.
    fn new_window(&self, working_directory: &Path, name: &str) -> Result<()> {
        let working_directory = working_directory.to_string_lossy();
        self.run(&[
            "new-window",
            "-c", &working_directory,
            "-n", name,
            "-t", &self.session,
        ])
    }

    fn rename_window(&self, window: usize, name: &str) -> Result<()> {
        let target = format!("{}:{}", self.session, window);
        self.run(&["rename-window", "-t", &target, name])
    }

    fn exec(&self, args: &[&str]) -> Result<String> {
        let args = [&["-L", &self.sock][..], args].concat();
        crate::process::exec("tmux", &args[..])
    }

    fn run(&self, args: &[&str]) -> Result<()> {
        let args = [&["-L", &self.sock][..], args].concat();
        crate::process::run("tmux", &args[..])
    }
}
