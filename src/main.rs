use std::time::Duration;

use api::ApiHandler;
use chrono::{Timelike, Utc};
use lazy_static::lazy_static;
use log::{error, info, LevelFilter};
use teloxide::{prelude::*, utils::command::BotCommands};

mod api;
mod redis;

lazy_static! {
    static ref CHAT_IDS: Vec<i64> = std::env::var("CHAT_ID")
        .expect("CHAT_ID is not set")
        .split(',')
        .map(|i| i.parse().expect("chat id wasn't an integer"))
        .collect();
}

fn parse_filter(log_level: &str) -> LevelFilter {
    match log_level {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => panic!("invalid log level"),
    }
}

#[tokio::main]
async fn main() {
    let log_level = std::env::var("RUST_LOG").expect("RUST_LOG not set");
    pretty_env_logger::formatted_builder()
        .filter_level(parse_filter(&log_level))
        .init();
    log::set_max_level(log::LevelFilter::Debug);

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
                            if percent.abs() > 3.0 || Utc::now().hour() % 11 == 0 {
                                let message = format!("BTC now at ${}, {}% {}", new_price.rate.round() as i32, percent, delta);
                                for chat_id in CHAT_IDS.iter() {
                                    if let Err(e) = timer_bot.send_message(ChatId(*chat_id), message.clone()).await {
                                        error!("failed to send message to telegram: {e}");
                                    };
                                }
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
    #[command(description = "Check current price in USD")]
    Price,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Health => bot.send_message(msg.chat.id, "Status: OK").await?,
        Command::Price => {
            // I hate expect() but this error type is a pain to deal with
            let api_handler = ApiHandler::new()
                .await
                .expect("failed to create api handler");

            let price = api_handler
                .get_price()
                .await
                .expect("failed to get price in `answer`");

            bot.send_message(msg.chat.id, format!("${}", price.rate))
                .await?
        }
    };
    Ok(())
}
