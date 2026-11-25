//! Thin client for the Woodpecker REST API.

use std::{fmt, time::Duration};

use anyhow::{Context, bail};
use reqwest::{
    Method, RequestBuilder, StatusCode, Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue, LOCATION},
    redirect,
};
use serde::{Serialize, de::DeserializeOwned};

use crate::{Config, model::Repo};

/// Step logs arrive in one response; this keeps a runaway build from exhausting memory.
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;
const MAX_ERROR_CHARS: usize = 300;

#[derive(Debug)]
pub enum Error {
    /// The server answered with a non-success status.
    Status { status: StatusCode, message: String },
    /// Anything else, already phrased for the model.
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Status { status, message } => {
                let hint = match *status {
                    StatusCode::UNAUTHORIZED => " The access token is missing, wrong or revoked.",
                    StatusCode::FORBIDDEN => " The token's user lacks permission for this.",
                    StatusCode::NOT_FOUND => {
                        " Check the repository (it must be enabled in Woodpecker), the pipeline \
                         number and the step id."
                    }
                    _ => "",
                };
                write!(f, "Woodpecker answered {status}.{hint}")?;
                if !message.is_empty() {
                    write!(f, " Server message: {message}")?;
                }
                Ok(())
            }
            Self::Other(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {}

pub struct Client {
    http: reqwest::Client,
    /// `<server>/api`
    base: Url,
}

pub struct Response {
    pub status: StatusCode,
    body: Vec<u8>,
}

impl Response {
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, Error> {
        serde_json::from_slice(&self.body).map_err(|error| {
            Error::Other(format!(
                "Unexpected response from Woodpecker ({error}). Check that `url` points at the \
                 Woodpecker server, including its subpath but without `/api`."
            ))
        })
    }
}

impl Client {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let url = config
            .url
            .as_deref()
            .context("`url` is required; set WOODPECKER_MCP_URL")?;
        let token = config
            .token
            .as_deref()
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .context("`token` is required; set WOODPECKER_MCP_TOKEN")?;

        // The parse error is not chained: it could echo credentials from the URL.
        let mut base = Url::parse(url).map_err(|_| {
            anyhow::anyhow!("`url` is not a valid URL, expected https://ci.example.com")
        })?;
        if !matches!(base.scheme(), "http" | "https") || base.host_str().is_none() {
            bail!("`url` must start with http:// or https://");
        }
        if !base.username().is_empty() || base.password().is_some() {
            bail!("`url` must not contain credentials; use `token`");
        }
        if base.query().is_some() || base.fragment().is_some() {
            bail!("`url` must not contain a query or fragment");
        }
        let local = matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if base.scheme() == "http" && !local {
            tracing::warn!("`url` uses plain http: the access token is sent unencrypted");
        }
        base.path_segments_mut()
            .map_err(|()| anyhow::anyhow!("`url` cannot be used as a base address"))?
            .pop_if_empty()
            .push("api");

        let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| anyhow::anyhow!("`token` contains characters that are not allowed"))?;
        authorization.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization);

        // Fails only when a provider is already installed, which is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(Duration::from_secs(config.timeout_seconds.max(1)))
            // A redirected POST would silently turn into a GET.
            .redirect(redirect::Policy::none())
            .build()
            .context("cannot create the HTTP client")?;

