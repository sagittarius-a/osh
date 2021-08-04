use console::style;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs::OpenOptions;
use std::io::Result;
use std::io::{stdout, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use log::LevelFilter;
use log::{info, warn};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

macro_rules! debug {
    ($config:ident) => {
        if $config.debug {
            print!("\n")
        }
    };
    ($config:ident, $fmt:expr) => {
        if $config.debug {
            print!(concat!($fmt, "\n"));
        }
    };
    ($config:ident, $fmt:expr, $($arg:tt)*) => {
            if $config.debug {
            print!(concat!($fmt, "\n"), $($arg)*)
        }
    };
}

struct ShellCommand {
    command: String,
    args: String,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    aliases: HashMap<String, String>,
    #[serde(default = "default_prompt")]
    prompt: String,
    #[serde(default)]
    debug: bool,
    #[serde(default = "get_username")]
    username: String,
    #[serde(default = "get_hostname")]
    hostname: String,
}

fn default_prompt() -> String {
    "$".to_string()
}

impl ConfigFile {
    fn new() -> ConfigFile {
        let mut config_file: Option<ConfigFile> = None;

        // Try to read the configuration file
        match std::fs::File::open(perform_expansion("~/.shell.yaml")) {
            Ok(f) => {
                // Load aliases
                config_file = serde_yaml::from_reader(f).unwrap();
            }
            Err(_) => {
                eprintln!("Cannot open configuration file '~/.shell.yaml'");
            }
        };
        match config_file {
            Some(c) => {
                debug!(c, "Config file: {:#?}", c);
                c
            }
            None => ConfigFile {
                aliases: HashMap::new(),
                prompt: default_prompt(),
                debug: false,
                username: get_username(),
                hostname: get_hostname(),
            },
        }
    }
}

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

fn setup_logging() {
    // https://docs.rs/log4rs/1.0.0/log4rs/encode/pattern/index.html
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build("/tmp/shell.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))
        .unwrap();

    log4rs::init_config(config).unwrap();
}

/// Replace the `command` with an alias if available.
fn lookup_aliases(config: &ConfigFile, command: &str, args: &str) -> Option<ShellCommand> {
    if !config.aliases.contains_key(command) {
        return None;
    }

    let mut c: &str = config.aliases.get_key_value(command).unwrap().1;

    let mut parts = c.trim().split_whitespace();
    c = parts.next().unwrap();

    // If any args in the alias, prepend them to the list of arguments
    let alias_args = parts.into_iter().collect::<Vec<&str>>().join(" ");

    let a = match args.is_empty() {
        true => alias_args,
        false => format!("{} {}", alias_args, args),
    };

    Some(ShellCommand {
        command: c.to_string(),
        args: a,
    })
}

fn list_aliases(aliases: &HashMap<String, String>) {
    for (key, value) in aliases.iter() {
        println!("{}: {}", key, value);
    }
}

/// Perform environment variable expansion.
fn perform_expansion(value: &str) -> String {
    // Early exit if not variable to expand is found
    if !value.contains('$') {
        if value.contains('~') && env::var("HOME").is_ok() {
            return value.replace('~', &env::var("HOME").unwrap());
        }
        return value.into();
    }

    let mut result = Vec::new();

    // Use space and slash as delimiters for environment variables
    let mut iter_space = value.split(' ').into_iter().peekable();

    while iter_space.peek().is_some() {
        let s = iter_space.next().unwrap();

        let mut iter_slash = s.split('/').into_iter().peekable();

        // Operate on slash separated values
        while iter_slash.peek().is_some() {
            let element = iter_slash.next().unwrap();

            // If the current element start with a '$'
            if let Some(key) = element.strip_prefix('$') {
                // Lookup for the given value in the environment
                let exp = match env::var(key) {
                    Ok(x) => x,
                    Err(_) => "".into(),
                };
                // Store its substitution value
                result.push(exp.clone());

                // Add a trailing slash if there is still some elements that
                // have been split by a slash
                if iter_slash.peek().is_some() {
                    println!("    Still some more element {}", iter_slash.peek().unwrap());
                    result.push('/'.into());
                }
            }
        }

        // Append a whitespace if there is still some elements that have been
        // split by a space
        if iter_space.peek().is_some() {
            result.push(' '.into());
        }
    }

    let expanded = result.join("");
    if expanded.contains('~') && env::var("HOME").is_ok() {
        return expanded.replace('~', &env::var("HOME").unwrap());
    }
    expanded
}

fn get_username() -> String {
    let mut username = String::new();
    if let Ok(u) = env::var("USERNAME") {
        username = u;
    };
    username
}

fn get_hostname() -> String {
    let mut hostname = String::new();
    if let Ok(mut f) = std::fs::File::open("/etc/hostname") {
        let mut tmp = String::new();
        f.read_to_string(&mut tmp).unwrap();
        hostname = tmp.trim().into();
    };
    hostname
}
// Prompt format: user@host pwd
//                green     blue
fn build_prompt(config: &ConfigFile) -> String {
    let mut prompt = String::new();

    // Fetch current directory
    let cwd = match env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}", e);
            // Use empty value if current directory cannot be read from env
            // It is very unlikely but who knows
            std::path::PathBuf::new()
        }
    };

    debug!(config, "cwd: {:?}", cwd);
    debug!(config, "config.username: {:?}", config.username);

    if !config.username.is_empty() {
        prompt += &style(&config.username).green().to_string();
    }
    if !config.hostname.is_empty() {
        prompt += &style(format!("@{} ", &config.hostname)).green().to_string();
    }
    prompt += &format!(
        "{} {} ",
        style(cwd.to_str().unwrap().replace("\"", ""))
            .blue()
            .bold()
            .to_string(),
        config.prompt
    );

    prompt
}

