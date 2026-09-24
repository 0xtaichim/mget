use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub tmdb: TmdbConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub network: NetworkConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmdbConfig {
    #[serde(default)]
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DefaultsConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_fallback_language")]
    pub fallback_language: String,
    #[serde(default = "default_image_size")]
    pub image_size: String,
    #[serde(default)]
    pub image_language: Option<String>,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default = "default_concurrent_downloads")]
    pub concurrent_downloads: u32,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_language() -> String {
    "zh-CN".into()
}
fn default_fallback_language() -> String {
    "en".into()
}
fn default_image_size() -> String {
    "original".into()
}
fn default_concurrent_downloads() -> u32 {
    4
}
fn default_timeout() -> u64 {
    30
}

impl Default for TmdbConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
            fallback_language: default_fallback_language(),
            image_size: default_image_size(),
            image_language: None,
            overwrite: false,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            proxy: None,
            concurrent_downloads: default_concurrent_downloads(),
            timeout: default_timeout(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tmdb: TmdbConfig::default(),
            defaults: DefaultsConfig::default(),
            network: NetworkConfig::default(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mget")
            .join("config.toml")
    }

    pub fn load() -> crate::error::Result<Self> {
        let path = Self::config_path();
        let mut config = if path.exists() {
            let content =
                std::fs::read_to_string(&path).map_err(crate::error::Error::FileSystem)?;
            toml::from_str(&content).map_err(|e| crate::error::Error::Config(e.to_string()))?
        } else {
            Config::default()
        };

        // Environment variable override
        if let Ok(key) = std::env::var("TMDB_API_KEY") {
            if !key.is_empty() {
                config.tmdb.api_key = key;
            }
        }

        Ok(config)
    }

    pub fn save(&self) -> crate::error::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| crate::error::Error::Config(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn set(&mut self, key: &str, value: &str) -> crate::error::Result<()> {
        match key {
            "tmdb.api_key" => self.tmdb.api_key = value.to_string(),
            "defaults.language" => self.defaults.language = value.to_string(),
            "defaults.fallback_language" => self.defaults.fallback_language = value.to_string(),
            "defaults.image_size" => self.defaults.image_size = value.to_string(),
            "defaults.image_language" => self.defaults.image_language = Some(value.to_string()),
            "defaults.overwrite" => {
                self.defaults.overwrite = value.parse().map_err(|_| {
                    crate::error::Error::Config(format!("Invalid boolean: {}", value))
                })?;
            }
            "network.proxy" => self.network.proxy = Some(value.to_string()),
            "network.concurrent_downloads" => {
                self.network.concurrent_downloads = value.parse().map_err(|_| {
                    crate::error::Error::Config(format!("Invalid number: {}", value))
                })?;
            }
            "network.timeout" => {
                self.network.timeout = value.parse().map_err(|_| {
                    crate::error::Error::Config(format!("Invalid number: {}", value))
                })?;
            }
            _ => {
                return Err(crate::error::Error::Config(format!(
                    "Unknown config key: {}",
                    key
                )))
            }
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> crate::error::Result<String> {
        match key {
            "tmdb.api_key" => Ok(self.tmdb.api_key.clone()),
            "defaults.language" => Ok(self.defaults.language.clone()),
            "defaults.fallback_language" => Ok(self.defaults.fallback_language.clone()),
            "defaults.image_size" => Ok(self.defaults.image_size.clone()),
            "defaults.image_language" => {
                Ok(self.defaults.image_language.clone().unwrap_or_default())
            }
            "defaults.overwrite" => Ok(self.defaults.overwrite.to_string()),
            "network.proxy" => Ok(self.network.proxy.clone().unwrap_or_default()),
            "network.concurrent_downloads" => Ok(self.network.concurrent_downloads.to_string()),
            "network.timeout" => Ok(self.network.timeout.to_string()),
            _ => Err(crate::error::Error::Config(format!(
                "Unknown config key: {}",
                key
            ))),
        }
    }

    pub fn list(&self) -> Vec<(String, String)> {
        vec![
            (
                "tmdb.api_key".into(),
                if self.tmdb.api_key.is_empty() {
                    "(not set)".into()
                } else {
                    "********".into()
                },
            ),
            ("defaults.language".into(), self.defaults.language.clone()),
            (
                "defaults.fallback_language".into(),
                self.defaults.fallback_language.clone(),
            ),
            (
                "defaults.image_size".into(),
                self.defaults.image_size.clone(),
            ),
            (
                "defaults.image_language".into(),
                self.defaults.image_language.clone().unwrap_or_default(),
            ),
            (
                "defaults.overwrite".into(),
                self.defaults.overwrite.to_string(),
            ),
            (
                "network.proxy".into(),
                self.network.proxy.clone().unwrap_or("(not set)".into()),
            ),
            (
                "network.concurrent_downloads".into(),
                self.network.concurrent_downloads.to_string(),
            ),
            ("network.timeout".into(), self.network.timeout.to_string()),
        ]
    }

    pub fn api_key(&self) -> crate::error::Result<&str> {
        if self.tmdb.api_key.is_empty() {
            Err(crate::error::Error::Config(
                "TMDB API key not set. Use 'mget config set tmdb.api_key YOUR_KEY' or set TMDB_API_KEY env var.".into(),
            ))
        } else {
            Ok(&self.tmdb.api_key)
        }
    }
}