        Ok(Self { http, base })
    }

    /// Builds `<server>/api/<segments...>`, escaping every segment.
    fn url(&self, segments: &[&str]) -> Url {
        let mut url = self.base.clone();
        if let Ok(mut path) = url.path_segments_mut() {
            path.extend(segments);
        }
        url
    }

    pub fn request(&self, method: Method, segments: &[&str]) -> RequestBuilder {
        self.http.request(method, self.url(segments))
    }

    pub async fn send(&self, request: RequestBuilder) -> Result<Response, Error> {
        let mut response = request.send().await.map_err(transport_error)?;
        let status = response.status();

        if status.is_redirection() {
            let target = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("an unknown address");
            return Err(Error::Other(format!(
                "Woodpecker redirected to {target}. Set `url` to the final address."
            )));
        }

        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if body.len() + chunk.len() > MAX_BODY_BYTES {
                return Err(Error::Other(format!(
                    "The response is larger than {} MiB and was not read.",
                    MAX_BODY_BYTES / 1024 / 1024
                )));
            }
            body.extend_from_slice(&chunk);
        }

        if !status.is_success() {
            let message = String::from_utf8_lossy(&body)
                .trim()
                .chars()
                .take(MAX_ERROR_CHARS)
                .collect();
            return Err(Error::Status { status, message });
        }
        Ok(Response { status, body })
    }

    pub async fn get<T: DeserializeOwned>(&self, segments: &[&str]) -> Result<T, Error> {
        self.send(self.request(Method::GET, segments)).await?.json()
    }

    pub async fn get_query<T: DeserializeOwned>(
        &self,
        segments: &[&str],
        query: &impl Serialize,
    ) -> Result<T, Error> {
        self.send(self.request(Method::GET, segments).query(query))
            .await?
            .json()
    }

    /// Resolves `owner/name` or a numeric id to the repository id used in paths.
    pub async fn repo_id(&self, repo: &str) -> Result<String, Error> {
        let repo = repo.trim();
        if !repo.is_empty() && repo.chars().all(|c| c.is_ascii_digit()) {
            return Ok(repo.to_owned());
        }
        // GitLab subgroups give names with more than two parts.
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() < 2 || parts.iter().any(|p| matches!(*p, "" | "." | "..")) {
            return Err(Error::Other(format!(
                "Invalid repository `{repo}`: use `owner/name` or the numeric id."
            )));
        }
        let mut segments = vec!["repos", "lookup"];
        segments.extend(parts);
        let found: Repo = self.get(&segments).await?;
        Ok(found.id.to_string())
    }
}

fn transport_error(error: reqwest::Error) -> Error {
    // The source chain names the real cause, such as a certificate problem.
    let mut message = format!("Cannot reach Woodpecker: {error}");
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        message.push_str(&format!(": {cause}"));
        source = cause.source();
    }
    Error::Other(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(url: &str) -> anyhow::Result<Client> {
        Client::new(&Config {
            url: Some(url.to_owned()),
            token: Some("secret".to_owned()),
            ..Config::default()
        })
    }

    #[test]
    fn appends_api_to_the_server_address() {
        for (url, expected) in [
            ("https://ci.example.com", "https://ci.example.com/api/repos"),
            (
                "https://ci.example.com/",
                "https://ci.example.com/api/repos",
            ),
            (
                "https://example.com/ci/",
                "https://example.com/ci/api/repos",
            ),
        ] {
            assert_eq!(
                client(url).unwrap().url(&["repos"]).as_str(),
                expected,
                "{url}"
            );
        }
    }

    #[test]
    fn path_segments_cannot_escape() {
        let url = client("https://ci.example.com")
            .unwrap()
            .url(&["repos", "a/../b?x=1"]);
        assert_eq!(
            url.as_str(),
            "https://ci.example.com/api/repos/a%2F..%2Fb%3Fx=1"
        );
    }

    #[test]
    fn rejects_unusable_addresses_without_echoing_secrets() {
        for url in [
            "ci.example.com",
            "ftp://ci.example.com",
            "https://user:hunter2@ci.example.com",
            "https://ci.example.com?x=1",
        ] {
            let error = match client(url) {
                Ok(_) => panic!("{url} was accepted"),
                Err(error) => format!("{error:#}"),
            };
            assert!(!error.contains("hunter2"), "{error}");
        }
    }

    #[test]
    fn debug_output_hides_the_token() {
        let config = Config {
            token: Some("secret".to_owned()),
            ..Config::default()
        };
        assert!(!format!("{config:?}").contains("secret"));
    }
}
