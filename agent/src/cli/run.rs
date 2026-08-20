//! # CLI Run Command — Owner A
//!
//! Entry point for starting the ForgetMeNot Agent via the CLI.
//!
//! Calls the shared application build/startup pipeline to bind the Axum router
//! and launch the web server.

use crate::config::AppConfig;

/// CLI run command entry point.
///
/// Loads the configuration and starts the shared agent runtime.
///
/// # Errors
///
/// Propagates configuration load errors, or server binding/runtime errors.
pub async fn run() -> anyhow::Result<()> {
    // 1. Load the existing application configuration.
    let config = AppConfig::load()?;

    // 2. Start the shared runtime.
    crate::build_and_run(config).await
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::Duration;

    #[tokio::test]
    async fn test_run_command_starts_app() {
        // Arrange: Setup configuration using environment variables and high test port
        env::set_var("FMN_MODE", "shadow");
        env::set_var("FMN_DB_PATH", ":memory:");
        env::set_var("FMN_WORM_PATH", "test_run.ledger");
        env::set_var("FMN_UPSTREAM_BASE_URL", "http://localhost");
        env::set_var("FMN_UPSTREAM_KEY", "key");
        env::set_var("FMN_BIND_ADDR", "127.0.0.1:18291");

        // Act: Start the server in a background task
        let handle = tokio::spawn(async {
            if let Err(e) = run().await {
                eprintln!("SERVER RUN ERROR: {:?}", e);
            }
        });

        // Give it a moment to bind and start serving
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Query the health endpoint
        let client = reqwest::Client::new();
        let res = client.get("http://127.0.0.1:18291/health").send().await;

        // Cleanup: Abort the server task
        handle.abort();

        // Assert
        assert!(
            res.is_ok(),
            "Failed to connect to the server: {:?}",
            res.err()
        );
        let response = res.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body, serde_json::json!({"status": "ok"}));
    }
}
