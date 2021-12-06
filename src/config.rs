// use crate::utils::{
//     Utils,
// };
use crate::shell::Osh;
use crate::utils::Utils;
use crate::{wdebug, werror};

use serde::Deserialize;
use std::collections::HashMap;

use console::style;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub aliases: HashMap<String, String>,
    #[serde(default = "Osh::default_prompt")]
    pub prompt: String,
    #[serde(default)]
    pub debug: bool,
    #[serde(default = "Osh::get_username")]
    pub username: String,
    #[serde(default = "Osh::get_hostname")]
    pub hostname: String,
}

impl ConfigFile {
    pub fn new() -> ConfigFile {
        let mut config_file: Option<ConfigFile> = None;

        // Try to read the configuration file
        match std::fs::File::open(Osh::perform_expansion_on_single_element("~/.shell.yaml")) {
            Ok(f) => {
                // Load aliases
                config_file = serde_yaml::from_reader(f).unwrap();
            }
            Err(_) => {
                werror!("Cannot open configuration file '~/.shell.yaml'");
            }
        };
        match config_file {
            Some(c) => {
                wdebug!(c, "Config file: {:#?}", c);
                c
            }
            None => ConfigFile {
                aliases: HashMap::new(),
                prompt: Osh::default_prompt(),
                debug: false,
                username: Osh::get_username(),
                hostname: Osh::get_hostname(),
            },
        }
    }
}
