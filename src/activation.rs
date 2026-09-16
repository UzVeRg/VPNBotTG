use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

use crate::{
    database::{Activation, Database, DatabaseError, Order},
    remnawave::{
        CreateUserRequest, RemnawaveClient, RemnawaveError, RemnawaveUser, UpdateUserRequest,
    },
};

const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Clone)]
pub struct ActivationService {
    database: Database,
    remnawave: RemnawaveClient,
}

#[derive(Debug)]
pub enum ActivationResult {
    Created(RemnawaveUser),
    Updated(RemnawaveUser),
    AlreadyActivated,
}

impl ActivationService {
    pub fn new(database: Database, remnawave: RemnawaveClient) -> Self {
        Self {
            database,
            remnawave,
        }
    }

    pub async fn activate_order(&self, order_id: i64) -> Result<ActivationResult, ActivationError> {
        let order = self
            .database
            .get_order(order_id)
            .await?
            .ok_or(ActivationError::OrderNotFound(order_id))?;

        if order.status != "paid" {
            return Err(ActivationError::OrderNotPaid(order.status.clone()));
        }

        let activation = self.database.get_or_create_activation(order.id).await?;

        if activation.status == "succeeded" {
            return Ok(ActivationResult::AlreadyActivated);
        }

        self.process_activation(&order, activation).await
    }

    async fn process_activation(
        &self,
        order: &Order,
        activation: Activation,
    ) -> Result<ActivationResult, ActivationError> {
        self.database
            .mark_activation_processing(activation.id)
            .await?;

        match self.process_activation_inner(order, &activation).await {
            Ok(result) => Ok(result),

            Err(error) => {
                if let Err(database_error) = self
                    .database
                    .mark_activation_failed(activation.id, &error.to_string())
                    .await
                {
                    tracing::error!(
                        activation_id = activation.id,
                        error = %database_error,
                        "Не удалось отметить activation как failed"
                    );
                }

                Err(error)
            }
        }
    }

    async fn process_activation_inner(
        &self,
        order: &Order,
        activation: &Activation,
    ) -> Result<ActivationResult, ActivationError> {
        let telegram_id =
            u64::try_from(order.telegram_id).map_err(|_| ActivationError::InvalidTelegramId)?;

        let squad_uuids = self.resolve_squads(&order.internal_squad_names).await?;

        let traffic_limit_bytes = traffic_limit_bytes(order.traffic_gib)?;

        let existing_user = self.find_existing_user(telegram_id).await?;

        match existing_user {
            Some(user) => {
                self.update_existing_user(
                    order,
                    activation,
                    telegram_id,
                    user,
                    squad_uuids,
                    traffic_limit_bytes,
                )
                .await
            }

            None => {
                self.create_new_user(
                    order,
                    activation,
                    telegram_id,
                    squad_uuids,
                    traffic_limit_bytes,
                )
                .await
            }
        }
    }

    async fn find_existing_user(
        &self,
        telegram_id: u64,
    ) -> Result<Option<RemnawaveUser>, ActivationError> {
        let profile = self.database.ensure_user(telegram_id).await?;

        if let Some(remnawave_user_id) = profile.remnawave_user_id {
            if let Some(user) = self.remnawave.get_user_by_id(remnawave_user_id).await? {
                return Ok(Some(user));
            }

            tracing::warn!(
                telegram_id,
                remnawave_user_id,
                "Связанный пользователь не найден в Remnawave"
            );
        }

        let mut users = self
            .remnawave
            .find_users_by_telegram_id(telegram_id)
            .await?;

        match users.len() {
            0 => Ok(None),

            1 => Ok(users.pop()),

            count => Err(ActivationError::MultipleRemnawaveUsers { telegram_id, count }),
        }
    }

    async fn update_existing_user(
        &self,
        order: &Order,
        activation: &Activation,
        telegram_id: u64,
        user: RemnawaveUser,
        squad_uuids: Vec<String>,
        traffic_limit_bytes: u64,
    ) -> Result<ActivationResult, ActivationError> {
        let target_expire_at = match activation.target_expire_at {
            Some(expire_at) => expire_at,

            None => {
                let current_expire_at = parse_expiration(&user)?;

                let base_expire_at = if current_expire_at > Utc::now() {
                    current_expire_at
                } else {
                    Utc::now()
                };

                base_expire_at + Duration::days(i64::from(order.duration_days))
            }
        };

        let target_traffic_limit_bytes =
            i64::try_from(traffic_limit_bytes).map_err(|_| ActivationError::InvalidTrafficLimit)?;

        let activation = self
            .database
            .set_activation_plan(
                activation.id,
                "update",
                Some(user.id),
                target_expire_at,
                target_traffic_limit_bytes,
                order.hwid_limit,
                &squad_uuids,
            )
            .await?;

        let request = UpdateUserRequest {
            id: user.id,

            status: String::from("ACTIVE"),

            traffic_limit_bytes,

            traffic_limit_strategy: String::from("NO_RESET"),

            expire_at: target_expire_at.to_rfc3339(),

            description: format!("Платная подписка SilentOkVPN, заказ #{}", order.id),

            tag: String::from("PAID"),

            telegram_id,

            hwid_device_limit: u32::try_from(order.hwid_limit)
                .map_err(|_| ActivationError::InvalidHwidLimit)?,

            active_internal_squads: squad_uuids,
        };

        let updated_user = self.remnawave.update_user(&request).await?;

        self.database
            .set_user_subscription(telegram_id, updated_user.id, &order.tariff_code)
            .await?;

        self.database
            .mark_activation_succeeded(activation.id, updated_user.id)
            .await?;

        Ok(ActivationResult::Updated(updated_user))
    }

