//! Typed helpers for sending Chrome DevTools Protocol commands through
//! `reach-browserd`.
//!
//! The crate keeps CDP command construction in Rust types while preserving an
//! escape hatch through [`RawCdpCommand`] for protocol methods that do not have
//! typed wrappers yet.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use tracing::{debug, trace};

/// Typed command definitions for the CDP methods Reach currently uses.
pub mod commands;

const DEFAULT_BROWSERD_URL: &str = "http://127.0.0.1:8401";

/// HTTP client for the `reach-browserd` CDP bridge.
#[derive(Debug, Clone)]
pub struct CdpClient {
    http: reqwest::Client,
    browserd_url: String,
}

impl CdpClient {
    /// Create a client for a `reach-browserd` base URL.
    pub fn new(browserd_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            browserd_url: trim_trailing_slash(browserd_url.into()),
        }
    }

    /// Create a client for the default local `reach-browserd` endpoint.
    pub fn localhost() -> Self {
        Self::new(DEFAULT_BROWSERD_URL)
    }

    /// Return the normalized `reach-browserd` base URL.
    pub fn browserd_url(&self) -> &str {
        &self.browserd_url
    }

    /// Create a client and verify that `reach-browserd` responds to CDP.
    pub async fn connect(browserd_url: impl Into<String>) -> Result<Self> {
        let client = Self::new(browserd_url);
        client
            .send::<_, Value>(RawCdpCommand::new("Browser.getVersion"))
            .await?;
        Ok(client)
    }

    /// Send a typed CDP command through `reach-browserd`.
    pub async fn send<C, R>(&self, command: C) -> Result<CdpResponse<R>>
    where
        C: CdpCommand,
        R: DeserializeOwned,
    {
        let method = command.method();
        debug!(method, browserd_url = %self.browserd_url, "sending CDP command");

        let request = CdpRequest {
            method: method.to_string(),
            params: serde_json::to_value(command.params())
                .context("failed to serialize CDP command params")?,
        };

        let response = self
            .http
            .post(format!("{}/cdp", self.browserd_url))
            .json(&request)
            .send()
            .await
            .context("failed to send CDP command to reach-browserd")?
            .error_for_status()
            .context("reach-browserd returned an HTTP error")?
            .json::<CdpResponse<R>>()
            .await
            .context("failed to decode CDP response from reach-browserd")?;

        trace!(
            method,
            has_result = response.result.is_some(),
            has_error = response.error.is_some(),
            "received CDP response"
        );

        Ok(response)
    }
}

#[derive(Debug, Clone, Serialize)]
struct CdpRequest {
    method: String,
    params: Value,
}

/// Top-level response returned by a CDP command.
#[derive(Debug, Clone, Deserialize)]
pub struct CdpResponse<T = Value> {
    /// CDP command identifier, when supplied by the bridge.
    pub id: Option<u64>,
    /// Successful command result payload.
    pub result: Option<T>,
    /// CDP error payload, when the browser rejected the command.
    pub error: Option<CdpError>,
}

impl<T> CdpResponse<T> {
    /// Convert the CDP envelope into a standard `Result`.
    pub fn into_result(self) -> std::result::Result<T, CdpError> {
        match (self.result, self.error) {
            (Some(result), _) => Ok(result),
            (_, Some(error)) => Err(error),
            (None, None) => Err(CdpError {
                code: None,
                message: "CDP response did not include result or error".to_string(),
                data: None,
            }),
        }
    }
}

/// Error object returned by a failed CDP command.
#[derive(Debug, Clone, Deserialize)]
pub struct CdpError {
    /// Protocol error code, when Chromium supplied one.
    pub code: Option<i64>,
    /// Human-readable CDP error message.
    pub message: String,
    /// Optional structured diagnostic data.
    pub data: Option<Value>,
}

/// Trait implemented by typed CDP commands.
pub trait CdpCommand {
    /// Serializable parameter object for the command.
    type Params: Serialize;

    /// CDP method name, such as `Page.navigate`.
    fn method(&self) -> &'static str;
    /// Parameters sent with the command.
    fn params(&self) -> &Self::Params;
}

/// Untyped CDP command for methods without a typed wrapper.
#[derive(Debug, Clone)]
pub struct RawCdpCommand<P = Value> {
    method: &'static str,
    params: P,
}

impl RawCdpCommand<Value> {
    /// Create a raw command with an empty parameter object.
    pub fn new(method: &'static str) -> Self {
        Self {
            method,
            params: Value::Object(Default::default()),
        }
    }
}

impl<P> RawCdpCommand<P>
where
    P: Serialize,
{
    /// Create a raw command with caller-provided parameters.
    pub fn with_params(method: &'static str, params: P) -> Self {
        Self { method, params }
    }
}

impl<P> CdpCommand for RawCdpCommand<P>
where
    P: Serialize,
{
    type Params = P;

    fn method(&self) -> &'static str {
        self.method
    }

    fn params(&self) -> &Self::Params {
        &self.params
    }
}

fn trim_trailing_slash(mut url: String) -> String {
    while url.ends_with('/') {
        url.pop();
    }
    url
}
