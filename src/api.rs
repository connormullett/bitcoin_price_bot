use lazy_static::lazy_static;
use log::info;
use redis::Commands;
use serde::{Deserialize, Serialize};
use thiserror::Error;

lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::new();
    // TODO: Change this to a custom client
    static ref REDIS: redis::Client =
        redis::Client::open("redis://127.0.0.1/").expect("failed to connect to redis");
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    Cache(#[from] redis::RedisError),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
}

#[derive(Deserialize)]
pub struct ExchangeRateData {
    pub time: String,
    pub asset_id_base: String,
    pub asset_id_quote: String,
    pub rate: f64,
}

#[derive(Deserialize, Serialize)]
pub struct ExchangeRate {
    pub time: String,
    pub rate: f64,
}

pub async fn get_price() -> Result<ExchangeRate, ApiError> {
    info!("getting redis connection");
    let mut connection = REDIS.get_connection()?;
    let maybe_value = connection.get::<&str, String>("bitcoin_exchange_prices");

    let value = match maybe_value {
        Ok(v) => Some(v),
        Err(e) if e.kind() == redis::ErrorKind::TypeError => None,
        Err(e) => return Err(ApiError::Cache(e)),
    };

    if let Some(v) = value {
        info!("found key in cache");
        let cache_entry: ExchangeRate = serde_json::from_str(&v)?;
        return Ok(cache_entry);
    }

    let exchange_rate = get_price_raw().await?;

    let new_value = serde_json::to_string(&exchange_rate)?;

    info!("setting key in cache");
    connection.set_ex("bitcoin_exchange_prices", new_value, 3_600 * 2)?;

    Ok(exchange_rate)
}

pub async fn get_price_raw() -> Result<ExchangeRate, ApiError> {
    info!("sending request to coin api");
    let api_key = std::env::var("COIN_API_KEY").expect("api key is not set");
    let res = CLIENT
        .get("https://rest.coinapi.io/v1/exchangerate/BTC/USD")
        .header("X-CoinAPI-Key", api_key)
        .send()
        .await?
        .error_for_status()?;

    let rate_response = res.json::<ExchangeRateData>().await?;
    let exchange_rate = ExchangeRate {
        time: rate_response.time,
        rate: rate_response.rate,
    };

    Ok(exchange_rate)
}

pub async fn set_cache_price(new_price: ExchangeRate) -> () {
    let mut connection = REDIS
        .get_connection()
        .expect("failed to get redis connection");
    let value = serde_json::to_string(&new_price).expect("failed to serialize data");
    connection
        .set_ex::<String, String, ()>("bitcoin_exchange_prices".into(), value, 3600 * 2)
        .expect("failed to set data in redis");
}
