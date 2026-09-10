use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub const CALLBACK_HOME: &str = "home";
pub const CALLBACK_STATUS: &str = "status";
pub const CALLBACK_SUBSCRIPTION: &str = "subscription";
pub const CALLBACK_ABOUT: &str = "about";

pub fn home_text() -> &'static str {
    "🛡 SilentOkVPN\n\n\
     Панель правления вашей VPN-подпиской.\n\n\
     Выберите действие:"
}

pub fn home_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
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

pub fn about_text() -> &'static str {
    "ℹ️ О сервисе\n\n\
     Бот начинающего проекта, предоставляющего средства анонимизации трафика для общественных мест или создания корпоративных сетей.\n\n\
     Здесь можно приобрести подписку, проверить состояние подписки, \
     посмотреть использование трафика и получить ссылку для подключения.\n\n\
     Новые пользователи могут опробовать весь функционал бесплатно. Бесплатная подписка действует 3 дня и ограничена по трафику."
}
