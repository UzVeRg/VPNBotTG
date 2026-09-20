mod activation;
mod bot;
mod config;
mod database;
mod device;
mod payment;
mod remnawave;
mod tariff;
mod trial;
mod ui;
mod webhook;

use config::Config;
use database::Database;
use remnawave::RemnawaveClient;
use tariff::TariffCatalog;
use teloxide::Bot;
use tracing_subscriber::EnvFilter;
use trial::TrialService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    init_logging();

    let config = Config::from_env()?;

    let webhook_bind = config.platega.webhook_bind;

    let tariffs = TariffCatalog::load("config/tariffs.json")?;

    tracing::info!(
        tariffs = tariffs.len(),
        active_tariffs = tariffs.active().count(),
        "Каталог тарифов загружен"
    );

    let database = Database::connect(&config.database_url).await?;

    tracing::info!("PostgreSQL подключён");

    let bot = Bot::new(config.telegram_token.clone());

    let remnawave =
        RemnawaveClient::new(config.remnawave_url.clone(), config.remnawave_token.clone());

    let trial =
        TrialService::new(database.clone(), remnawave.clone(), config.trial.clone()).await?;

    let webhook_task = tokio::spawn(async move {
        tracing::info!(bind = %webhook_bind, "HTTP endpoint Platega запущен");

        if let Err(error) = webhook::serve(webhook_bind).await {
            tracing::error!(error = %error, "HTTP endpoint Platega остановлен с ошибкой");
        }
    });

    tracing::info!("VPNBotTG запущен");

    bot::run(bot, remnawave, trial, database, tariffs, config.service).await;

    webhook_task.abort();

    tracing::info!("VPNBotTG остановлен");

    Ok(())
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("vpn_bot_tg=info,teloxide=info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}
