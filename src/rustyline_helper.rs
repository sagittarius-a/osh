use std::borrow::Cow::{self, Borrowed, Owned};
use std::env::current_dir;
use std::fs;
use std::io::Cursor;
use std::path::{self, Path};

use rustyline::completion::{escape, extract_word, unescape, Completer, Pair, Quote};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{self, MatchingBracketValidator, Validator};
use rustyline::Context;
use rustyline_derive::Helper;

extern crate skim;
use skim::prelude::*;

#[derive(Helper)]
pub struct MyHelper {
    pub completer: MyFilenameCompleter,
    pub highlighter: MatchingBracketHighlighter,
    pub validator: MatchingBracketValidator,
    pub hinter: HistoryHinter,
    pub colored_prompt: String,
}

impl Completer for MyHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for MyHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    /// Hint for command suggestions based on history
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        // Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
        // Owned("\x1b[31m".to_owned() + hint + "\x1b[m")
        Owned("\x1b[2m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

impl Validator for MyHelper {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

/// A `Completer` for file and folder names.
pub struct MyFilenameCompleter {
    break_chars: &'static [u8],
    #[allow(dead_code)]
    double_quotes_special_chars: &'static [u8],
}

impl MyFilenameCompleter {
    pub fn new() -> Self {
        // Reuse values defined in rustyline
        const DEFAULT_BREAK_CHARS: [u8; 18] = [
            b' ', b'\t', b'\n', b'"', b'\\', b'\'', b'`', b'@', b'$', b'>', b'<', b'=', b';', b'|',
            b'&', b'{', b'(', b'\0',
        ];
        #[allow(dead_code)]
        const ESCAPE_CHAR: Option<char> = Some('\\');
        const DOUBLE_QUOTES_SPECIAL_CHARS: [u8; 4] = [b'"', b'$', b'\\', b'`'];

        Self {
            break_chars: &DEFAULT_BREAK_CHARS,
            double_quotes_special_chars: &DOUBLE_QUOTES_SPECIAL_CHARS,
        }
    }

    /// Some kind of fuzzy matching. Ues the last token as pattern and try to find it in files
    /// and directories in current directory
    /// Example: pattern `txt` would match any file containing it in name, such as `files.txt`
    /// or `txt-dir/`.
    pub fn try_complete_with_pattern(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // const ESCAPE_CHAR: Option<char> = Some('\\');
        const ESCAPE_CHAR: Option<char> = Some('\\');
        let (start, pattern) = extract_word(line, pos, ESCAPE_CHAR, self.break_chars);
        let pattern = unescape(pattern, ESCAPE_CHAR);

        let mut matches = self.complete_filename_with_pattern(
            &pattern,
            ESCAPE_CHAR,
            self.break_chars,
            Quote::None,
        );

        #[allow(clippy::unnecessary_sort_by)]
        matches.sort_by(|a, b| a.display.cmp(&b.display));
        Ok((start, matches))
    }

    fn complete_filename_with_pattern(
        &self,
        pattern: &str,
        esc_char: Option<char>,
        break_chars: &[u8],
        quote: Quote,
    ) -> Vec<Pair> {
        // Normalize directory and associated information
        let sep = path::MAIN_SEPARATOR;
        let (dir_name, file_name) = match pattern.rfind(sep) {
            Some(idx) => pattern.split_at(idx + sep.len_utf8()),
            None => ("", pattern),
        };

        let dir_path = Path::new(dir_name);
        let dir = if dir_path.is_relative() {
            if let Ok(cwd) = current_dir() {
                cwd.join(dir_path)
            } else {
                dir_path.to_path_buf()
            }
        } else {
            dir_path.to_path_buf()
        };

        let mut entries: Vec<Pair> = Vec::new();

        // if dir doesn't exist, then don't offer any completions
        if !dir.exists() {
            return entries;
        }

        let mut candidates = Vec::new();

        // Handle special patterns
        if pattern.contains("**") {
            candidates = self.list_all_files_and_directories(&dir);
        } else {
            // if any of the below IO operations have errors, just ignore them
            if let Ok(read_dir) = dir.read_dir() {
                for entry in read_dir.flatten() {
                    if let Some(s) = entry.file_name().to_str() {
                        if entry.file_name().to_str().unwrap().contains(file_name) {
                            if let Ok(_metadata) = fs::metadata(entry.path()) {
                                let candidate = String::from(dir_name) + s;
                                candidates.push(candidate);
                            }
                        }
                    }
                }
            }
        }

        if candidates.is_empty() {
            return entries;
        }

        if candidates.len() > 1 {
            // If multiple matches have been found, use skim to filter them
            let options = SkimOptionsBuilder::default()
                .height(Some("30%"))
                .multi(true)
                .reverse(true)
                .build()
                .unwrap();
            let item_reader = SkimItemReader::default();
            let items = item_reader.of_bufread(Cursor::new(candidates.join("\n")));

            #[allow(clippy::redundant_closure)]
            let selected_items = Skim::run_with(&options, Some(items))
                .map(|out| out.selected_items)
                .unwrap_or_else(|| Vec::new());

            let mut replacement = Vec::new();
            for item in selected_items.iter() {
                let name = escape(item.output().to_string(), esc_char, break_chars, quote);
                // Make sure to compute proper path by prefixing matched valued with the possible
                // directory they are in
                replacement.push(format!("{}{}", dir_name, name));
            }

            entries.push(Pair {
                display: "".into(), // No display needed since values will be changed in place
                replacement: replacement.join(" "),
            });
        } else {
            // If only one matching file has been found, return a single substitution so it will
            // be applied right away
            let candidate = candidates.first().unwrap().to_string();
            // Make sure to compute proper path by prefixing matched valued with the possible
            // directory they are in
            let path = format!("{}{}", dir_name, candidate);
            entries.push(Pair {
                display: "".into(), // No display needed since values will be changed in place
                replacement: escape(path, esc_char, break_chars, quote),
            });
        }

        entries
    }

    /// List all entries, both files and directories present in `dir`.
    fn list_all_files_and_directories(&self, dir: &Path) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(read_dir) = dir.read_dir() {
            // let file_name = self.normalize(file_name);
            for entry in read_dir.flatten() {
                files.push(entry.file_name().to_str().unwrap().to_string());
            }
        }
        files
    }
}

impl Default for MyFilenameCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for MyFilenameCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Provide custom completer that tries to complete filename based on a pattern.
        // If no pattern is provided, it will simply list files and directories.
        let (start, matches) = self.try_complete_with_pattern(line, pos, ctx).unwrap();

        Ok((start, matches))
    }
}
