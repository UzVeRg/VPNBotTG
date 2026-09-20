use crate::{
    config::{ServiceConfig, TrialConfig},
    database::{Order, Payment},
    device::{DeviceList, device_token},
    remnawave::HwidDevice,
    tariff::{Tariff, TariffCatalog},
};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

#[cfg(test)]
mod tests;

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
pub const CALLBACK_PAYMENT_PREFIX: &str = "payment:";
pub const CALLBACK_DEVICES: &str = "devices";
pub const CALLBACK_DEVICES_PAGE_PREFIX: &str = "devices_page:";
pub const CALLBACK_DEVICE_PREFIX: &str = "device:";
//pub const CALLBACK_DEVICE_DELETE_PREFIX: &str = "device_delete:";

const DEVICES_PAGE_SIZE: usize = 5;

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
            "📱 Устройства",
            CALLBACK_DEVICES,
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
        vec![InlineKeyboardButton::callback(
            "📱 Устройства",
            CALLBACK_DEVICES,
        )],
        vec![InlineKeyboardButton::callback("⬅️ Назад", CALLBACK_HOME)],
    ])
}

pub fn devices_screen(list: &DeviceList, page: usize) -> (String, InlineKeyboardMarkup) {
    let pages = list.devices.len().div_ceil(DEVICES_PAGE_SIZE).max(1);
    let page = page.min(pages - 1);
    let start = page * DEVICES_PAGE_SIZE;
    let limit = match list.limit {
        Some(0) => String::from("Без лимита"),
        Some(limit) => limit.to_string(),
        None => String::from("По настройкам сервиса"),
    };
    let mut text = format!(
        "📱 Устройства\n\nПодключено: {}\nЛимит: {limit}\n",
        list.devices.len(),
    );
    let mut rows = Vec::new();

    if list.devices.is_empty() {
        text.push_str("\nУстройств пока нет. Добавьте ссылку подписки в приложение и обновите её.");
    } else {
        //text.push_str("\nВыберите устройство, чтобы освободить его место:\n");
        text.push_str("\nВыберите устройство, чтобы посмотреть информацию:\n");
        for (index, device) in list
            .devices
            .iter()
            .enumerate()
            .skip(start)
            .take(DEVICES_PAGE_SIZE)
        {
            let number = index + 1;
            let name = device_name(device);
            text.push_str(&format!(
                "\n{number}. {name}\nДобавлено: {}\n",
                device_added_at(device),
            ));
            rows.push(vec![InlineKeyboardButton::callback(
                format!("{number}. {name}"),
                format!("{CALLBACK_DEVICE_PREFIX}{}", device_token(device)),
            )]);
        }
        if pages > 1 {
            text.push_str(&format!("\nСтраница {} из {pages}", page + 1));
        }
    }

    let mut navigation = Vec::new();
    if page > 0 {
        navigation.push(InlineKeyboardButton::callback(
            "⬅️",
            format!("{CALLBACK_DEVICES_PAGE_PREFIX}{}", page - 1),
        ));
    }
    if page + 1 < pages {
        navigation.push(InlineKeyboardButton::callback(
            "➡️",
            format!("{CALLBACK_DEVICES_PAGE_PREFIX}{}", page + 1),
        ));
    }
    if !navigation.is_empty() {
        rows.push(navigation);
    }
    rows.push(vec![InlineKeyboardButton::callback(
        "🔄 Обновить",
        format!("{CALLBACK_DEVICES_PAGE_PREFIX}{page}"),
    )]);
    rows.push(vec![InlineKeyboardButton::callback(
        "⬅️ К подписке",
        CALLBACK_STATUS,
    )]);

    (text, InlineKeyboardMarkup::new(rows))
}

