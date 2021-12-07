// use crate::alias::list_aliases;
// use crate::alias::lookup_aliases;
use crate::alias::Alias;

use crate::config::ConfigFile;
use crate::rustyline_helper::{MyFilenameCompleter, MyHelper};
use crate::utils::Utils;
use crate::{wdebug, werror, winfo};

use std::collections::HashMap;
use std::env::{self, remove_var, set_var};
use std::fs::OpenOptions;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use rustyline::error::ReadlineError;
use rustyline::Movement;
use rustyline::Word;
use rustyline::{Cmd, Config, Editor, KeyEvent};

use rustyline::config::OutputStreamType;
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{CompletionType, EditMode};

use console::style;

#[derive(Debug)]
struct ShellCommand {
    command: String,
    args: Vec<String>,
    redirection: Redirection,
    piped: bool,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum Redirection {
    None,
    Stdout,
    Stderr,
    Both,
}

pub struct Osh {
    config: ConfigFile,
    pub aliases: HashMap<String, String>,
    rl: Editor<MyHelper>,
    history_path: String,
    status: u32,
    prompt: String,
    previous_directory: PathBuf,

struct BuiltinCommandResult {
    is_builtin: bool,
    /// Determine if we skip any further processing after builtin command execution
    skip: bool,
}

impl Osh {
    pub fn new() -> Self {
        // Initialize interactive prompt
        let editor_config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .tab_stop(4)
            .edit_mode(EditMode::Emacs)
            .output_stream(OutputStreamType::Stdout)
            .build();

        let helper = MyHelper {
            completer: MyFilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter {},
            colored_prompt: ">>>".to_owned(),
            validator: MatchingBracketValidator::new(),
        };

        let mut rl = Editor::with_config(editor_config);
        rl.set_helper(Some(helper));

        // Add custom keybindings to provide better user experience
        // It basically mimics features provided by various shell plugins
        rl.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchForward);
        rl.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchBackward);
        rl.bind_sequence(
            KeyEvent::alt('w'),
            Cmd::Kill(Movement::BackwardWord(1, Word::Emacs)),
        );
        rl.bind_sequence(KeyEvent::alt('u'), Cmd::Undo(1));
        rl.bind_sequence(KeyEvent::ctrl('f'), Cmd::CompleteHint);
        rl.bind_sequence(
            KeyEvent::ctrl('o'),
            Cmd::AcceptOrInsertLine {
                accept_in_the_middle: true,
            },
        );

        // Load history of previous sessions
        let homedir = match env::var("HOME") {
            Ok(val) => val,
            // Use /tmp as default directory if no $HOME directory has been found
            // This way, the user can still use this feature, even if the history
            // content won't survive reboots
            // TODO: Avoid collision when used by multiple users (even without $USER)
            Err(_) => "/tmp".into(),
        };
        let history_path = homedir + "/.history";
        let _ = rl.load_history(&history_path);

        let status = 0u32;

        let config = ConfigFile::new();
        let aliases = config.aliases.clone();

        let prompt = Osh::build_prompt(&config, status);

        Osh {
            config,
            aliases,
            history_path,
            rl,
            status,
            prompt,
            previous_directory: env::current_dir().unwrap(),
        }
    }