    async fn create_new_user(
        &self,
        order: &Order,
        activation: &Activation,
        telegram_id: u64,
        squad_uuids: Vec<String>,
        traffic_limit_bytes: u64,
    ) -> Result<ActivationResult, ActivationError> {
        let target_expire_at = activation
            .target_expire_at
            .unwrap_or_else(|| Utc::now() + Duration::days(i64::from(order.duration_days)));

        let target_traffic_limit_bytes =
            i64::try_from(traffic_limit_bytes).map_err(|_| ActivationError::InvalidTrafficLimit)?;

        let activation = self
            .database
            .set_activation_plan(
                activation.id,
                "create",
                None,
                target_expire_at,
                target_traffic_limit_bytes,
                order.hwid_limit,
                &squad_uuids,
            )
            .await?;

        let request = CreateUserRequest {
            username: format!("user_{telegram_id}"),

            status: String::from("ACTIVE"),

            traffic_limit_bytes,

            traffic_limit_strategy: String::from("NO_RESET"),

            expire_at: target_expire_at.to_rfc3339(),

            description: format!("Платная подписка SilentOkVPN, заказ #{}", order.id),

            tag: String::from("PAID"),

            telegram_id,

            hwid_device_limit: u32::try_from(order.hwid_limit)
                .map_err(|_| ActivationError::InvalidHwidLimit)?,

            active_internal_squads: squad_uuids,
        };

        let created_user = match self.remnawave.create_user(&request).await {
            Ok(user) => user,

            Err(create_error) => {
                let users = self
                    .remnawave
                    .find_users_by_telegram_id(telegram_id)
                    .await?;

                if let Some(user) = users
                    .into_iter()
                    .find(|user| user.tag.as_deref() == Some("PAID"))
                {
                    tracing::warn!(
                        telegram_id,
                        remnawave_user_id = user.id,
                        "POST мог завершиться успешно; \
                             пользователь восстановлен"
                    );

                    user
                } else {
                    return Err(create_error.into());
                }
            }
        };

        self.database
            .set_user_subscription(telegram_id, created_user.id, &order.tariff_code)
            .await?;

        self.database
            .mark_activation_succeeded(activation.id, created_user.id)
            .await?;

        Ok(ActivationResult::Created(created_user))
    }

    async fn resolve_squads(&self, squad_names: &[String]) -> Result<Vec<String>, ActivationError> {
        let mut uuids = Vec::with_capacity(squad_names.len());

        for name in squad_names {
            let squad = self
                .remnawave
                .find_internal_squad_by_name(name)
                .await?
                .ok_or_else(|| ActivationError::InternalSquadNotFound(name.clone()))?;

            uuids.push(squad.uuid);
        }

        Ok(uuids)
    }
}

fn traffic_limit_bytes(traffic_gib: Option<i64>) -> Result<u64, ActivationError> {
    match traffic_gib {
        None => Ok(0),

        Some(gib) => {
            let gib = u64::try_from(gib).map_err(|_| ActivationError::InvalidTrafficLimit)?;

            gib.checked_mul(GIB)
                .ok_or(ActivationError::InvalidTrafficLimit)
        }
    }
}

fn parse_expiration(user: &RemnawaveUser) -> Result<DateTime<Utc>, ActivationError> {
    Ok(DateTime::parse_from_rfc3339(&user.expire_at)?.with_timezone(&Utc))
}

#[derive(Debug, Error)]
pub enum ActivationError {
    #[error(transparent)]
    Database(#[from] DatabaseError),

    #[error(transparent)]
    Remnawave(#[from] RemnawaveError),

    #[error("заказ #{0} не найден")]
    OrderNotFound(i64),

    #[error("заказ не оплачен; текущий статус: {0}")]
    OrderNotPaid(String),

    #[error("Telegram ID не помещается в u64")]
    InvalidTelegramId,

    #[error("некорректный лимит трафика")]
    InvalidTrafficLimit,

    #[error("некорректный лимит устройств")]
    InvalidHwidLimit,

    #[error("Internal Squad '{0}' не найден")]
    InternalSquadNotFound(String),

    #[error(
        "для Telegram ID {telegram_id} найдено \
         несколько пользователей Remnawave: {count}"
    )]
    MultipleRemnawaveUsers { telegram_id: u64, count: usize },

    #[error("некорректная дата окончания подписки: {0}")]
    InvalidExpiration(#[from] chrono::ParseError),
}
