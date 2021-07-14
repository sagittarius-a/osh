use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::collections::HashMap;
use std::env;
use std::io::Result;
use std::io::{stdout, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

struct ShellCommand {
    command: String,
    args: String,
}

/// Replace the `command` with an alias if available.
fn lookup_aliases(
    aliases: &HashMap<String, String>,
    command: &str,
    args: &str,
) -> Option<ShellCommand> {
    if !aliases.contains_key(command) {
        return None;
    }

    let mut c: &str = aliases.get_key_value(command).unwrap().1;

    let mut parts = c.trim().split_whitespace();
    c = parts.next().unwrap();

    // If any args in the alias, prepend them to the list of arguments
    let alias_args = parts.into_iter().collect::<Vec<&str>>().join(" ");
    let a = &format!("{} {}", alias_args, args);

    Some(ShellCommand {
        command: c.to_string(),
        args: a.to_string(),
    })
}

/// Replace the `command` with an alias if available.
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

fn main() -> Result<()> {
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
    let mut aliases: HashMap<String, String> = HashMap::new();

    loop {
        // Setup prompt
        let cwd = match env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{}", e);
                // Use empty prompt if current directory cannot be read from env
                std::path::PathBuf::new()
            }
        };
        let prompt = format!("{} $ ", cwd.to_str().unwrap().replace("\"", ""));
        // Need to explicitly flush to ensure it prints before read_line
        stdout().flush().unwrap();

        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                if line.is_empty() {
                    continue;
                }

                // Save input in history
                rl.add_history_entry(line.as_str());
                rl.save_history(&history).unwrap();

                // read_line leaves a trailing newline, which trim removes
                // this needs to be peekable so we can determine when we are on the last command
                let mut commands = line.trim().split(" | ").peekable();
                let mut previous_command = None;

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
                                    list_aliases(&aliases);
                                    continue;
                                }
                            };

                            // Build the command by parsing the rest of the command provided
                            let aliased = args.into_iter().collect::<Vec<&str>>().join(" ");

                            aliases.insert(new_alias.into(), aliased);
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

                            match aliases.get_key_value(request) {
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
                        mut command => {
                            let stdin = previous_command
                                .map_or(Stdio::inherit(), |output: Child| {
                                    Stdio::from(output.stdout.unwrap())
                                });

                            let stdout = if commands.peek().is_some() {
                                // there is another command piped behind this one
                                // prepare to send output to the next command
                                Stdio::piped()
                            } else {
                                // there are no more commands piped behind this one
                                // send output to shell stdout
                                Stdio::inherit()
                            };

                            // Perform environment variable expansion
                            let mut to_expand = Vec::new();
                            for a in args {
                                to_expand.push(a);
                            }
                            let mut args = perform_expansion(&to_expand.join(" "));

                            // Use alias if available
                            let aliased;
                            if let Some(shell_command) = lookup_aliases(&aliases, command, &args) {
                                aliased = shell_command.command.to_owned();
                                command = &aliased;
                                args = shell_command.args;
                            }

                            let output = Command::new(command)
                                .args(args.split_whitespace())
                                .stdin(stdin)
                                .stdout(stdout)
                                .spawn();

                            match output {
                                Ok(output) => {
                                    previous_command = Some(output);
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
