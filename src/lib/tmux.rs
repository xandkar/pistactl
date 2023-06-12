use anyhow::Result;

#[derive(Debug)]
pub struct Terminal {
    session: String,
    window: usize,
    pane: usize,
}

impl std::fmt::Display for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}.{}", self.session, self.window, self.pane)
    }
}

#[derive(Debug)]
pub struct Tmux {
    cur_window: usize,
    tot_windows: usize,
    sock: String,
    session: String,
}

impl Tmux {
    pub fn new(sock: &str, session: &str) -> Self {
        Self {
            cur_window: 0,
            tot_windows: 1,
            sock: sock.to_owned(),
            session: session.to_owned(),
        }
    }

    pub fn new_terminal(&mut self) -> Result<Terminal> {
        let available = self.tot_windows - self.cur_window;
        match available {
            0 => {
                self.new_window()?;
                self.tot_windows += 1;
            }
            1 => (),
            n => {
                tracing::error!("Invalid number of available windows: {}", n);
                unreachable!()
            }
        }
        let term = Terminal {
            session: self.session.clone(),
            window: self.cur_window,
            pane: 0,
        };
        self.cur_window += 1;
        tracing::debug!("Allocated terminal: {:?}", term);
        Ok(term)
    }

    pub fn status(&self) -> Result<()> {
        println!(
            "\
            socket name  : {:?},\n\
            session      : {:?},\n\
            tmux windows :\n\
            ",
            self.sock, self.session
        );
        self.run(&["list-windows", "-a"])
    }

    pub fn new_session(&self) -> Result<()> {
        self.run(&["new-session", "-d", "-s", &self.session])
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

    fn new_window(&self) -> Result<()> {
        self.run(&["new-window", "-t", &self.session])
    }

    fn run(&self, args: &[&str]) -> Result<()> {
        let args = [&["-L", &self.sock][..], args].concat();
        crate::process::run("tmux", &args[..])
    }
}
