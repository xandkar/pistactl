pub mod cli;
pub mod cmd;
pub mod logger;
pub mod tmux;

mod cfg;
mod process;

#[macro_export]
macro_rules! NAME {
    () => {
        env!("CARGO_CRATE_NAME")
    };
}
