use crate::config::ConfigFile;

use std::collections::HashMap;

/// Replace the `command` with an alias if available.
pub fn lookup_aliases(config: &ConfigFile, value: &str) -> Option<String> {
    config.aliases.get(value).map(|s| s.to_string())
}

// TODO: List aliases for better readability
pub fn list_aliases(aliases: &HashMap<String, String>) {
    for (key, value) in aliases.iter() {
        println!("{}: {}", key, value);
    }
}
