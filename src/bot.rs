use teloxide::{
    dptree,
    prelude::*,
    utils::command::BotCommands,
};

use crate::remnawave::{RemnawaveClient, RemnawaveUser};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "Доступные команды:"
)]
enum Command {
    /// Запустить бота.
    Start,

    /// Показать список команд.
    Help,

    /// Проверить свою VPN-подписку.
    Status,
}

pub async fn run(bot: Bot, remnawave: RemnawaveClient) {
    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(handle_command);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![remnawave])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    command: Command,
    remnawave: RemnawaveClient,
) -> ResponseResult<()> {
    match command {
        Command::Start => {
            handle_start(bot, msg).await?;
        }

        Command::Help => {
            bot.send_message(
                msg.chat.id,
                Command::descriptions().to_string(),
            )
            .await?;
        }

        Command::Status => {
            handle_status(bot, msg, remnawave).await?;
        }
    }

    Ok(())
}

async fn handle_start(
    bot: Bot,
    msg: Message,
) -> ResponseResult<()> {
    let telegram_id = msg
        .from
        .as_ref()
        .map(|user| user.id.0);

    let text = match telegram_id {
        Some(id) => format!(
            "VPN Bot запущен.\n\n\
             Ваш Telegram ID: {id}\n\n\
             Команда /status проверит VPN-подписку, \
             привязанную к этому Telegram ID."
        ),

        None => String::from(
            "VPN Bot запущен.\n\n\
             Не удалось определить ваш Telegram ID."
        ),
    };

    bot.send_message(msg.chat.id, text).await?;

    Ok(())
}

async fn handle_status(
    bot: Bot,
    msg: Message,
    remnawave: RemnawaveClient,
) -> ResponseResult<()> {
    if !msg.chat.is_private() {
        bot.send_message(
            msg.chat.id,
            "Эта команда доступна только в личном чате с ботом.",
        )
        .await?;

        return Ok(());
    }

    let Some(user) = msg.from.as_ref() else {
        bot.send_message(
            msg.chat.id,
            "Не удалось определить ваш Telegram ID.",
        )
        .await?;

        return Ok(());
    };

    let telegram_id = user.id.0;

    match remnawave
        .find_users_by_telegram_id(telegram_id)
        .await
    {
        Ok(users) if users.is_empty() => {
            bot.send_message(
                msg.chat.id,
                "VPN-подписка для вашего Telegram-аккаунта пока не найдена.",
            )
            .await?;
        }

        Ok(users) => {
            bot.send_message(
                msg.chat.id,
                format_users(&users),
            )
            .await?;
        }

        Err(error) => {
            tracing::error!(
                telegram_id,
                error = %error,
                "Remnawave API request failed"
            );

            bot.send_message(
                msg.chat.id,
                "Не удалось получить информацию о подписке. Попробуйте позже.",
            )
            .await?;
        }
    }

    Ok(())
}

fn format_users(users: &[RemnawaveUser]) -> String {
    let mut message = String::from("Ваши VPN-подписки:\n");

    for (index, user) in users.iter().enumerate() {
        message.push_str(&format!(
            "\n{}. {}\n\
             Статус: {}\n\
             Действует до: {}\n\
             Ссылка подписки:\n{}\
             \n",
            index + 1,
            user.username,
            user.status,
            user.expire_at,
            user.subscription_url,
        ));
    }

    message
}