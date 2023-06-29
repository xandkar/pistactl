pub mod cfg;
pub mod cmd;
pub mod fs;
pub mod logger;
pub mod tmux;

mod process;
mod scripts;
mod x11;

#[macro_export]
macro_rules! NAME {
    () => {
        env!("CARGO_CRATE_NAME")
    };
}
