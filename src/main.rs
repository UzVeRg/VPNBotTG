mod bot;
mod config;
mod database;
mod payment;
mod remnawave;
mod tariff;
mod trial;
mod ui;

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

    tracing::info!("VPNBotTG запущен");

    bot::run(bot, remnawave, trial, database, tariffs, config.service).await;

    tracing::info!("VPNBotTG остановлен");

    Ok(())
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("vpn_bot_tg=info,teloxide=info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}