    // Prompt format: user@host pwd
    //                green     blue or red if status != 0
    fn build_prompt(config: &ConfigFile, status: u32) -> String {
        let mut prompt = String::new();

        // Fetch current directory
        let cwd = match env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                werror!("{}", e);
                // Use empty value if current directory cannot be read from env
                // It is very unlikely but who knows
                std::path::PathBuf::new()
            }
        };

        wdebug!(config, "cwd: {:?}", cwd);
        wdebug!(config, "config.username: {:?}", config.username);

        if !config.username.is_empty() {
            prompt += &style(&config.username).green().to_string();
        }
        if !config.hostname.is_empty() {
            prompt += &style(format!("@{} ", &config.hostname)).green().to_string();
        }

        if status == 0 {
            prompt += &format!(
                "{} {} ",
                style(cwd.to_str().unwrap().replace("\"", ""))
                    .blue()
                    .bold()
                    .to_string(),
                config.prompt
            );
        } else {
            prompt += &format!(
                "{} {} ",
                style(cwd.to_str().unwrap().replace("\"", ""))
                    .bold()
                    .to_string(),
                config.prompt
            );
        }

        prompt
    }

    /// Check if the supplied command is a builtin and set flags accordingly
    fn try_builtin(&mut self, shell_command: &ShellCommand) -> BuiltinCommandResult {
        let mut result = BuiltinCommandResult {
            is_builtin: true,
            skip: false,
        };

        match &shell_command.command[..] {
            "export" => {
                let mut args = shell_command.args.iter();
                let env_var = match args.next() {
                    Some(v) => v.clone(),
                    None => {
                        werror!("No environment variable provided");
                        self.status = 1;
                        result.skip = true;
                        return result;
                    }
                };

                match args.next() {
                    Some(v) => {
                        set_var(env_var, v);
                    }
                    None => {
                        remove_var(env_var);
                    }
                };

                self.status = 0;
            }
            "unset" => {
                let mut args = shell_command.args.iter();
                match args.next() {
                    Some(v) => {
                        remove_var(v);
                    }
                    None => {
                        werror!("No environment variable provided");
                        self.status = 1;
                        result.skip = true;
                    }
                };
            }
            "alias" => {
                // Register a new alias
                let mut args = shell_command.args.iter();
                let new_alias = match args.next() {
                    Some(v) => v.clone(),
                    None => {
                        self.list_aliases();
                        self.status = 0;
                        result.skip = true;
                        return result;
                    }
                };

                // Build the command by parsing the rest of the command provided
                let aliased = args.cloned().collect::<Vec<String>>().join(" ");

                self.config.aliases.insert(new_alias, aliased);
                self.status = 0;
            }
            "unalias" => {
                result.is_builtin = true;
                // Fetch the name of the new alias or display available aliases if not alias
                // has been found
                let mut args = shell_command.args.iter();
                let request = match args.next() {
                    Some(v) => v,
                    None => {
                        werror!("No alias provided");
                        self.status = 1;
                        result.skip = true;
                        return result;
                    }
                };

                if !self.config.aliases.contains_key(request) {
                    werror!("{} is not an alias", request);
                    self.status = 1;
                    result.skip = true;
                }
                self.config.aliases.remove(request);
                self.status = 0;
            }
            "config" => {
                let editor = match env::var("EDITOR") {
                    Ok(e) => e,
                    Err(_) => {
                        werror!("EDITOR variable not set. Cannot open configuration file");
                        self.status = 1;
                        result.is_builtin = true;
                        result.skip = true;
                        return result;
                    }
                };

                let _ = Command::new(editor)
                    .args(vec![Osh::perform_expansion_on_single_element(
                        "~/.shell.yaml",
                    )])
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .spawn()
                    .unwrap()
                    .wait();

                self.config = ConfigFile::new();
                winfo!("Configuration file reloaded");

                self.status = 0;
            }
            "reload" => {
                self.config = ConfigFile::new();
                winfo!("Configuration file reloaded");
                self.status = 0;
            }
            "status" => {
                winfo!("Status: {}", self.status);
                self.status = 0;
            }
            "history" => {
                result.is_builtin = true;

                for (i, h) in self.rl.history().iter().enumerate() {
                    println!("{:>3} :: {}", i, h);
                }
                self.status = 0;
            }
            "cd" => {
                // default to '~' of '/' as new directory if one was not provided
                let dir = match env::var("HOME") {
                    Ok(val) => val,
                    Err(_) => {
                        werror!("HOME variable not set. Using / as default target");
                        "/".into()
                    }
                };
                let mut args = shell_command.args.iter();
                let new_dir = match args.next() {
                    Some(v) => v,
                    None => &dir[..],
                };

                let target;
                // Use "-" to go to the last directory visited
                if new_dir == "-" {
                    target = self.previous_directory.to_str().unwrap();
                } else {
                    target = new_dir;
                }

                // Perform variable expansion
                let target = Osh::perform_expansion_on_single_element(target);
                // Save the location we're in before changing directory
                let dir_before_cd = env::current_dir().unwrap();

                if let Err(e) = env::set_current_dir(Path::new(&target)) {
                    werror!("Error: {}: '{}'", e, target);
                    self.status = 1;
                    result.skip = true;
                }

                // Update the last directory if need be
                if env::current_dir().unwrap() != dir_before_cd {
                    self.previous_directory =
                        Path::new(dir_before_cd.to_str().unwrap()).to_path_buf();
                }

                self.status = 0;
            }
            _ => {
                result.is_builtin = false;
            }
        }
        result
    }

    pub fn repl(&mut self) -> rustyline::Result<()> {
        'shell_loop: loop {
            self.prompt = Osh::build_prompt(&self.config, self.status);
            // Need to explicitly flush to ensure it prints before read_line
            stdout().flush().unwrap();
            self.rl.helper_mut().expect("No helper").colored_prompt = self.prompt.clone();

            match self.rl.readline(&self.prompt) {
                Ok(line) => {
                    if line.is_empty() {
                        continue;
                    }

                    // Save input in history
                    self.rl.add_history_entry(line.as_str());
                    self.rl.save_history(&self.history_path).unwrap();

                    let commands = shell_words::split(&line).expect("Failed to split command line");
                    let is_unalias_command = commands[0].eq("unalias");
                    let mut previous_command = None;

                    // For each command, use an alias if available. It allows user to use aliases
                    // even in the commands following | character
                    let mut resolved = Vec::new();
                    for command in commands {
                        let expanded = Osh::perform_expansion_on_single_element(&command);

                        if !is_unalias_command {
                            // If we've found an alias, resolve it and parse the resolved string as a new
                            // command, since it can be composed of several words
                            if let Some(resolved_alias) = self.lookup_aliases(&expanded) {
                                let parts = shell_words::split(&resolved_alias)
                                    .expect("Failed to split resolved alias");
                                for part in parts {
                                    resolved.push(part);
                                }
                            } else if let Some(wildcard_expanded) =
                                Osh::perform_wildcard_expansion(&expanded)
                            {
                                for w in wildcard_expanded.iter() {
                                    resolved.push(w.to_string());
                                }
                            } else {
                                // If no alias has been found, no wildcard expanded, simply use the
                                // word as is
                                resolved.push(expanded);
                            }
                        } else {
                            // We're dealing with "unalias" command so we need to make sure to keep
                            // value as is
                            resolved.push(expanded);
                        }
                    }

                    // Now the command line has been preprocessed, split it in several commands to
                    // execute
                    let shell_commands = self.build_commands(resolved);
                    for shell_command in shell_commands {
                        // Try to execute the command as builtin if need be
                        let builtin_result = self.try_builtin(&shell_command);
                        if builtin_result.skip {
                            continue 'shell_loop;
                        }

                        if !builtin_result.is_builtin {
                            let command = shell_command.command;
                            let stdin = previous_command
                                .map_or(Stdio::inherit(), |output: Child| {
                                    Stdio::from(output.stdout.unwrap())
                                });

                            let mut stdout = Stdio::inherit();
                            let mut stderr = Stdio::inherit();
                            if shell_command.piped {
                                stdout = Stdio::piped();
                                stderr = Stdio::piped();
                            } else if shell_command.redirection != Redirection::None {
                                stdout = Stdio::piped();
                                stderr = Stdio::piped();
                            }

                            wdebug!(self.config, "Command            : {}", command);
                            wdebug!(
                                self.config,
                                "Command args       : {:#?}",
                                &shell_command.args
                            );
                            wdebug!(self.config, "Command piped      : {}", &shell_command.piped);
                            wdebug!(
                                self.config,
                                "Command redirection: {:#?}",
                                &shell_command.redirection
                            );

                            let child = Command::new(command.clone())
                                .args(shell_command.args)
                                .stdin(stdin)
                                .stdout(stdout)
                                .stderr(stderr)
                                .spawn();

                            match child {
                                Ok(child) => {
                                    self.status = 0;

                                    if !shell_command.piped {
                                        child.wait_with_output().expect("failed to wait on child");
                                        previous_command = None;
                                    } else {
                                        previous_command = Some(child);
                                    }

                                    // self.child = None;

                                    // // Process redirection if need be
                                    // if shell_command.redirection != Redirection::None {
                                    //     let _o = &child
                                    //         .wait_with_output()
                                    //         .expect("failed to wait on child");
                                    //
                                    //     wwarning!("TODO: manage redirection");
                                    //     // redirect(&line, o);
                                    //     previous_command = None;
                                    //     previous_command = Some(child);
                                    // } else {
                                    //     println!("Previous command = {:#?}", &child);
                                    //     previous_command = Some(child);
                                    // }
                                }
                                Err(e) => {
                                    previous_command = None;
                                    werror!("{}: {:?}", e, command);
                                    self.status = 1;
                                }
                            };
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => (),
                Err(ReadlineError::Eof) => {
                    return Ok(());
                }
                Err(err) => {
                    werror!("Interactive error: {:?}. Exiting", err);
                    break;
                }
            }
        }
        self.rl.save_history(&self.history_path).unwrap();

        Ok(())
    }

    fn build_commands(&self, words: Vec<String>) -> Vec<ShellCommand> {
        let mut commands = Vec::new();

        // Find command separators
        let mut parts = Vec::new();
        let mut current = Vec::new();
        for w in words {
            if w.eq("|") {
                parts.push((current, true));
                current = Vec::new();
            } else if w.eq(";") {
                parts.push((current, false));
                current = Vec::new();
            } else {
                current.push(w)
            }
        }
        if !current.is_empty() {
            parts.push((current, false));
        }

        for part in parts {
            if part.0.len() > 1 {
                let command = &part.0[0];
                let piped = &part.1;
                let redirection = Redirection::None;
                let args = &part.0[1..];

                // TODO: Parse args to find potential redirection
                //
                // In order to perform this operation:
                // $ id > /tmp/asdf | grep uid
                //
                // This is not the kind of operation I'd like to do everyday but keeping a note is the
                // best way to think about it some day

                commands.push(ShellCommand {
                    command: command.to_string(),
                    args: args.to_vec(),
                    redirection,
                    piped: *piped,
                })
            } else {
                // Command only contains the command itself, no redirection, no pipe
                commands.push(ShellCommand {
                    command: part.0[0].to_string(),
                    args: Vec::new(),
                    redirection: Redirection::None,
                    piped: part.1,
                });
            }
        }

        commands
    }

    #[allow(dead_code)]
    /// Redirect stdout and/or stderr to a file
    fn redirect(command: &str, output: &std::process::Output) {
        let filename;
        let mut file_options = OpenOptions::new();

        // Set default permissions
        file_options.write(true);

        let mut content = String::from_utf8_lossy(&output.stdout);

        let mut parts = command.split('>');

        // Determine if stderr needs to be redirected as well
        if parts.next().unwrap().ends_with('2') {
            let stderr = String::from_utf8_lossy(&output.stderr);
            content += stderr;
        }

        // Determine if the file must be truncated or if the content should be
        // appended
        if command.contains(">>") {
            file_options.append(true);
            filename = command.split(">>").nth(1).unwrap().trim();
        } else {
            file_options.truncate(true);
            file_options.create(true);
            filename = command.split('>').nth(1).unwrap().trim();
        }

        let mut file = file_options
            .open(filename)
            .unwrap_or_else(|_| panic!("Failed to open {} to redirect content to it", filename));
        write!(file, "{}", content)
            .unwrap_or_else(|_| panic!("Failed to write content to redirect to {}", filename));
    }
}
