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

#[derive(Debug)]
pub enum MarkPaymentPaidResult {
    Applied,
    AlreadyPaid,
    OrderNotPayable,
    NotFound,
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

    pub async fn create_order(&self, new_order: NewOrder) -> Result<Order, DatabaseError> {
        let telegram_id =
            i64::try_from(new_order.telegram_id).map_err(|_| DatabaseError::InvalidTelegramId)?;

        let price_kopecks =
            i64::try_from(new_order.price_kopecks).map_err(|_| DatabaseError::InvalidOrderValue)?;

        let duration_days =
            i32::try_from(new_order.duration_days).map_err(|_| DatabaseError::InvalidOrderValue)?;

        let traffic_gib = new_order
            .traffic_gib
            .map(i64::try_from)
            .transpose()
            .map_err(|_| DatabaseError::InvalidOrderValue)?;

        let hwid_limit =
            i32::try_from(new_order.hwid_limit).map_err(|_| DatabaseError::InvalidOrderValue)?;

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE orders
            SET
                status = 'cancelled',
                updated_at = NOW()
            WHERE telegram_id = $1
              AND status = 'pending'
            "#,
        )
        .bind(telegram_id)
        .execute(&mut *tx)
        .await?;

        let order = sqlx::query_as::<_, Order>(
            r#"
            INSERT INTO orders (
                telegram_id,
                tariff_code,
                tariff_name,
                tariff_description,
                price_kopecks,
                currency,
                duration_days,
                traffic_gib,
                hwid_limit,
                internal_squad_names,
                status
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                'RUB',
                $6,
                $7,
                $8,
                $9,
                'pending'
            )
            RETURNING
                id,
                telegram_id,
                tariff_code,
                tariff_name,
                tariff_description,
                price_kopecks,
                currency,
                duration_days,
                traffic_gib,
                hwid_limit,
                internal_squad_names,
                status,
                created_at,
                updated_at
            "#,
        )
        .bind(telegram_id)
        .bind(&new_order.tariff_code)
        .bind(&new_order.tariff_name)
        .bind(&new_order.tariff_description)
        .bind(price_kopecks)
        .bind(duration_days)
        .bind(traffic_gib)
        .bind(hwid_limit)
        .bind(&new_order.internal_squad_names)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(order)
    }

    pub async fn get_order(&self, order_id: i64) -> Result<Option<Order>, DatabaseError> {
        let order = sqlx::query_as::<_, Order>(
            r#"
            SELECT
                id,
                telegram_id,
                tariff_code,
                tariff_name,
                tariff_description,
                price_kopecks,
                currency,
                duration_days,
                traffic_gib,
                hwid_limit,
                internal_squad_names,
                status,
                created_at,
                updated_at
            FROM orders
            WHERE id = $1
            "#,
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(order)
    }

    pub async fn get_user_pending_order(
        &self,
        telegram_id: u64,
    ) -> Result<Option<Order>, DatabaseError> {
        let telegram_id =
            i64::try_from(telegram_id).map_err(|_| DatabaseError::InvalidTelegramId)?;

        let order = sqlx::query_as::<_, Order>(
            r#"
            SELECT
                id,
                telegram_id,
                tariff_code,
                tariff_name,
                tariff_description,
                price_kopecks,
                currency,
                duration_days,
                traffic_gib,
                hwid_limit,
                internal_squad_names,
                status,
                created_at,
                updated_at
            FROM orders
            WHERE telegram_id = $1
              AND status = 'pending'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(telegram_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(order)
    }

    pub async fn create_payment(&self, new_payment: NewPayment) -> Result<Payment, DatabaseError> {
        let payment = sqlx::query_as::<_, Payment>(
            r#"
            INSERT INTO payments (
                order_id,
                provider,
                idempotency_key,
                amount_kopecks,
                currency,
                status
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                'created'
            )
            RETURNING
                id,
                order_id,
                provider,
                provider_payment_id,
                idempotency_key,
                amount_kopecks,
                currency,
                status,
                payment_url,
                created_at,
                updated_at,
                paid_at
            "#,
        )
        .bind(new_payment.order_id)
        .bind(&new_payment.provider)
        .bind(&new_payment.idempotency_key)
        .bind(new_payment.amount_kopecks)
        .bind(&new_payment.currency)
        .fetch_one(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn get_payment(&self, payment_id: i64) -> Result<Option<Payment>, DatabaseError> {
        let payment = sqlx::query_as::<_, Payment>(
            r#"
            SELECT
                id,
                order_id,
                provider,
                provider_payment_id,
                idempotency_key,
                amount_kopecks,
                currency,
                status,
                payment_url,
                created_at,
                updated_at,
                paid_at
            FROM payments
            WHERE id = $1
            "#,
        )
        .bind(payment_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn get_payment_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<Payment>, DatabaseError> {
        let payment = sqlx::query_as::<_, Payment>(
            r#"
            SELECT
                id,
                order_id,
                provider,
                provider_payment_id,
                idempotency_key,
                amount_kopecks,
                currency,
                status,
                payment_url,
                created_at,
                updated_at,
                paid_at
            FROM payments
            WHERE idempotency_key = $1
            "#,
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(payment)
    }

    pub async fn get_or_create_payment(
        &self,
        order: &Order,
        provider: &str,
    ) -> Result<Payment, DatabaseError> {
        let idempotency_key = format!("order:{}:{provider}", order.id);

        if let Some(payment) = self
            .get_payment_by_idempotency_key(&idempotency_key)
            .await?
        {
            return Ok(payment);
        }

        self.create_payment(NewPayment {
            order_id: order.id,
            provider: provider.to_owned(),
            amount_kopecks: order.price_kopecks,
            currency: order.currency.clone(),
            idempotency_key,
        })
        .await
    }

    pub async fn mark_payment_paid(
        &self,
        payment_id: i64,
    ) -> Result<MarkPaymentPaidResult, DatabaseError> {
        let mut tx = self.pool.begin().await?;

        let payment = sqlx::query_as::<_, Payment>(
            r#"
            SELECT
                id,
                order_id,
                provider,
                provider_payment_id,
                idempotency_key,
                amount_kopecks,
                currency,
                status,
                payment_url,
                created_at,
                updated_at,
                paid_at
            FROM payments
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(payment_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(payment) = payment else {
            tx.rollback().await?;

            return Ok(MarkPaymentPaidResult::NotFound);
        };

        if payment.status == "paid" {
            tx.rollback().await?;

            return Ok(MarkPaymentPaidResult::AlreadyPaid);
        }

        let order_status = sqlx::query_scalar::<_, String>(
            r#"
            SELECT status
            FROM orders
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(payment.order_id)
        .fetch_one(&mut *tx)
        .await?;

        if order_status != "pending" {
            tx.rollback().await?;

            return Ok(MarkPaymentPaidResult::OrderNotPayable);
        }

        sqlx::query(
            r#"
            UPDATE payments
            SET
                status = 'paid',
                paid_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(payment.id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE orders
            SET
                status = 'paid',
                updated_at = NOW()
            WHERE id = $1
              AND status = 'pending'
            "#,
        )
        .bind(payment.order_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(MarkPaymentPaidResult::Applied)
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

#[derive(Debug, Clone)]
pub struct NewOrder {
    pub telegram_id: u64,

    pub tariff_code: String,
    pub tariff_name: String,
    pub tariff_description: String,

    pub price_kopecks: u64,
    pub duration_days: i64,
    pub traffic_gib: Option<u64>,
    pub hwid_limit: u32,

    pub internal_squad_names: Vec<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Order {
    pub id: i64,

    pub telegram_id: i64,

    pub tariff_code: String,
    pub tariff_name: String,
    pub tariff_description: String,

    pub price_kopecks: i64,
    pub currency: String,

    pub duration_days: i32,
    pub traffic_gib: Option<i64>,
    pub hwid_limit: i32,

    pub internal_squad_names: Vec<String>,

    pub status: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Payment {
    pub id: i64,

    pub order_id: i64,

    pub provider: String,
    pub provider_payment_id: Option<String>,
    pub idempotency_key: String,

    pub amount_kopecks: i64,
    pub currency: String,

    pub status: String,

    pub payment_url: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewPayment {
    pub order_id: i64,

    pub provider: String,

    pub amount_kopecks: i64,
    pub currency: String,

    pub idempotency_key: String,
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

    #[error("значение заказа не помещается в тип PostgreSQL")]
    InvalidOrderValue,
}
