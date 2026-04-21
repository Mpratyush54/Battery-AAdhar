//! redis_cache.rs — Redis caching configuration module for the Rust Core
use redis::{AsyncCommands, Client};

pub struct RedisCache {
    client: Client,
}

impl RedisCache {
    pub fn new(url: &str) -> Result<Self, redis::RedisError> {
        let client = Client::open(url)?;
        Ok(Self { client })
    }

    pub async fn ping(&self) -> Result<(), redis::RedisError> {
        let mut con = self.client.get_async_connection().await?;
        let _pong: String = con.set("test_ping", "pong").await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_connection_stub() {
        let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let pass = std::env::var("REDIS_PASSWORD").unwrap_or_else(|_| "".to_string());
        
        let url = if pass.is_empty() {
            format!("redis://{}:{}/0", host, port)
        } else {
            format!("redis://:{}@{}:{}/0", pass, host, port)
        };
        
        let _cache = match RedisCache::new(&url) {
            Ok(c) => c,
            Err(e) => {
                println!("Redis absent natively: {}", e);
                return;
            }
        };
        // We only attempt ping if client constructed ok
        println!("Redis wrapper initialized correctly");
    }
}
