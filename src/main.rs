use std::time::Duration;

use lazy_static::lazy_static;
use log::{error, info};
use teloxide::{prelude::*, utils::command::BotCommands};

mod api;
mod redis;

lazy_static! {
    static ref CHAT_ID: i64 = std::env::var("CHAT_ID")
        .expect("CHAT_ID is not set")
        .parse()
        .expect("CHAT_ID was not an int");
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting command bot...");

    let bot = Bot::from_env();

    let timer_bot = bot.clone();
    let api_handler = api::ApiHandler::new()
        .await
        .expect("error occurred when creating api handler");

    tokio::spawn(async move {
        info!("starting timer thread");
        let sleep = tokio::time::sleep(Duration::from_secs(3600));
        tokio::pin!(sleep);

        loop {
            tokio::select! {
                () = &mut sleep => {
                    info!("timer elapsed");
                    let Ok(new_price) = api_handler.get_price_raw().await else {
                        error!("failed to get new price");
                        continue;
                    };
                    match api_handler.get_price().await {
                        Ok(price) => {
                            let percent = (new_price.rate / price.rate) * 100.0;
                            let delta = if new_price.rate > price.rate { "up" } else { "down"};
                            if percent.abs() > 3.0 {
                                let message = format!("BTC now at ${}, {}% {}", new_price.rate.round() as i32, percent, delta);
                                if let Err(e) = timer_bot.send_message(ChatId(*CHAT_ID), message).await {
                                    error!("failed to send message to telegram: {e}");
                                };
                            }
                        },
                        Err(e) => {
                            error!("error occurred while getting price: {e}");
                        }
                    };
                    if let Err(e) = api_handler.set_cache_price(new_price).await {
                        error!("error occurred while setting price {e}");
                    };
                    sleep.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(3600));
                },
            }
        }
    });

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Check status of bot")]
    Health,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Health => bot.send_message(msg.chat.id, "Status: OK").await?,
    };
    Ok(())
}
