use std::{env, net::SocketAddr, str::FromStr};

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
    pub platega: PlategaConfig,
}

#[derive(Clone)]
pub struct TrialConfig {
    pub days: i64,
    pub traffic_gib: u64,
    pub hwid_limit: u32,
    pub internal_squad_name: String,
}

#[derive(Clone)]
pub struct PlategaConfig {
    pub base_url: Url,
    pub merchant_id: Option<String>,
    pub api_key: Option<String>,
    pub webhook_bind: SocketAddr,
    pub callback_url: Url,
    pub return_url: Url,
    pub failed_url: Url,
}

impl PlategaConfig {
    pub fn is_configured(&self) -> bool {
        self.merchant_id.is_some() && self.api_key.is_some()
    }
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
            support_telegram_url: required_url("SERVICE_SUPPORT_TELEGRAM_URL")?,
            support_email: required("SERVICE_SUPPORT_EMAIL")?,

            phone: optional("SERVICE_PHONE"),
            contact_address: optional("SERVICE_CONTACT_ADDRESS"),

            docs_url: required_url("SERVICE_DOCS_URL")?,
            user_agreement_url: required_url("SERVICE_USER_AGREEMENT_URL")?,
            privacy_url: required_url("SERVICE_PRIVACY_URL")?,
        };

        let platega = PlategaConfig {
            base_url: required_url("PLATEGA_BASE_URL")?,
            merchant_id: optional("PLATEGA_MERCHANT_ID"),
            api_key: optional("PLATEGA_API_KEY"),
            webhook_bind: required("PLATEGA_WEBHOOK_BIND")?
                .parse()
                .map_err(|_| ConfigError::InvalidVariable("PLATEGA_WEBHOOK_BIND"))?,
            callback_url: required_url("PLATEGA_CALLBACK_URL")?,
            return_url: required_url("PLATEGA_RETURN_URL")?,
            failed_url: required_url("PLATEGA_FAILED_URL")?,
        };
        
        if platega.merchant_id.is_some() != platega.api_key.is_some() {
            return Err(ConfigError::IncompletePlategaCredentials);
        }

        Ok(Self {
            telegram_token: required("TELOXIDE_TOKEN")?,
            remnawave_url: required("REMNAWAVE_URL")?,
            remnawave_token: required("REMNAWAVE_TOKEN")?,
            database_url: required("DATABASE_URL")?,
            trial,
            service,
            platega,
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

    #[error("Merchant ID и API key Platega должны быть заданы одновременно")]
    IncompletePlategaCredentials,
}
