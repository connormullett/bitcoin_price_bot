use std::time::Duration;

use lazy_static::lazy_static;
use log::{error, info};
use teloxide::{prelude::*, utils::command::BotCommands};

mod api;

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
    tokio::spawn(async move {
        info!("starting timer thread");
        let sleep = tokio::time::sleep(Duration::from_secs(3600));
        tokio::pin!(sleep);

        loop {
            tokio::select! {
                () = &mut sleep => {
                    info!("timer elapsed");
                    let new_price = api::get_price_raw().await.expect("failed to get price");
                    match api::get_price().await {
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
                    api::set_cache_price(new_price).await;
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
    #[command(description = "display this text.")]
    Help,
    #[command(description = "check the current Bitcoin price")]
    Price,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Price => {
            let message = match api::get_price().await {
                Ok(price) => format!("${}", price.rate.round() as i32),
                Err(e) => {
                    error!("error occurred while getting price: {e}");
                    "something went wrong while getting the price of bitcoin D:".into()
                }
            };

            bot.send_message(msg.chat.id, message).await?
        }
    };

    Ok(())
}
