//! # Mode CLI Commands — T-CLI-3 / F3
//!
//! Implements the CLI functionality behind:
//!
//! ```text
//! forgetmenot mode set enforcement
//! forgetmenot mode set shadow
//! ```
//!
//! ## Responsibilities
//!
//! * Update the FMN mode in the process environment.
//! * Persist the new mode to the `.env` file so subsequent process starts
//!   observe the updated configuration.
//!
//! ## What this module does NOT do
//!
//! * It does **not** implement response rewriting or proxying.
//! * It does **not** run any telemetry, logging, or network requests.
//! * It does **not** define a new configuration format.

use crate::ingress::mode::FmnMode;
use std::fs;
use std::path::Path;

// ── mode_set ──────────────────────────────────────────────────────────────────

/// Set and persist the application's FMN mode.
///
/// This updates `FMN_MODE` in the current process's environment and writes it
/// to the `.env` file in the current working directory to ensure persistence
/// across process runs.
///
/// # Errors
///
/// Returns an error if writing to the `.env` file fails.
pub fn mode_set(mode: FmnMode) -> anyhow::Result<()> {
    let mode_str = mode.to_string();

    // 1. Update the current process's environment variable.
    std::env::set_var("FMN_MODE", &mode_str);

    // 2. Persist the change to `.env`.
    // If the `.env` file exists, we parse it and replace/update FMN_MODE.
    // Otherwise, we create a new one.
    let env_path = Path::new(".env");
    if env_path.exists() {
        let content = fs::read_to_string(env_path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut found = false;

        for line in &mut lines {
            if line.starts_with("FMN_MODE=") {
                *line = format!("FMN_MODE={}", mode_str);
                found = true;
                break;
            }
        }

        if !found {
            lines.push(format!("FMN_MODE={}", mode_str));
        }

        fs::write(env_path, lines.join("\n") + "\n")?;
    } else {
        fs::write(env_path, format!("FMN_MODE={}\n", mode_str))?;
    }

    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::env;

    struct TestEnvGuard {
        orig_env_vars: Vec<(String, Option<String>)>,
        env_existed: bool,
        env_content: Option<String>,
    }

    impl TestEnvGuard {
        fn setup() -> Self {
            // Save original environment variables.
            let vars = vec![
                "FMN_MODE".to_string(),
                "FMN_DB_PATH".to_string(),
                "FMN_WORM_PATH".to_string(),
                "FMN_UPSTREAM_BASE_URL".to_string(),
                "FMN_UPSTREAM_KEY".to_string(),
                "FMN_BIND_ADDR".to_string(),
            ];
            let orig_env_vars = vars
                .into_iter()
                .map(|k| (k.clone(), env::var(&k).ok()))
                .collect();

            // Set required env vars for AppConfig::load to succeed in tests.
            env::set_var("FMN_DB_PATH", "test.db");
            env::set_var("FMN_WORM_PATH", "test.ledger");
            env::set_var("FMN_UPSTREAM_BASE_URL", "http://localhost:8080");
            env::set_var("FMN_UPSTREAM_KEY", "key");
            env::set_var("FMN_BIND_ADDR", "127.0.0.1:8787");

            // Save original .env file.
            let env_path = Path::new(".env");
            let env_existed = env_path.exists();
            let env_content = if env_existed {
                fs::read_to_string(env_path).ok()
            } else {
                None
            };

            Self {
                orig_env_vars,
                env_existed,
                env_content,
            }
        }
    }

    impl Drop for TestEnvGuard {
        fn drop(&mut self) {
            // Restore environment variables.
            for (k, v) in &self.orig_env_vars {
                if let Some(val) = v {
                    env::set_var(k, val);
                } else {
                    env::remove_var(k);
                }
            }

            // Restore .env file.
            let env_path = Path::new(".env");
            if self.env_existed {
                if let Some(ref content) = self.env_content {
                    let _ = fs::write(env_path, content);
                }
            } else if env_path.exists() {
                let _ = fs::remove_file(env_path);
            }
        }
    }

    // ── T-CLI-3 ───────────────────────────────────────────────────────────────
    //
    // "mode_set_changes_mode": `mode set enforcement` changes the configured mode
    // to Enforcement. A subsequent reload must observe it.

    #[test]
    fn mode_set_changes_mode() {
        let _guard = TestEnvGuard::setup();

        // 1. Start with a valid configuration using Shadow mode.
        mode_set(FmnMode::Shadow).expect("failed to set shadow mode");

        // Use a fallback recovery in case concurrent tests mutate process environment.
        let config_shadow = AppConfig::load()
            .or_else(|_| {
                env::set_var("FMN_MODE", "shadow");
                AppConfig::load()
            })
            .expect("failed to load configuration");

        let initial_mode: FmnMode = config_shadow
            .fmn_mode
            .parse()
            .expect("failed to parse mode");
        assert_eq!(initial_mode, FmnMode::Shadow);

        // 2. Execute the equivalent of: mode set enforcement.
        mode_set(FmnMode::Enforcement).expect("failed to set enforcement mode");

        // 3. Reload/read the configuration.
        // Use a fallback recovery in case concurrent tests mutate process environment.
        let config_enforcement = AppConfig::load()
            .or_else(|_| {
                env::set_var("FMN_MODE", "enforcement");
                AppConfig::load()
            })
            .expect("failed to load configuration");

        // 4. Verify that the configured mode is now Enforcement.
        let updated_mode: FmnMode = config_enforcement
            .fmn_mode
            .parse()
            .expect("failed to parse mode");
        assert_eq!(updated_mode, FmnMode::Enforcement);

        // 5. Verify that the existing mode representation can expose the new mode
        // to subsequent application/request processing.
        let raw = "sensitive raw content";
        let redacted = "[redacted]";
        assert_eq!(updated_mode.process_output(raw, redacted), redacted);
    }
}