/*pub fn device_confirmation_screen(device: &HwidDevice) -> (String, InlineKeyboardMarkup) {
    let name = device_name(device);
    let platform = device.platform.as_deref().unwrap_or("Не указана");
    let os_version = device.os_version.as_deref().unwrap_or("Не указана");
    let text = format!(
        "📱 {name}\n\nСистема: {}\nВерсия: {}\nДобавлено: {}\n\n\
         Удалить устройство и освободить место?\n\n\
         Это не отзывает ссылку подписки. При её обновлении в приложении \
         устройство может снова занять место.",
        device_label(platform, 40),
        device_label(os_version, 40),
        device_added_at(device),
    );
    let keyboard = InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback(
            "🗑 Удалить устройство",
            format!("{CALLBACK_DEVICE_DELETE_PREFIX}{}", device_token(device)),
        )],
        vec![InlineKeyboardButton::callback("Отмена", CALLBACK_DEVICES)],
    ]);

    (text, keyboard)
}*/

pub fn device_confirmation_screen(device: &HwidDevice) -> (String, InlineKeyboardMarkup) {
    let name = device_name(device);
    let platform = device.platform.as_deref().unwrap_or("Не указана");
    let os_version = device.os_version.as_deref().unwrap_or("Не указана");

    let text = format!(
        "📱 {name}\n\nСистема: {}\nВерсия: {}\nДобавлено: {}",
        device_label(platform, 40),
        device_label(os_version, 40),
        device_added_at(device),
    );

    let keyboard = InlineKeyboardMarkup::new([[InlineKeyboardButton::callback(
        "⬅️ К устройствам",
        CALLBACK_DEVICES,
    )]]);

    (text, keyboard)
}

pub fn devices_back_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback(
            "📱 К устройствам",
            CALLBACK_DEVICES,
        )],
        vec![InlineKeyboardButton::callback(
            "🏠 Главное меню",
            CALLBACK_HOME,
        )],
    ])
}

fn device_name(device: &HwidDevice) -> String {
    let name = device
        .device_model
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(device
            .platform
            .as_deref()
            .filter(|value| !value.trim().is_empty()))
        .unwrap_or("Устройство");
    device_label(name, 40)
}

fn device_label(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(max_chars)
        .collect()
}

fn device_added_at(device: &HwidDevice) -> String {
    chrono::DateTime::parse_from_rfc3339(&device.created_at)
        .map(|date| {
            date.with_timezone(&chrono::Utc)
                .format("%d.%m.%Y %H:%M UTC")
                .to_string()
        })
        .unwrap_or_else(|_| String::from("Не указано"))
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

pub fn order_text(order: &Order, payment: Option<&Payment>) -> String {
    let payment_state = if payment
        .and_then(|payment| payment.payment_url.as_ref())
        .is_some()
    {
        "Платёжная ссылка создана.\nНажмите «Оплатить» ниже."
    } else {
        "Заказ сохранён.\n\
         Платёжная ссылка пока недоступна.\n\
         Попробуйте снова немного позже."
    };

    format!(
        "🧾 Заказ #{}\n\n\
         Тариф: {}\n\
         Сумма: {} ₽\n\
         Срок доступа: {} дней\n\
         Статус: ожидает оплаты\n\n\
         {}",
        order.id,
        order.tariff_name,
        order.price_kopecks / 100,
        order.duration_days,
        payment_state,
    )
}

pub fn order_keyboard(
    service: &ServiceConfig,
    order: &Order,
    payment: Option<&Payment>,
) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();

    if let Some(url) = payment
        .and_then(|payment| payment.payment_url.as_deref())
        .and_then(|value| reqwest::Url::parse(value).ok())
    {
        rows.push(vec![InlineKeyboardButton::url("💳 Оплатить", url)]);
    } else {
        rows.push(vec![InlineKeyboardButton::callback(
            "🔄 Попробовать снова",
            format!("{CALLBACK_CHECKOUT_PREFIX}{}", order.tariff_code),
        )]);
    }

    rows.push(vec![
        InlineKeyboardButton::url("📜 Соглашение", service.user_agreement_url.clone()),
        InlineKeyboardButton::url("🔐 Конфиденциальность", service.privacy_url.clone()),
    ]);

    rows.push(vec![InlineKeyboardButton::callback(
        "⬅️ К тарифам",
        CALLBACK_BUY,
    )]);

    rows.push(vec![InlineKeyboardButton::callback(
        "🏠 Главное меню",
        CALLBACK_HOME,
    )]);

    InlineKeyboardMarkup::new(rows)
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
