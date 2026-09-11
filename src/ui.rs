use crate::config::TrialConfig;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub const CALLBACK_HOME: &str = "home";
pub const CALLBACK_STATUS: &str = "status";
pub const CALLBACK_SUBSCRIPTION: &str = "subscription";
pub const CALLBACK_ABOUT: &str = "about";
pub const CALLBACK_TRIAL: &str = "trial";

pub fn home_text() -> &'static str {
    "🛡 SilentOkVPN\n\n\
     Панель управления вашей VPN-подпиской.\n\n\
     Выберите действие:"
}

pub fn home_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback(
            "🎁 Попробовать бесплатно",
            CALLBACK_TRIAL,
        )],
        vec![InlineKeyboardButton::callback(
            "📋 Моя подписка",
            CALLBACK_STATUS,
        )],
        vec![
            InlineKeyboardButton::callback("🔑 Ссылка", CALLBACK_SUBSCRIPTION),
            InlineKeyboardButton::callback("ℹ️ О сервисе", CALLBACK_ABOUT),
        ],
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
