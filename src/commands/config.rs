use clap::ValueEnum;
use serde_json::{Map, Value};

use crate::cli::ConfigAction;
use crate::config::{Config, ConfigKey};
use crate::error::Result;

pub fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Set { key, value } => {
            // Loads without env overrides so an env-provided key is never persisted.
            let mut config = Config::load_file()?;
            config.set(key, &value)?;
            config.save()?;
            eprintln!("Set {} = {}", key.name(), config.display(key));
        }
        ConfigAction::Get { key } => println!("{}", Config::load()?.get(key)),
        ConfigAction::List => {
            let config = Config::load()?;
            let map: Map<String, Value> = ConfigKey::value_variants()
                .iter()
                .map(|&key| (key.name().to_string(), Value::String(config.display(key))))
                .collect();
            super::print_json(&map)?;
        }
        ConfigAction::Reset => {
            Config::default().save()?;
            eprintln!("Configuration reset to defaults");
        }
        ConfigAction::Path => println!("{}", Config::path().display()),
    }
    Ok(())
}
