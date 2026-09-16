use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
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

    pub async fn get_user_by_id(
        &self,
        user_id: i64,
    ) -> Result<Option<RemnawaveUser>, RemnawaveError> {
        let url = format!("{}/api/users/{}", self.base_url, user_id);

        let response = self.http.get(url).bearer_auth(&self.token).send().await?;

        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("<response body unavailable>"));

            return Err(RemnawaveError::Api { status, body });
        }

        let response = response.json::<UserResponse>().await?;

        Ok(Some(response.response))
    }

    pub async fn create_user(
        &self,
        request: &CreateUserRequest,
    ) -> Result<RemnawaveUser, RemnawaveError> {
        let url = format!("{}/api/users", self.base_url);

        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .json(request)
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

        let response = response.json::<UserResponse>().await?;

        Ok(response.response)
    }

    pub async fn update_user(
        &self,
        request: &UpdateUserRequest,
    ) -> Result<RemnawaveUser, RemnawaveError> {
        let url = format!("{}/api/users", self.base_url);

        let response = self
            .http
            .patch(url)
            .bearer_auth(&self.token)
            .json(request)
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

        let response = response.json::<UserResponse>().await?;

        Ok(response.response)
    }

    pub async fn find_internal_squad_by_name(
        &self,
        name: &str,
    ) -> Result<Option<InternalSquad>, RemnawaveError> {
        let url = format!("{}/api/internal-squads", self.base_url);

        let response = self.http.get(url).bearer_auth(&self.token).send().await?;

        let status = response.status();

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("<response body unavailable>"));

            return Err(RemnawaveError::Api { status, body });
        }

        let response = response.json::<InternalSquadsResponse>().await?;

        Ok(response
            .response
            .internal_squads
            .into_iter()
            .find(|squad| squad.name == name))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub username: String,
    pub status: String,
    pub traffic_limit_bytes: u64,
    pub traffic_limit_strategy: String,
    pub expire_at: String,
    pub description: String,
    pub tag: String,
    pub telegram_id: u64,
    pub hwid_device_limit: u32,
    pub active_internal_squads: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    pub id: i64,

    pub status: String,

    pub traffic_limit_bytes: u64,

    pub traffic_limit_strategy: String,

    pub expire_at: String,

    pub description: String,

    pub tag: String,

    pub telegram_id: u64,

    pub hwid_device_limit: u32,

    pub active_internal_squads: Vec<String>,
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
    pub tag: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
pub struct InternalSquad {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct UserResponse {
    response: RemnawaveUser,
}

#[derive(Debug, Deserialize)]
struct UsersStreamResponse {
    response: UsersStreamData,
}

#[derive(Debug, Deserialize)]
struct UsersStreamData {
    users: Vec<RemnawaveUser>,
}

#[derive(Debug, Deserialize)]
struct InternalSquadsResponse {
    response: InternalSquadsData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InternalSquadsData {
    internal_squads: Vec<InternalSquad>,
}

#[derive(Debug, Error)]
pub enum RemnawaveError {
    #[error("ошибка HTTP-запроса: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Remnawave API вернул HTTP {status}: {body}")]
    Api { status: StatusCode, body: String },
}
