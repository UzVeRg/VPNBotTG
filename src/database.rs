use crate::tariff::Tariff;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, postgres::PgPoolOptions};
use thiserror::Error;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

#[derive(Debug)]
pub enum TrialClaimDecision {
    Acquired,
    AlreadyUsed,
    Ineligible,
    InProgress,
}

impl Database {
    pub async fn connect(database_url: &str) -> Result<Self, DatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn get_active_tariffs(&self) -> Result<Vec<Tariff>, DatabaseError> {
        let tariffs = sqlx::query_as::<_, Tariff>(
            r#"
            SELECT
                code,
                name,
                price_kopecks,
                duration_days,
                traffic_gib,
                hwid_limit,
                internal_squad_names,
                is_active,
                sort_order
            FROM tariffs
            WHERE is_active = TRUE
            ORDER BY sort_order ASC, price_kopecks ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tariffs)
    }

    pub async fn get_tariff(&self, code: &str) -> Result<Option<Tariff>, DatabaseError> {
        let tariff = sqlx::query_as::<_, Tariff>(
            r#"
            SELECT
                code,
                name,
                price_kopecks,
                duration_days,
                traffic_gib,
                hwid_limit,
                internal_squad_names,
                is_active,
                sort_order,
                created_at,
                updated_at
            FROM tariffs
            WHERE code = $1
            "#,
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(tariff)
    }

    pub async fn ensure_user(&self, telegram_id: u64) -> Result<UserProfile, DatabaseError> {
        let telegram_id =
            i64::try_from(telegram_id).map_err(|_| DatabaseError::InvalidTelegramId)?;

        let user = sqlx::query_as::<_, UserProfile>(
            r#"
            INSERT INTO users (
                telegram_id
            )
            VALUES ($1)
            ON CONFLICT (telegram_id)
            DO UPDATE SET
                telegram_id = EXCLUDED.telegram_id
            RETURNING
                telegram_id,
                remnawave_user_id,
                registered_at,
                current_tariff_code,
                status,
                updated_at
            "#,
        )
        .bind(telegram_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn set_user_subscription(
        &self,
        telegram_id: u64,
        remnawave_user_id: i64,
        tariff_code: &str,
    ) -> Result<(), DatabaseError> {
        let telegram_id =
            i64::try_from(telegram_id).map_err(|_| DatabaseError::InvalidTelegramId)?;

        sqlx::query(
            r#"
            INSERT INTO users (
                telegram_id,
                remnawave_user_id,
                current_tariff_code
            )
            VALUES ($1, $2, $3)
            ON CONFLICT (telegram_id)
            DO UPDATE SET
                remnawave_user_id = EXCLUDED.remnawave_user_id,
                current_tariff_code = EXCLUDED.current_tariff_code,
                updated_at = NOW()
            "#,
        )
        .bind(telegram_id)
        .bind(remnawave_user_id)
        .bind(tariff_code)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn link_remnawave_user(
        &self,
        telegram_id: u64,
        remnawave_user_id: i64,
    ) -> Result<(), DatabaseError> {
        let telegram_id =
            i64::try_from(telegram_id).map_err(|_| DatabaseError::InvalidTelegramId)?;

        sqlx::query(
            r#"
            INSERT INTO users (
                telegram_id,
                remnawave_user_id
            )
            VALUES ($1, $2)
            ON CONFLICT (telegram_id)
            DO UPDATE SET
                remnawave_user_id = EXCLUDED.remnawave_user_id,
                updated_at = NOW()
            "#,
        )
        .bind(telegram_id)
        .bind(remnawave_user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn claim_trial(&self, telegram_id: i64) -> Result<TrialClaimDecision, DatabaseError> {
        let inserted = sqlx::query(
            r#"
            INSERT INTO trial_claims (
                telegram_id,
                status
            )
            VALUES ($1, 'creating')
            ON CONFLICT (telegram_id) DO NOTHING
            "#,
        )
        .bind(telegram_id)
        .execute(&self.pool)
        .await?;

        if inserted.rows_affected() == 1 {
            return Ok(TrialClaimDecision::Acquired);
        }

        let status = sqlx::query_scalar::<_, String>(
            r#"
            SELECT status
            FROM trial_claims
            WHERE telegram_id = $1
            "#,
        )
        .bind(telegram_id)
        .fetch_one(&self.pool)
        .await?;

        match status.as_str() {
            "active" => Ok(TrialClaimDecision::AlreadyUsed),

            "ineligible" => Ok(TrialClaimDecision::Ineligible),

            "failed" | "creating" => {
                let reacquired = sqlx::query(
                    r#"
                    UPDATE trial_claims
                    SET
                        status = 'creating',
                        updated_at = NOW()
                    WHERE telegram_id = $1
                      AND (
                          status = 'failed'
                          OR (
                              status = 'creating'
                              AND updated_at < NOW() - INTERVAL '5 minutes'
                          )
                      )
                    "#,
                )
                .bind(telegram_id)
                .execute(&self.pool)
                .await?;

                if reacquired.rows_affected() == 1 {
                    Ok(TrialClaimDecision::Acquired)
                } else {
                    Ok(TrialClaimDecision::InProgress)
                }
            }

            unknown => Err(DatabaseError::UnknownTrialStatus(unknown.to_owned())),
        }
    }

    pub async fn mark_trial_active(
        &self,
        telegram_id: i64,
        remnawave_user_id: i64,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE trial_claims
            SET
                status = 'active',
                remnawave_user_id = $2,
                issued_at = COALESCE(issued_at, NOW()),
                expires_at = $3,
                updated_at = NOW()
            WHERE telegram_id = $1
            "#,
        )
        .bind(telegram_id)
        .bind(remnawave_user_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn mark_trial_failed(&self, telegram_id: i64) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE trial_claims
            SET
                status = 'failed',
                updated_at = NOW()
            WHERE telegram_id = $1
              AND status = 'creating'
            "#,
        )
        .bind(telegram_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn mark_trial_ineligible(&self, telegram_id: i64) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE trial_claims
            SET
                status = 'ineligible',
                updated_at = NOW()
            WHERE telegram_id = $1
            "#,
        )
        .bind(telegram_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct UserProfile {
    pub telegram_id: i64,
    pub remnawave_user_id: Option<i64>,
    pub registered_at: DateTime<Utc>,
    pub current_tariff_code: Option<String>,
    pub status: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("ошибка PostgreSQL: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("ошибка миграции PostgreSQL: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("неизвестное состояние trial: {0}")]
    UnknownTrialStatus(String),

    #[error("Telegram ID не помещается в BIGINT")]
    InvalidTelegramId,
}
