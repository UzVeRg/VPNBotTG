use std::net::SocketAddr;

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::{get, post},
};
use serde::Deserialize;

use crate::{
    activation::ActivationService,
    config::PlategaConfig,
    database::{Database, MarkPaymentPaidResult},
    payment::PROVIDER_PLATEGA,
};

#[derive(Clone)]
pub struct WebhookState {
    database: Database,
    activation: ActivationService,
    merchant_id: Option<String>,
    api_key: Option<String>,
}

impl WebhookState {
    pub fn new(
        database: Database,
        activation: ActivationService,
        config: PlategaConfig,
    ) -> Self {
        Self {
            database,
            activation,
            merchant_id: config.merchant_id,
            api_key: config.api_key,
        }
    }
}

pub async fn serve(
    bind: SocketAddr,
    state: WebhookState,
) -> std::io::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/platega/webhook", post(platega_webhook))
        .route("/payment/success", get(payment_success))
        .route("/payment/failed", get(payment_failed))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind).await?;

    axum::serve(listener, app).await
}

async fn health() -> &'static str {
    "ok"
}

async fn platega_webhook(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    if body.is_empty() || body.iter().all(|byte| byte.is_ascii_whitespace()) {
        return StatusCode::OK;
    }

    let Some(merchant_id) = state.merchant_id.as_deref() else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };

    let Some(api_key) = state.api_key.as_deref() else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };

    if !header_matches(&headers, "x-merchantid", merchant_id)
        || !header_matches(&headers, "x-secret", api_key)
    {
        return StatusCode::UNAUTHORIZED;
    }

    let callback = match serde_json::from_slice::<PlategaWebhook>(&body) {
        Ok(callback) => callback,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "Получен некорректный callback Platega"
            );

            return StatusCode::BAD_REQUEST;
        }
    };

    let payment = match state
        .database
        .get_payment_by_provider_payment_id(
            PROVIDER_PLATEGA,
            &callback.id,
        )
        .await
    {
        Ok(Some(payment)) => payment,

        Ok(None) => {
            tracing::warn!(
                transaction_id = callback.id,
                "Транзакция Platega не найдена"
            );

            return StatusCode::NOT_FOUND;
        }

        Err(error) => {
            tracing::error!(
                transaction_id = callback.id,
                error = %error,
                "Не удалось получить платёж из PostgreSQL"
            );

            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let Some(amount_kopecks) = amount_to_kopecks(callback.amount) else {
        return StatusCode::BAD_REQUEST;
    };

    if amount_kopecks != payment.amount_kopecks
        || callback.currency != payment.currency
    {
        tracing::error!(
            payment_id = payment.id,
            transaction_id = callback.id,
            "Сумма или валюта callback не совпадает с платежом"
        );

        return StatusCode::CONFLICT;
    }

    if let Some(payload) = callback.payload.as_deref() {
        let expected =
            format!("order:{};payment:{}", payment.order_id, payment.id);

        if payload != expected {
            tracing::error!(
                payment_id = payment.id,
                transaction_id = callback.id,
                "Payload callback не совпадает с платежом"
            );

            return StatusCode::CONFLICT;
        }
    }

    match callback.status.as_str() {
        "PENDING" => StatusCode::OK,

        "CANCELED" => {
            match state.database.mark_payment_cancelled(payment.id).await {
                Ok(()) => StatusCode::OK,

                Err(error) => {
                    tracing::error!(
                        payment_id = payment.id,
                        error = %error,
                        "Не удалось отменить платёж"
                    );

                    StatusCode::INTERNAL_SERVER_ERROR
                }
            }
        }

        "CHARGEBACKED" | "CHARGEBACK" => {
            match state.database.mark_payment_chargeback(payment.id).await {
                Ok(()) => {
                    tracing::warn!(
                        payment_id = payment.id,
                        order_id = payment.order_id,
                        "Получен chargeback Platega"
                    );

                    StatusCode::OK
                }

                Err(error) => {
                    tracing::error!(
                        payment_id = payment.id,
                        error = %error,
                        "Не удалось сохранить chargeback"
                    );

                    StatusCode::INTERNAL_SERVER_ERROR
                }
            }
        }

        "CONFIRMED" => {
            let result = match state.database.mark_payment_paid(payment.id).await {
                Ok(result) => result,

                Err(error) => {
                    tracing::error!(
                        payment_id = payment.id,
                        error = %error,
                        "Не удалось подтвердить платёж"
                    );

                    return StatusCode::INTERNAL_SERVER_ERROR;
                }
            };

            match result {
                MarkPaymentPaidResult::Applied
                | MarkPaymentPaidResult::AlreadyPaid =>
                {
                    match state
                        .activation
                        .activate_order(payment.order_id)
                        .await
                    {
                        Ok(_) => {
                            tracing::info!(
                                payment_id = payment.id,
                                order_id = payment.order_id,
                                transaction_id = callback.id,
                                "Оплата подтверждена и подписка активирована"
                            );

                            StatusCode::OK
                        }

                        Err(error) => {
                            tracing::error!(
                                payment_id = payment.id,
                                order_id = payment.order_id,
                                error = %error,
                                "Оплата подтверждена, но активация не завершена"
                            );

                            StatusCode::SERVICE_UNAVAILABLE
                        }
                    }
                }

                MarkPaymentPaidResult::OrderNotPayable => {
                    tracing::error!(
                        payment_id = payment.id,
                        order_id = payment.order_id,
                        "Оплаченный заказ находится в недопустимом состоянии"
                    );

                    StatusCode::CONFLICT
                }

                MarkPaymentPaidResult::NotFound => StatusCode::NOT_FOUND,
            }
        }

        _ => StatusCode::BAD_REQUEST,
    }
}

fn header_matches(
    headers: &HeaderMap,
    name: &'static str,
    expected: &str,
) -> bool {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        == Some(expected)
}

fn amount_to_kopecks(amount: f64) -> Option<i64> {
    if !amount.is_finite() || amount <= 0.0 {
        return None;
    }

    let value = amount * 100.0;
    let rounded = value.round();

    if (value - rounded).abs() > 0.000001 {
        return None;
    }

    if rounded > i64::MAX as f64 {
        return None;
    }

    Some(rounded as i64)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlategaWebhook {
    id: String,
    amount: f64,
    currency: String,
    status: String,

    #[serde(rename = "paymentMethod")]
    _payment_method: Option<u32>,

    payload: Option<String>,
}

async fn payment_success() -> Html<&'static str> {
    Html(
        "<!doctype html><html lang=\"ru\"><meta charset=\"utf-8\"><title>SilentOkVPN</title><body><h1>Оплата получена</h1><p>Вернитесь в Telegram-бот SilentOkVPN.</p></body></html>",
    )
}

async fn payment_failed() -> Html<&'static str> {
    Html(
        "<!doctype html><html lang=\"ru\"><meta charset=\"utf-8\"><title>SilentOkVPN</title><body><h1>Оплата не завершена</h1><p>Вернитесь в Telegram-бот SilentOkVPN и попробуйте снова.</p></body></html>",
    )
}