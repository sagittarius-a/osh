use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::env;
use std::io::Result;
use std::io::{stdout, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Perform environment variable expansion.
fn perform_expansion(value: &str) -> String {
    // Early exit if not variable to expand is found
    if !value.contains('$') {
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

    // Remove last space and last slash inserted
    result.join("")
}

fn main() -> Result<()> {
    // Initialize interactive prompt
    let mut rl = Editor::<()>::new();
    let homedir = match env::var("HOME") {
        Ok(val) => val,
        Err(_) => "/tmp".into(),
    };
    let history = homedir + "/.history";
    println!("History: {}", history);
    let _ = rl.load_history(&history);

    loop {
        // Setup prompt
        let cwd = match env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{}", e);
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
                    let args = parts;

                    match command {
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

                            // Perform variable expansion
                            let new_dir = perform_expansion(new_dir);
                            if let Err(e) = env::set_current_dir(Path::new(&new_dir)) {
                                eprintln!("{}", e);
                            }

                            previous_command = None;
                        }
                        "exit" => return Ok(()),
                        command => {
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
                            let args = perform_expansion(&to_expand.join(" "));

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
