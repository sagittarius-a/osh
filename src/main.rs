mod alias;
mod config;
mod logging;
mod macros;
mod rustyline_helper;
mod shell;
mod utils;

use crate::logging::setup_logging;
use crate::rustyline_helper::{MyFilenameCompleter, MyHelper};
use crate::shell::shell_loop;

use rustyline::config::OutputStreamType;
// use rustyline::error::ReadlineError;
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
// use rustyline::Movement;
// use rustyline::Word;
use rustyline::{CompletionType, Config, EditMode};

extern crate shell_words;

fn main() -> rustyline::Result<()> {
    setup_logging();

    // Initialize interactive prompt
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .tab_stop(4)
        .edit_mode(EditMode::Emacs)
        .output_stream(OutputStreamType::Stdout)
        .build();

    let h = MyHelper {
        completer: MyFilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter {},
        colored_prompt: ">>>".to_owned(),
        validator: MatchingBracketValidator::new(),
    };

    shell_loop(config, h)
}
