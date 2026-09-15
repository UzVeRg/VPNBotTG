use serde::Deserialize;
use std::{collections::HashSet, fs, path::Path};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct Tariff {
    pub code: String,
    pub name: String,
    pub description: String,
    pub price_kopecks: u64,
    pub duration_days: i64,
    pub traffic_gib: Option<u64>,
    pub hwid_limit: u32,
    pub internal_squad_names: Vec<String>,
    pub is_active: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
pub struct TariffCatalog {
    tariffs: Vec<Tariff>,
}

impl TariffCatalog {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, TariffError> {
        let contents = fs::read_to_string(path)?;
        let mut tariffs: Vec<Tariff> = serde_json::from_str(&contents)?;

        validate_tariffs(&tariffs)?;

        tariffs.sort_by_key(|tariff| (tariff.sort_order, tariff.price_kopecks));

        Ok(Self { tariffs })
    }

    pub fn active(&self) -> impl Iterator<Item = &Tariff> {
        self.tariffs.iter().filter(|tariff| tariff.is_active)
    }

    pub fn get(&self, code: &str) -> Option<&Tariff> {
        self.tariffs.iter().find(|tariff| tariff.code == code)
    }

    pub fn get_active(&self, code: &str) -> Option<&Tariff> {
        self.tariffs
            .iter()
            .find(|tariff| tariff.code == code && tariff.is_active)
    }

    pub fn len(&self) -> usize {
        self.tariffs.len()
    }
}

impl Tariff {
    pub fn price_rubles(&self) -> u64 {
        self.price_kopecks / 100
    }
}

fn validate_tariffs(tariffs: &[Tariff]) -> Result<(), TariffError> {
    let mut codes = HashSet::new();

    for tariff in tariffs {
        if tariff.code.is_empty()
            || tariff.code.len() > 32
            || !tariff.code.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '_' || character == '-'
            })
        {
            return Err(TariffError::Invalid(format!(
                "некорректный код тарифа '{}'",
                tariff.code
            )));
        }

        if !codes.insert(tariff.code.clone()) {
            return Err(TariffError::Invalid(format!(
                "дублирующийся код тарифа '{}'",
                tariff.code
            )));
        }

        if tariff.name.trim().is_empty() {
            return Err(TariffError::Invalid(format!(
                "у тарифа '{}' отсутствует название",
                tariff.code
            )));
        }

        if tariff.description.trim().is_empty() {
            return Err(TariffError::Invalid(format!(
                "у тарифа '{}' отсутствует описание",
                tariff.code
            )));
        }

        if tariff.price_kopecks == 0 {
            return Err(TariffError::Invalid(format!(
                "у тарифа '{}' цена должна быть больше нуля",
                tariff.code
            )));
        }

        if tariff.duration_days <= 0 {
            return Err(TariffError::Invalid(format!(
                "у тарифа '{}' срок должен быть больше нуля",
                tariff.code
            )));
        }

        if tariff.traffic_gib == Some(0) {
            return Err(TariffError::Invalid(format!(
                "у тарифа '{}' лимит трафика не может быть равен нулю",
                tariff.code
            )));
        }

        if tariff.hwid_limit == 0 {
            return Err(TariffError::Invalid(format!(
                "у тарифа '{}' должен быть хотя бы один слот устройства",
                tariff.code
            )));
        }

        if tariff.internal_squad_names.is_empty()
            || tariff
                .internal_squad_names
                .iter()
                .any(|name| name.trim().is_empty())
        {
            return Err(TariffError::Invalid(format!(
                "у тарифа '{}' некорректно указаны Internal Squads",
                tariff.code
            )));
        }
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum TariffError {
    #[error("не удалось прочитать файл тарифов: {0}")]
    Io(#[from] std::io::Error),

    #[error("некорректный JSON тарифов: {0}")]
    Json(#[from] serde_json::Error),

    #[error("некорректная конфигурация тарифов: {0}")]
    Invalid(String),
}
