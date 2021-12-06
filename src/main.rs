mod alias;
mod config;
mod logging;
mod macros;
mod rustyline_helper;
mod shell;
mod utils;

use crate::logging::setup_logging;
use crate::shell::Osh;
extern crate shell_words;

fn main() -> rustyline::Result<()> {
    setup_logging();

    let mut shell = Osh::new();
    shell.repl()
}
