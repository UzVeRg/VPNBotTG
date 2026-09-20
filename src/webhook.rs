use std::net::SocketAddr;

use axum::{
    Router,
    body::Bytes,
    http::StatusCode,
    response::Html,
    routing::{get, post},
};

pub async fn serve(bind: SocketAddr) -> std::io::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/platega/webhook", post(platega_webhook))
        .route("/payment/success", get(payment_success))
        .route("/payment/failed", get(payment_failed));

    let listener = tokio::net::TcpListener::bind(bind).await?;

    axum::serve(listener, app).await
}

async fn health() -> &'static str {
    "ok"
}

async fn platega_webhook(body: Bytes) -> StatusCode {
    if body.is_empty() || body.iter().all(|byte| byte.is_ascii_whitespace()) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
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