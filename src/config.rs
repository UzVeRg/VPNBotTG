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
    pub test_telegram_ids: Vec<u64>,
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
            test_telegram_ids: parse_test_ids(optional("TEST_TELEGRAM_IDS").as_deref())?,
            support_telegram_url: required_url("SERVICE_SUPPORT_TELEGRAM_URL")?,
            support_email: required("SERVICE_SUPPORT_EMAIL")?,

            phone: optional("SERVICE_PHONE"),
            contact_address: optional("SERVICE_CONTACT_ADDRESS"),

            docs_url: required_url("SERVICE_DOCS_URL")?,
            user_agreement_url: required_url("SERVICE_USER_AGREEMENT_URL")?,
            privacy_url: required_url("SERVICE_PRIVACY_URL")?,
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

fn parse_test_ids(value: Option<&str>) -> Result<Vec<u64>, ConfigError> {
    value
        .unwrap_or("")
        .split(',')
        .filter(|id| !id.trim().is_empty())
        .map(|id| {
            id.trim()
                .parse::<u64>()
                .ok()
                .filter(|id| *id > 0)
                .ok_or(ConfigError::InvalidVariable("TEST_TELEGRAM_IDS"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_test_ids;

    #[test]
    fn test_controls_default_to_disabled() {
        assert!(parse_test_ids(None).unwrap().is_empty());
        assert!(parse_test_ids(Some("  ")).unwrap().is_empty());
    }

    #[test]
    fn test_controls_accept_multiple_explicit_users() {
        assert_eq!(parse_test_ids(Some("123, 456")).unwrap(), vec![123, 456]);
    }

    #[test]
    fn test_controls_reject_invalid_user_ids() {
        for value in ["0", "-1", "123,all", "18446744073709551616"] {
            assert!(parse_test_ids(Some(value)).is_err(), "{value}");
        }
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
