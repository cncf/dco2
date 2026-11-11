// figment::Jail closures must return figment::Result, whose error type is large
#![allow(clippy::result_large_err)]

use std::path::PathBuf;

use figment::Jail;
use indoc::indoc;

use dco2::github::AppConfig;

use super::{Config, LogFormat};

/// Name of the configuration file created in the tests.
const CONFIG_FILE: &str = "dco2-server.yml";

#[test]
fn chart_rendered_config_parses() {
    Jail::expect_with(|jail| {
        // Setup config file as rendered by the chart's server secret template
        jail.clear_env();
        jail.create_file(
            CONFIG_FILE,
            indoc! {r#"
                github_app:
                  api_host: ghe.example.com
                  app_id: 1234
                  private_key: |-

                    -----BEGIN RSA PRIVATE KEY-----
                    key
                    -----END RSA PRIVATE KEY-----
                  webhook_secret: "0123"
                log_format: pretty
                server_addr: 0.0.0.0:9000
            "#},
        )?;

        // Load the configuration
        let cfg = Config::new(Some(&PathBuf::from(CONFIG_FILE))).unwrap();

        // Check all the values were loaded from the config file
        assert_eq!(
            cfg,
            Config {
                github_app: AppConfig {
                    api_host: Some("ghe.example.com".to_string()),
                    app_id: 1234,
                    private_key: "\n-----BEGIN RSA PRIVATE KEY-----\nkey\n-----END RSA PRIVATE KEY-----"
                        .to_string(),
                    webhook_secret: "0123".to_string(),
                },
                log_format: LogFormat::Pretty,
                server_addr: "0.0.0.0:9000".to_string(),
            }
        );
        Ok(())
    });
}

#[test]
fn empty_webhook_secret_is_rejected() {
    Jail::expect_with(|jail| {
        // Setup environment with an empty webhook secret
        jail.clear_env();
        set_github_app_env(jail);
        jail.set_env("DCO2_GITHUB_APP__WEBHOOK_SECRET", "");

        // Load the configuration and check it is rejected
        let err = Config::new(None).unwrap_err();
        assert_eq!(
            format!("{err:#}"),
            "invalid configuration: github app webhook secret cannot be empty"
        );
        Ok(())
    });
}

#[test]
fn env_vars_override_config_file() {
    Jail::expect_with(|jail| {
        // Setup config file and environment variables overriding some values
        jail.clear_env();
        jail.create_file(
            CONFIG_FILE,
            indoc! {r#"
                github_app:
                  app_id: 1234
                  private_key: key
                  webhook_secret: "secret"
                log_format: pretty
                server_addr: 0.0.0.0:9000
            "#},
        )?;
        jail.set_env("DCO2_GITHUB_APP__WEBHOOK_SECRET", "env-secret");
        jail.set_env("DCO2_LOG_FORMAT", "json");
        jail.set_env("DCO2_SERVER_ADDR", "127.0.0.1:1");

        // Load the configuration
        let cfg = Config::new(Some(&PathBuf::from(CONFIG_FILE))).unwrap();

        // Check the environment values (nested ones included) took precedence
        assert_eq!(
            cfg,
            Config {
                github_app: AppConfig {
                    api_host: None,
                    app_id: 1234,
                    private_key: "key".to_string(),
                    webhook_secret: "env-secret".to_string(),
                },
                log_format: LogFormat::Json,
                server_addr: "127.0.0.1:1".to_string(),
            }
        );
        Ok(())
    });
}

#[test]
fn missing_config_file_is_rejected() {
    Jail::expect_with(|jail| {
        // Setup environment with all the required values
        jail.clear_env();
        set_github_app_env(jail);
        let config_file = PathBuf::from(CONFIG_FILE);

        // Check a config file that does not exist is rejected
        let err = Config::new(Some(&config_file)).unwrap_err();
        assert!(err.to_string().contains(CONFIG_FILE), "{err:#}");

        // Check parent directories are not searched for the config file
        jail.create_file(CONFIG_FILE, "log_format: pretty")?;
        jail.change_dir(jail.create_dir("subdir")?)?;
        let err = Config::new(Some(&config_file)).unwrap_err();
        assert!(err.to_string().contains(CONFIG_FILE), "{err:#}");
        Ok(())
    });
}

#[test]
fn quoted_webhook_secret_in_config_file_is_rejected() {
    Jail::expect_with(|jail| {
        // Setup config file with a webhook secret wrapped in quotes
        jail.clear_env();
        jail.create_file(
            CONFIG_FILE,
            indoc! {r#"
                github_app:
                  app_id: 1234
                  private_key: key
                  webhook_secret: '"0123"'
            "#},
        )?;

        // Load the configuration and check it is rejected
        let err = Config::new(Some(&PathBuf::from(CONFIG_FILE))).unwrap_err();
        assert_eq!(
            format!("{err:#}"),
            "invalid configuration: github app webhook secret cannot be wrapped in quotes"
        );
        Ok(())
    });
}

#[test]
fn webhook_secret_env_var_is_trimmed_and_unquoted_but_not_parsed() {
    for (webhook_secret, expected_webhook_secret) in [
        ("0123", "0123"),
        ("123456", "123456"),
        ("true", "true"),
        (" secret\n", "secret"),
        ("\"0123\"", "0123"),
    ] {
        Jail::expect_with(|jail| {
            // Setup environment with a webhook secret figment would parse
            jail.clear_env();
            set_github_app_env(jail);
            jail.set_env("DCO2_GITHUB_APP__WEBHOOK_SECRET", webhook_secret);

            // Load the configuration
            let cfg = Config::new(None).unwrap();

            // Check the webhook secret is kept as a string and the app id is parsed
            assert_eq!(
                cfg.github_app,
                AppConfig {
                    api_host: None,
                    app_id: 1234,
                    private_key: "key".to_string(),
                    webhook_secret: expected_webhook_secret.to_string(),
                },
                "webhook secret: {webhook_secret:?}"
            );
            Ok(())
        });
    }
}

// Helpers.

/// Set the environment variables with the required GitHub app settings.
fn set_github_app_env(jail: &mut Jail) {
    jail.set_env("DCO2_GITHUB_APP__APP_ID", "1234");
    jail.set_env("DCO2_GITHUB_APP__PRIVATE_KEY", "key");
    jail.set_env("DCO2_GITHUB_APP__WEBHOOK_SECRET", "secret");
}
