use crate::shell::Osh;

pub trait Alias {
    fn lookup_aliases(&self, value: &str) -> Option<String>;
    fn list_aliases(&self);
}

impl Alias for Osh {
    /// Replace the `command` with an alias if available.
    fn lookup_aliases(&self, value: &str) -> Option<String> {
        self.aliases.get(value).map(|s| s.to_string())
    }

    // TODO: List aliases for better readability
    fn list_aliases(&self) {
        for (key, value) in self.aliases.iter() {
            println!("{}: {}", key, value);
        }
    }
}
