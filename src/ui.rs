use crate::{
    config::TrialConfig,
    tariff::{Tariff, TariffCatalog},
};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub const CALLBACK_HOME: &str = "home";
pub const CALLBACK_STATUS: &str = "status";
pub const CALLBACK_SUBSCRIPTION: &str = "subscription";
pub const CALLBACK_ABOUT: &str = "about";
pub const CALLBACK_TRIAL: &str = "trial";
pub const CALLBACK_BUY: &str = "buy";
pub const CALLBACK_TARIFF_PREFIX: &str = "tariff:";

pub fn home_text() -> &'static str {
    "🛡 SilentOkVPN\n\n\
     Панель управления вашей VPN-подпиской.\n\n\
     Выберите действие:"
}

pub fn home_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback(
            "📋 Моя подписка",
            CALLBACK_STATUS,
        )],
        vec![InlineKeyboardButton::callback(
            "💳 Купить подписку",
            CALLBACK_BUY,
        )],
        vec![InlineKeyboardButton::callback(
            "🎁 Попробовать бесплатно",
            CALLBACK_TRIAL,
        )],
        
        vec![
            InlineKeyboardButton::callback("🔑 Ссылка", CALLBACK_SUBSCRIPTION),
            InlineKeyboardButton::callback("ℹ️ О сервисе", CALLBACK_ABOUT),
        ],
    ])
}

pub fn tariffs_text(tariffs: &TariffCatalog) -> String {
    let active_count = tariffs.active().count();

    if active_count == 0 {
        return String::from(
            "💳 Тарифы\n\n\
             Сейчас нет доступных тарифов.",
        );
    }

    String::from(
        "💳 Тарифы\n\n\
         Выберите подходящий вариант:",
    )
}

pub fn tariffs_keyboard(tariffs: &TariffCatalog) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();

    for tariff in tariffs.active() {
        rows.push(vec![InlineKeyboardButton::callback(
            format!("{} — {} ₽", tariff.name, tariff.price_rubles(),),
            format!("{}{}", CALLBACK_TARIFF_PREFIX, tariff.code,),
        )]);
    }

    rows.push(vec![InlineKeyboardButton::callback(
        "⬅️ Назад",
        CALLBACK_HOME,
    )]);

    InlineKeyboardMarkup::new(rows)
}

pub fn tariff_text(tariff: &Tariff) -> String {
    let traffic = match tariff.traffic_gib {
        Some(gib) => format!("{gib} GiB"),
        None => String::from("Безлимитный"),
    };

    format!(
        "💳 {}\n\n\
         💰 Цена: {} ₽\n\
         📅 Срок: {} дней\n\
         📊 Трафик: {}\n\
         📱 Устройств: до {}\n\n\
         Тариф выбран. На следующем этапе здесь появится оформление заказа.",
        tariff.name,
        tariff.price_rubles(),
        tariff.duration_days,
        traffic,
        tariff.hwid_limit,
    )
}

pub fn tariff_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback("⬅️ К тарифам", CALLBACK_BUY)],
        vec![InlineKeyboardButton::callback(
            "🏠 Главное меню",
            CALLBACK_HOME,
        )],
    ])
}

pub fn trial_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback(
            "📋 Моя подписка",
            CALLBACK_STATUS,
        )],
        vec![InlineKeyboardButton::callback(
            "🔑 Получить ссылку",
            CALLBACK_SUBSCRIPTION,
        )],
        vec![InlineKeyboardButton::callback("⬅️ Назад", CALLBACK_HOME)],
    ])
}

pub fn status_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback(
            "🔄 Обновить",
            CALLBACK_STATUS,
        )],
        vec![InlineKeyboardButton::callback(
            "🔑 Получить ссылку",
            CALLBACK_SUBSCRIPTION,
        )],
        vec![InlineKeyboardButton::callback("⬅️ Назад", CALLBACK_HOME)],
    ])
}

pub fn back_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([[InlineKeyboardButton::callback("⬅️ Назад", CALLBACK_HOME)]])
}

pub fn about_text(trial: &TrialConfig) -> String {
    format!(
        "ℹ️ О сервисе\n\n\
         Бот развивающегося проекта, предоставляющего средства для анонимизации трафика \
         при использовании общественных сетей и для создания корпоративных сетей.\n\n\
         Здесь можно приобрести подписку, проверить её состояние, \
         посмотреть использование трафика и получить ссылку для подключения.\n\n\
         🎁 Пробная подписка:\n\
         📅 Срок: {} дн.\n\
         📊 Трафик: {} GiB\n\
         📱 Устройства: до {}",
        trial.days, trial.traffic_gib, trial.hwid_limit,
    )
}
