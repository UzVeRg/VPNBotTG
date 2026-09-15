use crate::{
    config::{ServiceConfig, TrialConfig},
    database::Order,
    tariff::{Tariff, TariffCatalog},
};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub const CALLBACK_HOME: &str = "home";
pub const CALLBACK_STATUS: &str = "status";
pub const CALLBACK_SUBSCRIPTION: &str = "subscription";
pub const CALLBACK_ABOUT: &str = "about";
pub const CALLBACK_TRIAL: &str = "trial";
pub const CALLBACK_BUY: &str = "buy";
pub const CALLBACK_SUPPORT: &str = "support";
pub const CALLBACK_DOCUMENTS: &str = "documents";
pub const CALLBACK_TARIFF_PREFIX: &str = "tariff:";
pub const CALLBACK_CHECKOUT_PREFIX: &str = "checkout:";

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
        vec![
            InlineKeyboardButton::callback("🆘 Поддержка", CALLBACK_SUPPORT),
            InlineKeyboardButton::callback("📄 Документы", CALLBACK_DOCUMENTS),
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
         Выберите подходящий вариант. Перед оформлением можно ознакомиться \
         с описанием и условиями каждого тарифа:",
    )
}

pub fn tariffs_keyboard(tariffs: &TariffCatalog) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();

    for tariff in tariffs.active() {
        rows.push(vec![InlineKeyboardButton::callback(
            format!("{} — {} ₽", tariff.name, tariff.price_rubles()),
            format!("{}{}", CALLBACK_TARIFF_PREFIX, tariff.code),
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
        None => String::from("Без лимита"),
    };

    format!(
        "💳 {}\n\n\
         {}\n\n\
         💰 Цена: {} ₽\n\
         📅 Срок доступа: {} дней\n\
         📊 Трафик: {}\n\
         📱 Устройств: до {}\n\n\
         Доступ активируется после подтверждения оплаты.",
        tariff.name,
        tariff.description,
        tariff.price_rubles(),
        tariff.duration_days,
        traffic,
        tariff.hwid_limit,
    )
}

pub fn tariff_keyboard(tariff: &Tariff, service: &ServiceConfig) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback(
            "Продолжить оформление",
            format!("{}{}", CALLBACK_CHECKOUT_PREFIX, tariff.code),
        )],
        vec![
            InlineKeyboardButton::url("📜 Соглашение", service.user_agreement_url.clone()),
            InlineKeyboardButton::url("🔐 Конфиденциальность", service.privacy_url.clone()),
        ],
        vec![InlineKeyboardButton::url(
            "📚 Все документы",
            service.docs_url.clone(),
        )],
        vec![InlineKeyboardButton::callback("⬅️ К тарифам", CALLBACK_BUY)],
        vec![InlineKeyboardButton::callback(
            "🏠 Главное меню",
            CALLBACK_HOME,
        )],
    ])
}

pub fn checkout_text(tariff: &Tariff) -> String {
    format!(
        "🧾 Оформление заказа\n\n\
         Тариф: {}\n\
         Сумма: {} ₽\n\
         Срок доступа: {} дней\n\n\
         Продолжая оформление, пользователь подтверждает, \
         что ознакомился с пользовательским соглашением \
         и политикой конфиденциальности.\n\n\
         Условия возврата указаны в пользовательском соглашении.\n\n\
         Создание платёжного счёта пока недоступно.",
        tariff.name,
        tariff.price_rubles(),
        tariff.duration_days,
    )
}

pub fn checkout_keyboard(service: &ServiceConfig) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::url("📜 Соглашение", service.user_agreement_url.clone()),
            InlineKeyboardButton::url("🔐 Конфиденциальность", service.privacy_url.clone()),
        ],
        vec![InlineKeyboardButton::url(
            "📚 Все документы",
            service.docs_url.clone(),
        )],
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

pub fn about_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::callback("🆘 Поддержка", CALLBACK_SUPPORT),
            InlineKeyboardButton::callback("📄 Документы", CALLBACK_DOCUMENTS),
        ],
        vec![InlineKeyboardButton::callback("⬅️ Назад", CALLBACK_HOME)],
    ])
}

pub fn support_text(service: &ServiceConfig) -> String {
    let mut text = format!(
        "🆘 Поддержка и контакты\n\n\
         Telegram: {}\n\
         E-mail: {}",
        service.support_telegram_url, service.support_email,
    );

    if let Some(phone) = &service.phone {
        text.push_str(&format!("\nТелефон: {phone}"));
    }

    if let Some(address) = &service.contact_address {
        text.push_str(&format!("\nАдрес: {address}"));
    }

    text.push_str(
        "\n\nПо вопросам работы сервиса, оплаты и возврата \
         обращайтесь в поддержку.",
    );

    text
}

pub fn support_keyboard(service: &ServiceConfig) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::url(
            "Написать в Telegram",
            service.support_telegram_url.clone(),
        )],
        vec![InlineKeyboardButton::callback(
            "📄 Документы",
            CALLBACK_DOCUMENTS,
        )],
        vec![InlineKeyboardButton::callback("⬅️ Назад", CALLBACK_HOME)],
    ])
}

pub fn documents_text() -> &'static str {
    "📄 Документы SilentOkVPN\n\n\
     Перед приобретением подписки ознакомьтесь \
     с пользовательским соглашением и политикой \
     конфиденциальности.\n\n\
     Условия возврата включены в пользовательское соглашение."
}

pub fn documents_keyboard(service: &ServiceConfig) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::url(
            "📚 SilentOkVPN Docs",
            service.docs_url.clone(),
        )],
        vec![InlineKeyboardButton::url(
            "📜 Пользовательское соглашение",
            service.user_agreement_url.clone(),
        )],
        vec![InlineKeyboardButton::url(
            "🔐 Политика конфиденциальности",
            service.privacy_url.clone(),
        )],
        vec![InlineKeyboardButton::callback("⬅️ Назад", CALLBACK_HOME)],
    ])
}

pub fn order_text(order: &Order) -> String {
    format!(
        "🧾 Заказ #{}\n\n\
         Тариф: {}\n\
         Сумма: {} ₽\n\
         Срок доступа: {} дней\n\
         Статус: ожидает оплаты\n\n\
         Заказ создан и сохранён.\n\
         Оплата для него пока недоступна.",
        order.id,
        order.tariff_name,
        order.price_kopecks / 100,
        order.duration_days,
    )
}

pub fn order_keyboard(service: &ServiceConfig) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::url("📜 Соглашение", service.user_agreement_url.clone()),
            InlineKeyboardButton::url("🔐 Конфиденциальность", service.privacy_url.clone()),
        ],
        vec![InlineKeyboardButton::callback("⬅️ К тарифам", CALLBACK_BUY)],
        vec![InlineKeyboardButton::callback(
            "🏠 Главное меню",
            CALLBACK_HOME,
        )],
    ])
}

pub fn about_text(trial: &TrialConfig) -> String {
    format!(
        "ℹ️ О сервисе\n\n\
         SilentOkVPN предоставляет VPN-доступ для защищённого сетевого соединения.\n\n\
         В боте можно приобрести подписку, проверить её состояние, \
         посмотреть использование трафика и получить ссылку для подключения.\n\n\
         🎁 Пробная подписка:\n\
         📅 Срок: {} дн.\n\
         📊 Трафик: {} GiB\n\
         📱 Устройства: до {}",
        trial.days, trial.traffic_gib, trial.hwid_limit,
    )
}
