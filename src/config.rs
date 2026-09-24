use std::path::PathBuf;

use clap::ValueEnum;
use clap::builder::PossibleValue;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::fsutil;

const API_KEY_ENV: &str = "TMDB_API_KEY";
const NOT_SET: &str = "(not set)";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub tmdb: TmdbConfig,
    pub defaults: DefaultsConfig,
    pub network: NetworkConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TmdbConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DefaultsConfig {
    pub language: String,
    pub fallback_language: String,
    pub image_size: String,
    pub image_language: Option<String>,
    pub overwrite: bool,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            language: "zh-CN".into(),
            fallback_language: "en".into(),
            image_size: "original".into(),
            image_language: None,
            overwrite: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub proxy: Option<String>,
    pub concurrent_downloads: u32,
    pub timeout: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            proxy: None,
            concurrent_downloads: 4,
            timeout: 30,
        }
    }
}

/// Every user-settable configuration key, addressed by its dotted name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKey {
    TmdbApiKey,
    Language,
    FallbackLanguage,
    ImageSize,
    ImageLanguage,
    Overwrite,
    Proxy,
    ConcurrentDownloads,
    Timeout,
}

impl ConfigKey {
    pub fn name(self) -> &'static str {
        match self {
            ConfigKey::TmdbApiKey => "tmdb.api_key",
            ConfigKey::Language => "defaults.language",
            ConfigKey::FallbackLanguage => "defaults.fallback_language",
            ConfigKey::ImageSize => "defaults.image_size",
            ConfigKey::ImageLanguage => "defaults.image_language",
            ConfigKey::Overwrite => "defaults.overwrite",
            ConfigKey::Proxy => "network.proxy",
            ConfigKey::ConcurrentDownloads => "network.concurrent_downloads",
            ConfigKey::Timeout => "network.timeout",
        }
    }

    pub fn is_secret(self) -> bool {
        self == ConfigKey::TmdbApiKey
    }
}

