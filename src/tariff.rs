use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Tariff {
    pub code: String,
    pub name: String,
    pub price_kopecks: i64,
    pub duration_days: i32,
    pub traffic_gib: Option<i64>,
    pub hwid_limit: i32,
    pub internal_squad_names: Vec<String>,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Tariff {
    pub fn price_rubles(&self) -> i64 {
        self.price_kopecks / 100
    }
}
