use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::env;
use std::io::Result;
use std::io::{stdin, stdout, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

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

                            if let Err(e) = env::set_current_dir(Path::new(new_dir)) {
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

                            let output = Command::new(command)
                                .args(args)
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