impl ValueEnum for ConfigKey {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            ConfigKey::TmdbApiKey,
            ConfigKey::Language,
            ConfigKey::FallbackLanguage,
            ConfigKey::ImageSize,
            ConfigKey::ImageLanguage,
            ConfigKey::Overwrite,
            ConfigKey::Proxy,
            ConfigKey::ConcurrentDownloads,
            ConfigKey::Timeout,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(self.name()))
    }
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mget")
            .join("config.toml")
    }

    /// Loads the config file (or defaults), then applies environment overrides.
    pub fn load() -> Result<Self> {
        let mut config = Self::load_file()?;
        if let Ok(key) = std::env::var(API_KEY_ENV)
            && !key.is_empty()
        {
            config.tmdb.api_key = key;
        }
        Ok(config)
    }

    /// Loads only the persisted file, without environment overrides, so that
    /// `config set` never writes an env-provided key to disk.
    pub fn load_file() -> Result<Self> {
        match std::fs::read_to_string(Self::path()) {
            Ok(content) => toml::from_str(&content).map_err(|e| Error::Config(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        let content = toml::to_string_pretty(self).map_err(|e| Error::Config(e.to_string()))?;
        fsutil::write_atomic(&path, content.as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    pub fn get(&self, key: ConfigKey) -> String {
        match key {
            ConfigKey::TmdbApiKey => self.tmdb.api_key.clone(),
            ConfigKey::Language => self.defaults.language.clone(),
            ConfigKey::FallbackLanguage => self.defaults.fallback_language.clone(),
            ConfigKey::ImageSize => self.defaults.image_size.clone(),
            ConfigKey::ImageLanguage => self.defaults.image_language.clone().unwrap_or_default(),
            ConfigKey::Overwrite => self.defaults.overwrite.to_string(),
            ConfigKey::Proxy => self.network.proxy.clone().unwrap_or_default(),
            ConfigKey::ConcurrentDownloads => self.network.concurrent_downloads.to_string(),
            ConfigKey::Timeout => self.network.timeout.to_string(),
        }
    }

    /// Value suitable for display: secrets are masked and empty values shown as "(not set)".
    pub fn display(&self, key: ConfigKey) -> String {
        let value = self.get(key);
        match (value.is_empty(), key.is_secret()) {
            (true, _) => NOT_SET.into(),
            (false, true) => "********".into(),
            (false, false) => value,
        }
    }

    /// Sets a key from its string form. An empty value clears optional keys.
    pub fn set(&mut self, key: ConfigKey, value: &str) -> Result<()> {
        let optional = |v: &str| (!v.is_empty()).then(|| v.to_string());
        match key {
            ConfigKey::TmdbApiKey => self.tmdb.api_key = value.trim().to_string(),
            ConfigKey::Language => self.defaults.language = non_empty(key, value)?,
            ConfigKey::FallbackLanguage => self.defaults.fallback_language = non_empty(key, value)?,
            ConfigKey::ImageSize => self.defaults.image_size = non_empty(key, value)?,
            ConfigKey::ImageLanguage => self.defaults.image_language = optional(value),
            ConfigKey::Overwrite => self.defaults.overwrite = parse(key, value)?,
            ConfigKey::Proxy => self.network.proxy = optional(value),
            ConfigKey::ConcurrentDownloads => {
                self.network.concurrent_downloads = positive(key, parse(key, value)?)?;
            }
            ConfigKey::Timeout => self.network.timeout = positive(key, parse(key, value)?)?,
        }
        Ok(())
    }

    pub fn api_key(&self) -> Result<&str> {
        if self.tmdb.api_key.is_empty() {
            return Err(Error::Config(format!(
                "TMDB API key not set. Use 'mget config set tmdb.api_key YOUR_KEY' or set {API_KEY_ENV} env var."
            )));
        }
        Ok(&self.tmdb.api_key)
    }

    /// Preferred image languages, most preferred first.
    pub fn image_languages(&self) -> Vec<String> {
        self.defaults
            .image_language
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }
}

fn non_empty(key: ConfigKey, value: &str) -> Result<String> {
    if value.trim().is_empty() {
        return Err(Error::Config(format!("{} cannot be empty", key.name())));
    }
    Ok(value.trim().to_string())
}

fn parse<T: std::str::FromStr>(key: ConfigKey, value: &str) -> Result<T> {
    value
        .trim()
        .parse()
        .map_err(|_| Error::Config(format!("Invalid value for {}: {value}", key.name())))
}

fn positive<T: PartialOrd + Default>(key: ConfigKey, value: T) -> Result<T> {
    if value <= T::default() {
        return Err(Error::Config(format!(
            "{} must be greater than 0",
            key.name()
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set_round_trip() {
        let mut config = Config::default();
        for key in ConfigKey::value_variants() {
            let sample = match key {
                ConfigKey::Overwrite => "true",
                ConfigKey::ConcurrentDownloads | ConfigKey::Timeout => "7",
                _ => "value",
            };
            config.set(*key, sample).unwrap();
            assert_eq!(config.get(*key), sample, "{}", key.name());
        }
    }

    #[test]
    fn rejects_invalid_values() {
        let mut config = Config::default();
        assert!(config.set(ConfigKey::ConcurrentDownloads, "0").is_err());
        assert!(config.set(ConfigKey::Timeout, "abc").is_err());
        assert!(config.set(ConfigKey::Overwrite, "yes").is_err());
        assert!(config.set(ConfigKey::Language, " ").is_err());
    }

    #[test]
    fn empty_value_clears_optional_keys() {
        let mut config = Config::default();
        config.set(ConfigKey::Proxy, "http://p:1").unwrap();
        config.set(ConfigKey::Proxy, "").unwrap();
        assert_eq!(config.network.proxy, None);
        assert_eq!(config.display(ConfigKey::Proxy), NOT_SET);
    }

    #[test]
    fn masks_secrets() {
        let mut config = Config::default();
        assert_eq!(config.display(ConfigKey::TmdbApiKey), NOT_SET);
        config.set(ConfigKey::TmdbApiKey, "abc").unwrap();
        assert_eq!(config.display(ConfigKey::TmdbApiKey), "********");
    }

    #[test]
    fn parses_image_languages() {
        let mut config = Config::default();
        config
            .set(ConfigKey::ImageLanguage, " zh, en ,,ja")
            .unwrap();
        assert_eq!(config.image_languages(), ["zh", "en", "ja"]);
    }

    #[test]
    fn partial_file_uses_defaults() {
        let config: Config = toml::from_str("[defaults]\nlanguage = \"en\"\n").unwrap();
        assert_eq!(config.defaults.language, "en");
        assert_eq!(config.defaults.image_size, "original");
        assert_eq!(config.network.concurrent_downloads, 4);
    }
}
