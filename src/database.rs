use chrono::{DateTime, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
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

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("ошибка PostgreSQL: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("ошибка миграции PostgreSQL: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("неизвестное состояние trial: {0}")]
    UnknownTrialStatus(String),
}
