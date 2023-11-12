use log::{error, warn};
use redis::{aio::ConnectionManager, AsyncCommands, RedisError, RedisResult};

use crate::api::ApiError;

pub struct RedisClient {
    connection_manager: ConnectionManager,
}

impl RedisClient {
    pub async fn new(connection_string: String) -> Result<Self, ApiError> {
        let client = redis::Client::open(connection_string)?;

        let mut retries = 3;
        let connection_manager = loop {
            if retries == 0 {
                return Err(ApiError::Generic("failed to open redis connection".into()));
            }
            if let Ok(manager) = client.get_tokio_connection_manager().await {
                break manager;
            }

            retries -= 1;
            warn!("failed to connect to redis, retrying {retries} more times...");
        };

        Ok(Self { connection_manager })
    }

    pub async fn set(&self, key: &str, value: &str, expiration: usize) -> Result<(), RedisError> {
        let mut connection = self.connection_manager.clone();
        connection.set_ex(key, value, expiration).await?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> RedisResult<Option<String>> {
        let mut connection = self.connection_manager.clone();
        match connection.get(key).await {
            Ok(value) => Ok(Some(value)),
            Err(e) => {
                if e.kind() == redis::ErrorKind::TypeError {
                    Ok(None)
                } else {
                    error!("error occurred while fetching key {key}: {e}");
                    Err(e)
                }
            }
        }
    }
}
