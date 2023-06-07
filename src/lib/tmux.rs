use anyhow::Result;

#[derive(Debug)]
pub struct Tmux {
    i: usize,
    n: usize,
    sock: String,
    session: String,
}

impl Tmux {
    pub fn new(sock: &str, session: &str) -> Self {
        Self {
            i: 0,
            n: 1,
            sock: sock.to_owned(),
            session: session.to_owned(),
        }
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

    pub fn launch_cmd(&mut self, cmd: &str) -> Result<()> {
        match self.n - self.i {
            0 => self.new_window()?,
            1 => (),
            _ => unreachable!(),
        }
        self.send_keys(cmd)?;
        Ok(())
    }

    fn send_keys(&mut self, cmd: &str) -> Result<()> {
        let Tmux { i, session, .. } = self;
        let pane = 0;
        let target_pane = format!("{}:{}.{}", session, i, pane);
        self.run(&["send-keys", "-t", &target_pane, cmd, "ENTER"])?;
        self.i += 1;
        Ok(())
    }

    fn new_window(&mut self) -> Result<()> {
        self.run(&["new-window", "-t", &self.session])?;
        self.n += 1;
        Ok(())
    }

    fn run(&self, args: &[&str]) -> Result<()> {
        let args = [&["-L", &self.sock][..], args].concat();
        crate::process::run("tmux", &args[..])
    }
}
