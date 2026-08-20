use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub fmn_mode: String,
    pub fmn_db_path: String,
    pub fmn_worm_path: String,
    pub fmn_upstream_base_url: String,
    pub fmn_upstream_key: String,
    pub fmn_bind_addr: String,
}

impl AppConfig {
    #[must_use]
    pub fn load() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        // Ignore missing .env and fall back to process environment.

        let fmn_mode = std::env::var("FMN_MODE")
            .map_err(|_| anyhow::anyhow!("Missing required environment variable FMN_MODE"))?;
        let fmn_db_path = std::env::var("FMN_DB_PATH")
            .map_err(|_| anyhow::anyhow!("Missing required environment variable FMN_DB_PATH"))?;
        let fmn_worm_path = std::env::var("FMN_WORM_PATH")
            .map_err(|_| anyhow::anyhow!("Missing required environment variable FMN_WORM_PATH"))?;
        let fmn_upstream_base_url = std::env::var("FMN_UPSTREAM_BASE_URL").map_err(|_| {
            anyhow::anyhow!("Missing required environment variable FMN_UPSTREAM_BASE_URL")
        })?;
        let fmn_upstream_key = std::env::var("FMN_UPSTREAM_KEY").map_err(|_| {
            anyhow::anyhow!("Missing required environment variable FMN_UPSTREAM_KEY")
        })?;
        let fmn_bind_addr = std::env::var("FMN_BIND_ADDR")
            .map_err(|_| anyhow::anyhow!("Missing required environment variable FMN_BIND_ADDR"))?;

        Ok(Self {
            fmn_mode,
            fmn_db_path,
            fmn_worm_path,
            fmn_upstream_base_url,
            fmn_upstream_key,
            fmn_bind_addr,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: [&str; 6] = [
        "FMN_MODE",
        "FMN_DB_PATH",
        "FMN_WORM_PATH",
        "FMN_UPSTREAM_BASE_URL",
        "FMN_UPSTREAM_KEY",
        "FMN_BIND_ADDR",
    ];

    struct EnvGuard {
        original: Vec<(String, Option<String>)>,
    }

    impl EnvGuard {
        fn new() -> Self {
            let original = KEYS
                .iter()
                .map(|&k| (k.to_string(), std::env::var(k).ok()))
                .collect();
            Self { original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in &self.original {
                if let Some(val) = v {
                    std::env::set_var(k, val);
                } else {
                    std::env::remove_var(k);
                }
            }
        }
    }

    fn set_valid_env() {
        std::env::set_var("FMN_MODE", "shadow");
        std::env::set_var("FMN_DB_PATH", "fmn.db");
        std::env::set_var("FMN_WORM_PATH", "fmn.ledger");
        std::env::set_var("FMN_UPSTREAM_BASE_URL", "https://api.openai.com/v1");
        std::env::set_var("FMN_UPSTREAM_KEY", "sk-testkey");
        std::env::set_var("FMN_BIND_ADDR", "127.0.0.1:8787");
    }

    #[test]
    fn config_loads_from_env() {
        let _guard = EnvGuard::new();
        set_valid_env();

        let config = AppConfig::load();
        assert!(config.is_ok());

        let cfg = config.unwrap();
        assert_eq!(cfg.fmn_mode, "shadow");
        assert_eq!(cfg.fmn_db_path, "fmn.db");
        assert_eq!(cfg.fmn_worm_path, "fmn.ledger");
        assert_eq!(cfg.fmn_upstream_base_url, "https://api.openai.com/v1");
        assert_eq!(cfg.fmn_upstream_key, "sk-testkey");
        assert_eq!(cfg.fmn_bind_addr, "127.0.0.1:8787");
    }

    #[test]
    fn test_missing_variable_returns_error() {
        let _guard = EnvGuard::new();
        set_valid_env();

        // Remove one required variable
        std::env::remove_var("FMN_MODE");

        let config = AppConfig::load();
        assert!(config.is_err());
        assert!(config.unwrap_err().to_string().contains("FMN_MODE"));
    }

    #[test]
    fn test_values_read_correctly() {
        let _guard = EnvGuard::new();

        std::env::set_var("FMN_MODE", "enforcement");
        std::env::set_var("FMN_DB_PATH", "test.db");
        std::env::set_var("FMN_WORM_PATH", "test.ledger");
        std::env::set_var("FMN_UPSTREAM_BASE_URL", "http://localhost:8080");
        std::env::set_var("FMN_UPSTREAM_KEY", "secret-key-123");
        std::env::set_var("FMN_BIND_ADDR", "0.0.0.0:9999");

        let config = AppConfig::load().unwrap();

        assert_eq!(config.fmn_mode, "enforcement");
        assert_eq!(config.fmn_db_path, "test.db");
        assert_eq!(config.fmn_worm_path, "test.ledger");
        assert_eq!(config.fmn_upstream_base_url, "http://localhost:8080");
        assert_eq!(config.fmn_upstream_key, "secret-key-123");
        assert_eq!(config.fmn_bind_addr, "0.0.0.0:9999");
    }
}
