// figment::Jail closures must return figment::Result, whose error type is large
#![allow(clippy::result_large_err)]

use figment::{Jail, providers::Env};
use serde::Deserialize;

use dco2::github::AppConfig;

use super::env_with_raw_keys;

#[test]
fn flat_raw_key_is_loaded_as_trimmed_string() {
    for (value, expected_value) in [("0123", "0123"), ("true", "true"), (" 0123\n", "0123")] {
        Jail::expect_with(|jail| {
            // Setup environment as the lambda does (flat keys)
            jail.clear_env();
            jail.set_env("DCO2_APP_ID", "1234");
            jail.set_env("DCO2_PRIVATE_KEY", "key");
            jail.set_env("DCO2_WEBHOOK_SECRET", value);

            // Load the configuration
            let cfg: AppConfig = env_with_raw_keys(&Env::prefixed("DCO2_"), &["webhook_secret"]).extract()?;

            // Check the raw key is kept as a string and other keys are parsed
            assert_eq!(
                cfg,
                AppConfig {
                    api_host: None,
                    app_id: 1234,
                    private_key: "key".to_string(),
                    webhook_secret: expected_value.to_string(),
                },
                "value: {value:?}"
            );
            Ok(())
        });
    }
}

#[test]
fn nested_raw_key_is_loaded_as_trimmed_string() {
    for (value, expected_value) in [("0123", "0123"), ("true", "true"), (" 0123\n", "0123")] {
        Jail::expect_with(|jail| {
            // Setup environment as the server does (nested keys)
            jail.clear_env();
            jail.set_env("DCO2_GITHUB_APP__APP_ID", "1234");
            jail.set_env("DCO2_GITHUB_APP__PRIVATE_KEY", "key");
            jail.set_env("DCO2_GITHUB_APP__WEBHOOK_SECRET", value);

            // Load the configuration
            let env = Env::prefixed("DCO2_").split("__");
            let cfg: Wrapper = env_with_raw_keys(&env, &["github_app.webhook_secret"]).extract()?;

            // Check the raw key is kept as a string and other keys are parsed
            assert_eq!(
                cfg.github_app,
                AppConfig {
                    api_host: None,
                    app_id: 1234,
                    private_key: "key".to_string(),
                    webhook_secret: expected_value.to_string(),
                },
                "value: {value:?}"
            );
            Ok(())
        });
    }
}

#[test]
fn raw_key_wrapping_quotes_are_removed() {
    for (value, expected_value) in [
        ("\"0123\"", "0123"),
        (" \"0123\"\n", "0123"),
        ("\" 0123 \"", " 0123 "),
        ("\"\"0123\"\"", "\"0123\""),
        ("\"0123", "\"0123"),
        ("01\"23", "01\"23"),
        ("\"", "\""),
    ] {
        Jail::expect_with(|jail| {
            // Setup environment with a raw key that may be wrapped in quotes
            jail.clear_env();
            jail.set_env("DCO2_APP_ID", "1234");
            jail.set_env("DCO2_PRIVATE_KEY", "key");
            jail.set_env("DCO2_WEBHOOK_SECRET", value);

            // Load the configuration
            let cfg: AppConfig = env_with_raw_keys(&Env::prefixed("DCO2_"), &["webhook_secret"]).extract()?;

            // Check only the quotes wrapping the value were removed
            assert_eq!(cfg.webhook_secret, expected_value, "value: {value:?}");
            Ok(())
        });
    }
}

// Helpers.

/// Configuration with the GitHub app settings nested under a key.
#[derive(Debug, Deserialize)]
struct Wrapper {
    github_app: AppConfig,
}
