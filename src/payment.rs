use std::time::Duration;

use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    config::PlategaConfig,
    database::{Database, DatabaseError, Order, Payment},
};

pub const PROVIDER_PLATEGA: &str = "platega";

#[derive(Clone)]
pub struct PaymentService {
    database: Database,
    platega: PlategaClient,
}

impl PaymentService {
    pub fn new(database: Database, config: PlategaConfig) -> Result<Self, PaymentError> {
        Ok(Self {
            database,
            platega: PlategaClient::new(config)?,
        })
    }

    pub async fn prepare_order(&self, order: &Order) -> Result<Payment, PaymentError> {
        let payment = self
            .database
            .get_or_create_payment(order, PROVIDER_PLATEGA)
            .await?;

        if payment.provider_payment_id.is_some() && payment.payment_url.is_some() {
            return Ok(payment);
        }

        let transaction = self.platega.create_transaction(order, &payment).await?;

        self.database
            .set_payment_provider_data(
                payment.id,
                &transaction.transaction_id,
                &transaction.payment_url,
            )
            .await
            .map_err(PaymentError::from)
    }
}

#[derive(Clone)]
struct PlategaClient {
    http: Client,
    base_url: Url,
    merchant_id: Option<String>,
    api_key: Option<String>,
    return_url: Url,
    failed_url: Url,
}

impl PlategaClient {
    fn new(config: PlategaConfig) -> Result<Self, PaymentError> {
        let http = Client::builder().timeout(Duration::from_secs(15)).build()?;

        Ok(Self {
            http,
            base_url: config.base_url,
            merchant_id: config.merchant_id,
            api_key: config.api_key,
            return_url: config.return_url,
            failed_url: config.failed_url,
        })
    }

    async fn create_transaction(
        &self,
        order: &Order,
        payment: &Payment,
    ) -> Result<CreatedTransaction, PaymentError> {
        let merchant_id = self
            .merchant_id
            .as_deref()
            .ok_or(PaymentError::NotConfigured)?;

        let api_key = self.api_key.as_deref().ok_or(PaymentError::NotConfigured)?;

        if payment.currency != "RUB" || order.currency != "RUB" {
            return Err(PaymentError::UnsupportedCurrency);
        }

        if payment.amount_kopecks <= 0 {
            return Err(PaymentError::InvalidAmount);
        }

        let endpoint = self
            .base_url
            .join("v2/transaction/process")
            .map_err(|_| PaymentError::InvalidEndpoint)?;

        let request = CreateTransactionRequest {
            payment_details: PaymentDetails {
                amount: payment.amount_kopecks as f64 / 100.0,
                currency: String::from("RUB"),
            },
            description: format!("SilentOkVPN, заказ #{}", order.id),
            return_url: self.return_url.as_str().to_owned(),
            failed_url: self.failed_url.as_str().to_owned(),
            payload: format!("order:{};payment:{}", order.id, payment.id),
            metadata: PaymentMetadata {
                user_id: order.telegram_id.to_string(),
            },
        };

        let response = self
            .http
            .post(endpoint)
            .header("X-MerchantId", merchant_id)
            .header("X-Secret", api_key)
            .json(&request)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            return Err(PaymentError::ApiStatus(status.as_u16()));
        }

        let response = response.json::<CreateTransactionResponse>().await?;

        if response.transaction_id.trim().is_empty() || response.url.trim().is_empty() {
            return Err(PaymentError::InvalidResponse);
        }

        let payment_url = Url::parse(&response.url).map_err(|_| PaymentError::InvalidResponse)?;

        if payment_url.scheme() != "https" {
            return Err(PaymentError::InvalidResponse);
        }

        Ok(CreatedTransaction {
            transaction_id: response.transaction_id,
            payment_url: payment_url.to_string(),
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateTransactionRequest {
    payment_details: PaymentDetails,
    description: String,

    #[serde(rename = "return")]
    return_url: String,

    failed_url: String,
    payload: String,
    metadata: PaymentMetadata,
}

#[derive(Serialize)]
struct PaymentDetails {
    amount: f64,
    currency: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentMetadata {
    user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTransactionResponse {
    transaction_id: String,
    status: String,
    url: String,
    expires_in: String,
}

struct CreatedTransaction {
    transaction_id: String,
    payment_url: String,
}

#[derive(Debug, Error)]
pub enum PaymentError {
    #[error("Platega API пока не настроен")]
    NotConfigured,

    #[error("поддерживается только RUB")]
    UnsupportedCurrency,

    #[error("некорректная сумма платежа")]
    InvalidAmount,

    #[error("некорректный адрес Platega API")]
    InvalidEndpoint,

    #[error("Platega вернула HTTP {0}")]
    ApiStatus(u16),

    #[error("Platega вернула некорректный ответ")]
    InvalidResponse,

    #[error("ошибка HTTP: {0}")]
    Http(#[from] reqwest::Error),

    #[error("ошибка базы данных: {0}")]
    Database(#[from] DatabaseError),
}
