use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Tariff {
    pub code: String,
    pub name: String,
    pub price_kopecks: i64,
    pub duration_days: i64,
    pub traffic_gib: Option<u64>,
    pub hwid_limit: u32,
    pub internal_squad_names: Vec<String>,
    pub is_active: bool,
    pub sort_order: i32,
}

impl Tariff {
    pub fn price_rubles(&self) -> i64 {
        self.price_kopecks / 100
    }
}
