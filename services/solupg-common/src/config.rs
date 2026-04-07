use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SolanaConfig {
    pub rpc_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://solupg:solupg_dev@localhost:5432/solupg".to_string(),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
        }
    }
}

impl Default for SolanaConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://localhost:8899".to_string(),
        }
    }
}
