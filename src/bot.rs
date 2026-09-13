use chrono::{DateTime, Utc};

use teloxide::{
    dptree,
    payloads::{SetMyDescriptionSetters, SetMyShortDescriptionSetters},
    prelude::*,
    types::CallbackQuery,
    utils::command::BotCommands,
};

use crate::{
    database::Database,
    remnawave::{RemnawaveClient, RemnawaveUser},
    tariff::TariffCatalog,
    trial::{TrialIssueResult, TrialService},
    ui,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Открыть главное меню.
    Start,
}

pub async fn run(
    bot: Bot,
    remnawave: RemnawaveClient,
    trial: TrialService,
    database: Database,
    tariffs: TariffCatalog,
) {
    configure_profile(&bot).await;

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(Update::filter_callback_query().endpoint(handle_callback))
        .branch(Update::filter_message().endpoint(handle_other_message));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![remnawave, trial, database, tariffs])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn configure_profile(bot: &Bot) {
    if let Err(error) = bot
        .set_my_description()
        .description(
            "Управление VPN-подпиской прямо из Telegram.\n\n\
             Проверяйте статус и трафик, получайте ссылку \
             для подключения и управляйте доступом.",
        )
        .await
    {
        tracing::warn!(
            error = %error,
            "Failed to set Telegram bot description"
        );
    }

    if let Err(error) = bot
        .set_my_short_description()
        .short_description("Личный кабинет VPN прямо в Telegram")
        .await
    {
        tracing::warn!(
            error = %error,
            "Failed to set Telegram bot short description"
        );
    }

    if let Err(error) = bot.delete_my_commands().await {
        tracing::warn!(
            error = %error,
            "Failed to delete Telegram command menu"
        );
    }
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    command: Command,
    database: Database,
) -> ResponseResult<()> {
    if !msg.chat.is_private() {
        return Ok(());
    }

    register_message_user(&database, &msg).await;

    match command {
        Command::Start => {
            show_home(&bot, msg.chat.id).await?;
        }
    }

    Ok(())
}

async fn handle_other_message(bot: Bot, msg: Message, database: Database) -> ResponseResult<()> {
    if !msg.chat.is_private() {
        return Ok(());
    }

    register_message_user(&database, &msg).await;

    show_home(&bot, msg.chat.id).await?;

    Ok(())
}

async fn handle_callback(
    bot: Bot,
    query: CallbackQuery,
    remnawave: RemnawaveClient,
    trial: TrialService,
    database: Database,
) -> ResponseResult<()> {
    // Telegram рекомендует отвечать на callback,
    // чтобы убрать индикатор загрузки у кнопки.
    bot.answer_callback_query(query.id.clone()).await?;

    let Some(data) = query.data.as_deref() else {
        return Ok(());
    };

    let Some(message) = query.regular_message() else {
        return Ok(());
    };

    if !message.chat.is_private() {
        return Ok(());
    }

    register_user(&database, query.from.id.0).await;

    match data {
        ui::CALLBACK_HOME => {
            edit_screen(&bot, message, ui::home_text(), ui::home_keyboard()).await?;
        }

        ui::CALLBACK_STATUS => {
            show_status(&bot, message, query.from.id.0, &remnawave).await?;
        }

        ui::CALLBACK_SUBSCRIPTION => {
            show_subscription(&bot, message, query.from.id.0, &remnawave).await?;
        }

        ui::CALLBACK_ABOUT => {
            let text = ui::about_text(trial.config());

            edit_screen(&bot, message, &text, ui::back_keyboard()).await?;
        }

        ui::CALLBACK_TRIAL => {
            show_trial(&bot, message, query.from.id.0, &trial).await?;
        }

        _ => {
            tracing::warn!(callback = data, "Unknown callback");
        }
    }

    Ok(())
}

async fn register_message_user(database: &Database, message: &Message) {
    let Some(user) = message.from.as_ref() else {
        return;
    };

    register_user(database, user.id.0).await;
}

async fn register_user(database: &Database, telegram_id: u64) {
    if let Err(error) = database.ensure_user(telegram_id).await {
        tracing::error!(
            telegram_id,
            error = %error,
            "Не удалось зарегистрировать пользователя в PostgreSQL"
        );
    }
}

async fn show_trial(
    bot: &Bot,
    message: &Message,
    telegram_id: u64,
    trial: &TrialService,
) -> ResponseResult<()> {
    match trial.issue_trial(telegram_id).await {
        Ok(TrialIssueResult::Created(user) | TrialIssueResult::Recovered(user)) => {
            let config = trial.config();

            let expires_at = format_datetime(&user.expire_at);

            let text = format!(
                "🎉 Пробная подписка активирована!\n\n\
                 📅 Срок: {} дн.\n\
                 📊 Трафик: {} GiB\n\
                 📱 Устройства: до {}\n\
                 ⏳ Действует до: {}\n\n\
                 Теперь можно получить ссылку для подключения.",
                config.days, config.traffic_gib, config.hwid_limit, expires_at,
            );

            edit_screen(bot, message, &text, ui::trial_keyboard()).await?;
        }

        Ok(TrialIssueResult::AlreadyUsed) => {
            edit_screen(
                bot,
                message,
                "🎁 Пробный период уже был использован.\n\n\
                 Повторное получение бесплатной \
                 подписки недоступно.",
                ui::back_keyboard(),
            )
            .await?;
        }

        Ok(TrialIssueResult::Ineligible) => {
            edit_screen(
                bot,
                message,
                "🎁 Пробный период доступен только \
                 новым пользователям.\n\n\
                 Для этого Telegram-аккаунта уже \
                 существует или ранее существовала \
                 VPN-подписка.",
                ui::back_keyboard(),
            )
            .await?;
        }

        Ok(TrialIssueResult::InProgress) => {
            edit_screen(
                bot,
                message,
                "⏳ Пробная подписка уже создаётся.\n\n\
                 Подождите несколько секунд и \
                 повторите попытку.",
                ui::back_keyboard(),
            )
            .await?;
        }

        Err(error) => {
            tracing::error!(
                telegram_id,
                error = %error,
                "Не удалось создать trial"
            );

            edit_screen(
                bot,
                message,
                "⚠️ Не удалось создать пробную \
                 подписку.\n\n\
                 Попробуйте ещё раз немного позже.",
                ui::back_keyboard(),
            )
            .await?;
        }
    }

    Ok(())
}

