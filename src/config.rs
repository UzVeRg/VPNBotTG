use std::env;

use thiserror::Error;

#[derive(Clone)]
pub struct Config {
    pub telegram_token: String,
    pub remnawave_url: String,
    pub remnawave_token: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            telegram_token: required("TELOXIDE_TOKEN")?,
            remnawave_url: required("REMNAWAVE_URL")?,
            remnawave_token: required("REMNAWAVE_TOKEN")?,
        })
    }
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    env::var(name).map_err(|_| ConfigError::MissingVariable(name))
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("не задана переменная окружения {0}")]
    MissingVariable(&'static str),
}
