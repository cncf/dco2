//! This module provides helpers to load configuration from environment
//! variables.

use figment::{
    Figment,
    providers::{Env, Serialized},
};

#[cfg(test)]
mod tests;

/// Create a provider that loads the environment variables selected by `env`,
/// keeping the values of the `raw_keys` provided as strings.
///
/// Figment parses the values of environment variables, so a secret like `0123`
/// would become the number `123`, and `true` a boolean. The values of the raw
/// keys bypass that parsing and are always loaded as strings. As figment does
/// when parsing values, leading and trailing whitespace is trimmed and the
/// double quotes wrapping a value are removed.
pub fn env_with_raw_keys(env: &Env, raw_keys: &[&str]) -> Figment {
    // Load the values of all keys but the raw ones, parsing them
    let parsed = Figment::from(env.clone().ignore(raw_keys));

    // Load the values of the raw keys as strings
    env.clone().only(raw_keys).iter().fold(parsed, |figment, (key, value)| {
        figment.merge(Serialized::default(key.as_str(), raw_value(&value)))
    })
}

/// Prepare the value of a raw key: leading and trailing whitespace is trimmed
/// and, when the value is wrapped in double quotes, they are removed. The rest
/// of the value is kept as is (escape sequences are not processed).
fn raw_value(value: &str) -> &str {
    let value = value.trim();
    value.strip_prefix('"').and_then(|unquoted| unquoted.strip_suffix('"')).unwrap_or(value)
}
