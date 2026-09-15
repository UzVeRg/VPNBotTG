use std::{env, str::FromStr};

use reqwest::Url;
use thiserror::Error;

#[derive(Clone)]
pub struct Config {
    pub telegram_token: String,
    pub remnawave_url: String,
    pub remnawave_token: String,
    pub database_url: String,
    pub trial: TrialConfig,
    pub service: ServiceConfig,
}

#[derive(Clone)]
pub struct TrialConfig {
    pub days: i64,
    pub traffic_gib: u64,
    pub hwid_limit: u32,
    pub internal_squad_name: String,
}

#[derive(Clone)]
pub struct ServiceConfig {
    pub support_telegram_url: Url,
    pub support_email: String,

    pub phone: Option<String>,
    pub contact_address: Option<String>,

    pub docs_url: Url,
    pub user_agreement_url: Url,
    pub privacy_url: Url,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let trial = TrialConfig {
            days: required_number("TRIAL_DAYS")?,
            traffic_gib: required_number("TRIAL_TRAFFIC_GIB")?,
            hwid_limit: required_number("TRIAL_HWID_LIMIT")?,
            internal_squad_name: required("TRIAL_INTERNAL_SQUAD_NAME")?,
        };

        if trial.days <= 0 || trial.traffic_gib == 0 || trial.hwid_limit == 0 {
            return Err(ConfigError::InvalidTrialConfiguration);
        }

        let service = ServiceConfig {
            support_telegram_url: required_url(
                "SERVICE_SUPPORT_TELEGRAM_URL"
            )?,
            support_email: required(
                "SERVICE_SUPPORT_EMAIL"
            )?,
        
            phone: optional("SERVICE_PHONE"),
            contact_address: optional(
                "SERVICE_CONTACT_ADDRESS"
            ),
        
            docs_url: required_url(
                "SERVICE_DOCS_URL"
            )?,
            user_agreement_url: required_url(
                "SERVICE_USER_AGREEMENT_URL"
            )?,
            privacy_url: required_url(
                "SERVICE_PRIVACY_URL"
            )?,
        };

        Ok(Self {
            telegram_token: required("TELOXIDE_TOKEN")?,
            remnawave_url: required("REMNAWAVE_URL")?,
            remnawave_token: required("REMNAWAVE_TOKEN")?,
            database_url: required("DATABASE_URL")?,
            trial,
            service,
        })
    }
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    let value = env::var(name).map_err(|_| ConfigError::MissingVariable(name))?;

    if value.trim().is_empty() {
        return Err(ConfigError::MissingVariable(name));
    }

    Ok(value)
}

fn required_url(name: &'static str) -> Result<Url, ConfigError> {
    let value = required(name)?;

    Url::parse(&value).map_err(|_| ConfigError::InvalidVariable(name))
}

fn optional(name: &'static str) -> Option<String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn required_number<T>(name: &'static str) -> Result<T, ConfigError>
where
    T: FromStr,
{
    required(name)?
        .parse::<T>()
        .map_err(|_| ConfigError::InvalidVariable(name))
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("не задана переменная окружения {0}")]
    MissingVariable(&'static str),

    #[error("некорректное значение переменной {0}")]
    InvalidVariable(&'static str),

    #[error("некорректные параметры пробной подписки")]
    InvalidTrialConfiguration,
}
