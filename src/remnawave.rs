use reqwest::{Client, StatusCode};
use serde::Deserialize;
use thiserror::Error;

#[derive(Clone)]
pub struct RemnawaveClient {
    http: Client,
    base_url: String,
    token: String,
}

impl RemnawaveClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            token,
        }
    }

    pub async fn find_users_by_telegram_id(
        &self,
        telegram_id: u64,
    ) -> Result<Vec<RemnawaveUser>, RemnawaveError> {
        let url = format!("{}/api/users/stream", self.base_url);

        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .query(&[
                ("telegramId", telegram_id.to_string()),
                ("size", "10".to_string()),
            ])
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("<response body unavailable>"));

            return Err(RemnawaveError::Api { status, body });
        }

        let response = response.json::<UsersStreamResponse>().await?;

        Ok(response.response.users)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemnawaveUser {
    pub id: i64,
    pub username: String,
    pub status: String,
    pub traffic_limit_bytes: u64,
    pub expire_at: String,
    pub telegram_id: Option<u64>,
    pub subscription_url: String,
    pub user_traffic: UserTraffic,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTraffic {
    pub used_traffic_bytes: u64,
    pub lifetime_used_traffic_bytes: u64,
    pub online_at: Option<String>,
    pub first_connected_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsersStreamResponse {
    response: UsersStreamData,
}

#[derive(Debug, Deserialize)]
struct UsersStreamData {
    users: Vec<RemnawaveUser>,
}

#[derive(Debug, Error)]
pub enum RemnawaveError {
    #[error("ошибка HTTP-запроса: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Remnawave API вернул HTTP {status}: {body}")]
    Api { status: StatusCode, body: String },
}
