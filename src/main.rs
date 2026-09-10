mod bot;
mod config;
mod remnawave;
mod ui;

use config::Config;
use remnawave::RemnawaveClient;
use teloxide::Bot;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    init_logging();

    let config = Config::from_env()?;

    let bot = Bot::new(config.telegram_token);

    let remnawave = RemnawaveClient::new(config.remnawave_url, config.remnawave_token);

    tracing::info!("VPNBotTG запущен");

    bot::run(bot, remnawave).await;

    tracing::info!("VPNBotTG остановлен");

    Ok(())
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("vpn_bot_tg=info,teloxide=info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}