fn main() -> Result<()> {
    setup_logging();

    // Initialize interactive prompt
    let mut previous_directory = env::current_dir().unwrap();
    let mut rl = Editor::<()>::new();
    let homedir = match env::var("HOME") {
        Ok(val) => val,
        // Use /tmp as default directory if no $HOME directory has been found
        // This way, the user can still use this feature, even if the history
        // content won't survive reboots
        // TODO: Avoid collision when used by multiple users (even without $USER)
        Err(_) => "/tmp".into(),
    };
    let history = homedir + "/.history";
    let _ = rl.load_history(&history);
    let mut config = ConfigFile::new();

    loop {
        let prompt = build_prompt(&config);
        // Need to explicitly flush to ensure it prints before read_line
        stdout().flush().unwrap();

        match rl.readline(&prompt) {
            Ok(line) => {
                if line.is_empty() {
                    continue;
                }

                // Save input in history
                rl.add_history_entry(line.as_str());
                rl.save_history(&history).unwrap();

                // read_line leaves a trailing newline, which trim removes
                // this needs to be peekable so we can determine when we are on the last command
                let commands = line.trim().split(" | ").peekable();
                let mut previous_command = None;

                // For each command, use an alias if available. It allows user to use aliases
                // even in the commands following |
                let mut resolved = Vec::new();
                for command in commands {
                    let mut parts = command.trim().split_whitespace();
                    let mut command = parts.next().unwrap();
                    let args = parts;
                    let aliased;

                    // Perform environment variable expansion
                    let mut to_expand = Vec::new();
                    for a in args {
                        to_expand.push(a);
                    }
                    let mut args = perform_expansion(&to_expand.join(" "));

                    if let Some(shell_command) = lookup_aliases(&config, command, &args) {
                        aliased = shell_command.command.to_owned();
                        command = &aliased;
                        args = shell_command.args;
                    }

                    let c = format!("{} {}", command, args);
                    resolved.push(c);
                }

                // Reconstruct the command line with alias resolved
                let aliases_resolved_line = resolved.join(" | ");

                // Parse the new command line
                let mut commands = aliases_resolved_line.trim().split(" | ").peekable();

                while let Some(command) = commands.next() {
                    // everything after the first whitespace character is interpreted as args to the command
                    let mut parts = command.trim().split_whitespace();
                    let command = parts.next().unwrap();
                    let mut args = parts;

                    match command {
                        // Register a new alias
                        "alias" => {
                            // Fetch the name of the new alias or display availables aliases if not alias
                            // has been found
                            let new_alias = match args.next() {
                                Some(v) => v,
                                None => {
                                    list_aliases(&config.aliases);
                                    continue;
                                }
                            };

                            // Build the command by parsing the rest of the command provided
                            let aliased = args.into_iter().collect::<Vec<&str>>().join(" ");

                            config.aliases.insert(new_alias.into(), aliased);
                        }
                        // Delete alias
                        "unalias" => {
                            // Fetch the name of the new alias or display availables aliases if not alias
                            // has been found
                            let request = match args.next() {
                                Some(v) => v,
                                None => {
                                    eprintln!("No alias provided");
                                    continue;
                                }
                            };

                            if !config.aliases.contains_key(request) {
                                eprintln!("{} is not an alias", request);
                                continue;
                            }
                            config.aliases.remove(request);
                        }
                        // Edit configuration file
                        "config" => {
                            let editor = match env::var("EDITOR") {
                                Ok(e) => e,
                                Err(_) => {
                                    eprintln!("EDITOR not set. Cannot open configuration file");
                                    // TODO: set error code to 1
                                    continue;
                                }
                            };

                            let _ = Command::new(editor)
                                .args(perform_expansion("~/.shell.yaml").split_whitespace())
                                .stdin(Stdio::inherit())
                                .stdout(Stdio::inherit())
                                .spawn()
                                .unwrap()
                                .wait();
                        }
                        // Reload configuration file
                        "reload" => {
                            config = ConfigFile::new();
                        }
                        // Show the content of an alias
                        "type" => {
                            let request = match args.next() {
                                Some(v) => v,
                                None => {
                                    // TODO: set error code to 1
                                    continue;
                                }
                            };

                            match config.aliases.get_key_value(request) {
                                Some(c) => {
                                    println!("{} is an alias for {}", request, c.1);
                                }
                                None => {
                                    println!("{} not found", request);
                                }
                            }
                        }
                        "cd" => {
                            // default to '~' of '/' as new directory if one was not provided
                            let dir = match env::var("HOME") {
                                Ok(val) => val,
                                Err(_) => "/".into(),
                            };
                            let mut peek = args.peekable();
                            let new_dir = match peek.peek() {
                                Some(v) => v,
                                None => &dir[..],
                            };

                            let target;
                            // Use "-" to go to the last directory visited
                            if new_dir == "-" {
                                target = previous_directory.to_str().unwrap();
                            } else {
                                target = new_dir;
                            }

                            // Perform variable expansion
                            let target = perform_expansion(target);

                            let dir_before_cd = env::current_dir().unwrap();

                            if let Err(e) = env::set_current_dir(Path::new(&target)) {
                                eprintln!("Error: {}", e);
                                continue;
                            }

                            // Update the last directory if need be
                            if env::current_dir().unwrap() != dir_before_cd {
                                previous_directory =
                                    Path::new(dir_before_cd.to_str().unwrap()).to_path_buf();
                            }

                            previous_command = None;
                        }
                        "exit" => return Ok(()),
                        command => {
                            let stdin = previous_command
                                .map_or(Stdio::inherit(), |output: Child| {
                                    Stdio::from(output.stdout.unwrap())
                                });

                            // Perform environment variable expansion
                            let mut to_expand = Vec::new();
                            for a in args {
                                to_expand.push(a);
                            }
                            let mut args = perform_expansion(&to_expand.join(" "));

                            let has_and = args.contains("&&");
                            let has_redirection = args.contains('>');
                            let has_or = args.contains("&&");

                            // Assume there are no more commands piped behind this one
                            // send output to shell stdout
                            let mut stdout = Stdio::inherit();
                            let mut stderr = Stdio::inherit();
                            if commands.peek().is_some() {
                                // there is another command piped behind this one
                                // prepare to send output to the next command
                                stdout = Stdio::piped();
                                stderr = Stdio::piped();
                            } else if has_redirection {
                                args = args.split('>').next().unwrap().to_string();
                                stdout = Stdio::piped();
                                stderr = Stdio::piped();
                            }

                            debug!(config, ">>> command = {}", command);
                            debug!(config, ">>> args = '{}'", args);

                            let output = Command::new(command)
                                .args(args.split_whitespace())
                                .stdin(stdin)
                                .stdout(stdout)
                                .stderr(stderr)
                                .spawn();

                            match output {
                                Ok(output) => {
                                    // Process redirection if need be
                                    if has_redirection {
                                        let o = &output
                                            .wait_with_output()
                                            .expect("failed to wait on child");

                                        redirect(&line, o);
                                        previous_command = None;
                                    } else {
                                        previous_command = Some(output);
                                    }
                                }
                                Err(e) => {
                                    previous_command = None;
                                    eprintln!("{}", e);
                                }
                            };
                        }
                    }
                }

                if let Some(mut final_command) = previous_command {
                    // block until the final command has finished
                    final_command.wait().unwrap();
                }
            }
            Err(ReadlineError::Interrupted) => (),
            Err(ReadlineError::Eof) => {
                return Ok(());
            }
            Err(err) => {
                println!("Interactive error: {:?}. Exiting", err);
                break;
            }
        }
    }
    rl.save_history(&history).unwrap();

    Ok(())
}