async fn show_home(bot: &Bot, chat_id: ChatId) -> ResponseResult<()> {
    bot.send_message(chat_id, ui::home_text())
        .reply_markup(ui::home_keyboard())
        .await?;

    Ok(())
}

async fn show_status(
    bot: &Bot,
    message: &Message,
    telegram_id: u64,
    remnawave: &RemnawaveClient,
) -> ResponseResult<()> {
    match remnawave.find_users_by_telegram_id(telegram_id).await {
        Ok(users) if users.is_empty() => {
            edit_screen(
                bot,
                message,
                "📋 Подписка\n\n\
                 VPN-подписка для вашего Telegram-аккаунта не найдена.",
                ui::back_keyboard(),
            )
            .await?;
        }

        Ok(users) => {
            let text = format_status(&users);

            edit_screen(bot, message, &text, ui::status_keyboard()).await?;
        }

        Err(error) => {
            tracing::error!(
                telegram_id,
                error = %error,
                "Remnawave API request failed"
            );

            edit_screen(
                bot,
                message,
                "⚠️ Не удалось получить информацию о подписке.\n\n\
                 Попробуйте ещё раз немного позже.",
                ui::status_keyboard(),
            )
            .await?;
        }
    }

    Ok(())
}

async fn show_subscription(
    bot: &Bot,
    message: &Message,
    telegram_id: u64,
    remnawave: &RemnawaveClient,
) -> ResponseResult<()> {
    match remnawave.find_users_by_telegram_id(telegram_id).await {
        Ok(users) if users.is_empty() => {
            edit_screen(
                bot,
                message,
                "🔑 Ссылка на подписку\n\n\
                 Активная VPN-подписка не найдена.",
                ui::back_keyboard(),
            )
            .await?;
        }

        Ok(users) => {
            let mut text = String::from("🔑 Ссылка подписки\n\n");

            for user in users {
                text.push_str(&format!("{}\n{}\n\n", user.username, user.subscription_url,));
            }

            text.push_str(
                "⚠️ Не передавайте эту ссылку другим людям. \
                 Она предоставляет доступ к вашей VPN-подписке.",
            );

            edit_screen(bot, message, &text, ui::back_keyboard()).await?;
        }

        Err(error) => {
            tracing::error!(
                telegram_id,
                error = %error,
                "Remnawave API request failed"
            );

            edit_screen(
                bot,
                message,
                "⚠️ Не удалось получить ссылку подписки.",
                ui::back_keyboard(),
            )
            .await?;
        }
    }

    Ok(())
}

async fn edit_screen(
    bot: &Bot,
    message: &Message,
    text: &str,
    keyboard: teloxide::types::InlineKeyboardMarkup,
) -> ResponseResult<()> {
    match bot
        .edit_message_text(message.chat.id, message.id, text)
        .reply_markup(keyboard)
        .await
    {
        Ok(_) => Ok(()),

        Err(teloxide::RequestError::Api(teloxide::ApiError::MessageNotModified)) => Ok(()),

        Err(error) => Err(error),
    }
}

fn format_status(users: &[RemnawaveUser]) -> String {
    let mut text = String::from("📋 Моя подписка\n");

    for user in users {
        let status_icon = match user.status.as_str() {
            "ACTIVE" => "🟢",
            "DISABLED" => "🔴",
            "LIMITED" => "🟠",
            "EXPIRED" => "⚫",
            _ => "⚪",
        };

        let used = format_bytes(user.user_traffic.used_traffic_bytes);

        let limit = if user.traffic_limit_bytes == 0 {
            String::from("Без лимита")
        } else {
            format_bytes(user.traffic_limit_bytes)
        };

        let expires_at = format_datetime(&user.expire_at);

        text.push_str(&format!(
            "\n{status_icon} Статус: {}\n\
                 👤 {}\n\
                 📅 Действует до: {}\n\
                 📊 Использовано: {}\n\
                 📦 Лимит: {}\n",
            user.status, user.username, expires_at, used, limit,
        ));
    }

    text
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;

    const MB: f64 = KB * 1024.0;

    const GB: f64 = MB * 1024.0;

    const TB: f64 = GB * 1024.0;

    let bytes = bytes as f64;

    if bytes >= TB {
        format!("{:.2} TB", bytes / TB,)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes / GB,)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes / MB,)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes / KB,)
    } else {
        format!("{bytes:.0} B")
    }
}

fn format_datetime(value: &str) -> String {
    match DateTime::parse_from_rfc3339(value) {
        Ok(datetime) => datetime
            .with_timezone(&Utc)
            .format("%d.%m.%Y %H:%M UTC")
            .to_string(),

        Err(_) => value.to_owned(),
    }
}
