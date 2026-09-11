use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

use crate::{
    config::TrialConfig,
    database::{Database, DatabaseError, TrialClaimDecision},
    remnawave::{CreateUserRequest, RemnawaveClient, RemnawaveError, RemnawaveUser},
};

const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Clone)]
pub struct TrialService {
    database: Database,
    remnawave: RemnawaveClient,
    config: TrialConfig,
    internal_squad_uuid: String,
}

#[derive(Debug)]
pub enum TrialIssueResult {
    Created(RemnawaveUser),
    Recovered(RemnawaveUser),
    AlreadyUsed,
    Ineligible,
    InProgress,
}

impl TrialService {
    pub async fn new(
        database: Database,
        remnawave: RemnawaveClient,
        config: TrialConfig,
    ) -> Result<Self, TrialError> {
        let squad = remnawave
            .find_internal_squad_by_name(&config.internal_squad_name)
            .await?
            .ok_or_else(|| TrialError::InternalSquadNotFound(config.internal_squad_name.clone()))?;

        tracing::info!(
            squad = %squad.name,
            uuid = %squad.uuid,
            "Internal Squad для trial найден"
        );

        Ok(Self {
            database,
            remnawave,
            config,
            internal_squad_uuid: squad.uuid,
        })
    }

    pub fn config(&self) -> &TrialConfig {
        &self.config
    }

    pub async fn issue_trial(&self, telegram_id: u64) -> Result<TrialIssueResult, TrialError> {
        let database_telegram_id =
            i64::try_from(telegram_id).map_err(|_| TrialError::InvalidTelegramId)?;

        match self.database.claim_trial(database_telegram_id).await? {
            TrialClaimDecision::AlreadyUsed => {
                return Ok(TrialIssueResult::AlreadyUsed);
            }

            TrialClaimDecision::Ineligible => {
                return Ok(TrialIssueResult::Ineligible);
            }

            TrialClaimDecision::InProgress => {
                return Ok(TrialIssueResult::InProgress);
            }

            TrialClaimDecision::Acquired => {}
        }

        let users = match self.remnawave.find_users_by_telegram_id(telegram_id).await {
            Ok(users) => users,

            Err(error) => {
                self.mark_failed_best_effort(database_telegram_id).await;

                return Err(error.into());
            }
        };

        if let Some(existing_trial) = users
            .iter()
            .find(|user| user.tag.as_deref() == Some("TRIAL"))
        {
            let expires_at = parse_expiration(existing_trial)?;

            self.database
                .mark_trial_active(database_telegram_id, existing_trial.id, expires_at)
                .await?;

            if existing_trial.status == "ACTIVE" && expires_at > Utc::now() {
                return Ok(TrialIssueResult::Recovered(existing_trial.clone()));
            }

            return Ok(TrialIssueResult::AlreadyUsed);
        }

        if !users.is_empty() {
            self.database
                .mark_trial_ineligible(database_telegram_id)
                .await?;

            return Ok(TrialIssueResult::Ineligible);
        }

        let expires_at = Utc::now() + Duration::days(self.config.days);

        let traffic_limit_bytes = self
            .config
            .traffic_gib
            .checked_mul(GIB)
            .ok_or(TrialError::InvalidTrafficLimit)?;

        let request = CreateUserRequest {
            username: format!("trial_{telegram_id}"),

            status: String::from("ACTIVE"),

            traffic_limit_bytes,

            traffic_limit_strategy: String::from("NO_RESET"),

            expire_at: expires_at.to_rfc3339(),

            description: String::from("Пробная подписка, созданная VPNBotTG"),

            tag: String::from("TRIAL"),

            telegram_id,

            hwid_device_limit: self.config.hwid_limit,

            active_internal_squads: vec![self.internal_squad_uuid.clone()],
        };

        match self.remnawave.create_user(&request).await {
            Ok(user) => {
                self.database
                    .mark_trial_active(database_telegram_id, user.id, expires_at)
                    .await?;

                Ok(TrialIssueResult::Created(user))
            }

            Err(create_error) => {
                // Запрос мог реально создать пользователя,
                // но соединение могло оборваться до ответа.
                // Поэтому перед разрешением повторной попытки
                // проверяем Remnawave ещё раз.
                if let Ok(users) = self.remnawave.find_users_by_telegram_id(telegram_id).await {
                    if let Some(user) = users
                        .into_iter()
                        .find(|user| user.tag.as_deref() == Some("TRIAL"))
                    {
                        let recovered_expiration = parse_expiration(&user)?;

                        self.database
                            .mark_trial_active(database_telegram_id, user.id, recovered_expiration)
                            .await?;

                        return Ok(TrialIssueResult::Recovered(user));
                    }
                }

                self.mark_failed_best_effort(database_telegram_id).await;

                Err(create_error.into())
            }
        }
    }

    async fn mark_failed_best_effort(&self, telegram_id: i64) {
        if let Err(error) = self.database.mark_trial_failed(telegram_id).await {
            tracing::error!(
                telegram_id,
                error = %error,
                "Не удалось отметить создание trial как failed"
            );
        }
    }
}

fn parse_expiration(user: &RemnawaveUser) -> Result<DateTime<Utc>, TrialError> {
    Ok(DateTime::parse_from_rfc3339(&user.expire_at)?.with_timezone(&Utc))
}

#[derive(Debug, Error)]
pub enum TrialError {
    #[error(transparent)]
    Database(#[from] DatabaseError),

    #[error(transparent)]
    Remnawave(#[from] RemnawaveError),

    #[error("Internal Squad '{0}' не найден")]
    InternalSquadNotFound(String),

    #[error("Telegram ID не помещается в BIGINT")]
    InvalidTelegramId,

    #[error("слишком большой лимит трафика")]
    InvalidTrafficLimit,

    #[error("некорректная дата окончания подписки: {0}")]
    InvalidExpiration(#[from] chrono::ParseError),
}
