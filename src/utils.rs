use crate::shell::Osh;

use std::env;
use std::fs;
use std::io;
use std::io::Read;

pub trait Utils {
    fn perform_expansion_on_single_element(value: &str) -> String;
    fn perform_wildcard_expansion(value: &str) -> Option<Vec<String>>;
    fn default_prompt() -> String;
    fn get_username() -> String;
    fn get_hostname() -> String;
}

impl Utils for Osh {
    /// Perform environment variable expansion.
    fn perform_expansion_on_single_element(value: &str) -> String {
        // Expand tilde character
        if !value.contains('$') {
            if value.contains('~') && env::var("HOME").is_ok() {
                return value.replace('~', &env::var("HOME").unwrap());
            }
            return value.into();
        }

        // Replace environment variable
        let mut result = String::new();
        if let Some(key) = value.strip_prefix('$') {
            // Lookup for the given value in the environment
            result = match env::var(key) {
                Ok(x) => x,
                Err(_) => "".into(),
            };
        }

        result
    }

    fn perform_wildcard_expansion(value: &str) -> Option<Vec<String>> {
        let mut result = Vec::new();

        if !value.contains('*') {
            return None;
        }

        let mut dir = ".";
        // If the user supplied a directory to perform wildcard expansion into, fetch the directory name
        if value.contains("/*") {
            if value.eq("/*") {
                dir = "/";
            } else {
                dir = value.split("/*").collect::<Vec<&str>>().get(0).expect(
                    "Failed to identify directory where wildcard expansion must be performed",
                );
            }
        }

        let mut entries = fs::read_dir(dir)
            .unwrap()
            .map(|res| res.map(|e| e.path().to_str().unwrap().to_string()))
            .collect::<Result<Vec<String>, io::Error>>()
            .unwrap();

        // The order in which `read_dir` returns entries is not guaranteed. If reproducible
        // ordering is required the entries should be explicitly sorted.
        entries.sort();

        for entry in entries.iter() {
            result.push(entry.to_string());
        }
        Some(result)
    }

    fn default_prompt() -> String {
        "$".to_string()
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
}
