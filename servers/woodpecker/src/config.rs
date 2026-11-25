use std::fmt;

use serde::Deserialize;

/// Settings, see `config.example.toml`.
#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Address of the Woodpecker server, including its subpath if it has one.
    pub url: Option<String>,
    /// Personal access token, from the "CLI & API" page of the Woodpecker UI.
    pub token: Option<String>,
    /// Hide every tool that changes something.
    pub read_only: bool,
    /// Default number of log lines a tool returns.
    pub max_log_lines: usize,
    /// Timeout of one request to Woodpecker.
    pub timeout_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: None,
            token: None,
            read_only: true,
            max_log_lines: 200,
            timeout_seconds: 30,
        }
    }
}

/// Written by hand so the token can never end up in a log.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("url", &self.url)
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("read_only", &self.read_only)
            .field("max_log_lines", &self.max_log_lines)
            .field("timeout_seconds", &self.timeout_seconds)
            .finish()
    }
}
