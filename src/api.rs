use log::debug;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::redis::RedisClient;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    Cache(#[from] redis::RedisError),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    Serialize(#[from] serde_json::Error),
    #[error("an error occurred, {0}")]
    Generic(String),
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

pub struct ApiHandler {
    http_client: reqwest::Client,
    redis_client: RedisClient,
}

const REDIS_KEY: &str = "bitcoin_exchange_price";
const TWO_HOURS: usize = 3600 * 2;

impl ApiHandler {
    pub async fn new() -> Result<Self, ApiError> {
        let http_client = reqwest::Client::new();
        let connection_string = std::env::var("REDIS_HOST").expect("REDIS_HOST was not set");
        let redis_client = RedisClient::new(connection_string).await?;
        Ok(Self {
            http_client,
            redis_client,
        })
    }

    pub async fn get_price(&self) -> Result<ExchangeRate, ApiError> {
        let value = self.redis_client.get(REDIS_KEY).await?;

        if let Some(v) = value {
            debug!("found key in cache");
            let cache_entry: ExchangeRate = serde_json::from_str(&v)?;
            return Ok(cache_entry);
        }

        debug!("getting price from CoinAPI");
        let exchange_rate = self.get_price_raw().await?;

        let new_value = serde_json::to_string(&exchange_rate)?;

        debug!("setting key in cache");
        self.redis_client
            .set(REDIS_KEY, &new_value, 3_600 * 2)
            .await?;

        Ok(exchange_rate)
    }

    pub async fn get_price_raw(&self) -> Result<ExchangeRate, ApiError> {
        debug!("sending request to coin api");
        let api_key = std::env::var("COIN_API_KEY").expect("api key is not set");
        let res = self
            .http_client
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

    pub async fn set_cache_price(&self, new_price: ExchangeRate) -> Result<(), ApiError> {
        let value = serde_json::to_string(&new_price).expect("failed to serialize data");

        self.redis_client.set(REDIS_KEY, &value, TWO_HOURS).await?;
        Ok(())
    }
}
