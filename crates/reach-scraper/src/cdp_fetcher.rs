//! CDP-backed scraper implementation.

use crate::{ProxyConfig, ReachScraper, ScrapeMetadata, ScrapeOutput};
use anyhow::{Context, Result, anyhow, bail};
use reach_cdp::{
    CdpClient, CdpCommand,
    commands::{
        Cookie, NetworkEnable, NetworkEnableResult, NetworkGetCookies, NetworkGetCookiesResult,
        PageNavigate, PageNavigateResult, RuntimeEvaluate, RuntimeEvaluateResult,
    },
};
use serde::de::DeserializeOwned;
use tracing::{debug, trace};

const DEFAULT_LOAD_TIMEOUT_MS: u64 = 15_000;
const DEFAULT_NETWORK_IDLE_MS: u64 = 500;

/// Fetches pages by navigating a live browser through CDP.
#[derive(Debug, Clone, Copy)]
pub struct CdpFetcher<'a> {
    cdp: &'a CdpClient,
    load_timeout_ms: u64,
    network_idle_ms: u64,
}

impl<'a> CdpFetcher<'a> {
    /// Create a fetcher for a CDP client.
    pub fn new(cdp: &'a CdpClient) -> Self {
        Self {
            cdp,
            load_timeout_ms: DEFAULT_LOAD_TIMEOUT_MS,
            network_idle_ms: DEFAULT_NETWORK_IDLE_MS,
        }
    }

    /// Create a fetcher from a high-level scraper.
    pub fn from_scraper(scraper: &'a ReachScraper) -> Self {
        Self::new(scraper.cdp())
    }

    /// Set the maximum time to wait for document load.
    pub fn with_load_timeout_ms(mut self, load_timeout_ms: u64) -> Self {
        self.load_timeout_ms = load_timeout_ms;
        self
    }

    /// Set the quiet window used as a network-idle heuristic.
    pub fn with_network_idle_ms(mut self, network_idle_ms: u64) -> Self {
        self.network_idle_ms = network_idle_ms;
        self
    }

    /// Fetch a URL through the browser and return the rendered HTML.
    pub async fn fetch(
        &self,
        url: impl Into<String>,
        proxy: Option<ProxyConfig>,
    ) -> Result<ScrapeOutput> {
        let url = url.into();
        debug!(
            url = %url,
            load_timeout_ms = self.load_timeout_ms,
            network_idle_ms = self.network_idle_ms,
            uses_proxy = proxy.is_some(),
            "starting CDP scrape"
        );

        if let Some(proxy) = proxy.as_ref() {
            self.configure_proxy(proxy).await?;
        }

        let _: NetworkEnableResult = self
            .send(NetworkEnable::new())
            .await
            .context("failed to enable CDP network domain")?;

        let navigation: PageNavigateResult = self
            .send(PageNavigate::new(url.clone()))
            .await
            .context("failed to navigate via CDP")?;

        if let Some(error_text) = navigation.error_text {
            bail!("CDP navigation failed for {url}: {error_text}");
        }

        self.wait_for_load_and_network_idle()
            .await
            .context("failed while waiting for page load")?;

        let html = self
            .evaluate_string("document.documentElement.outerHTML")
            .await
            .context("failed to extract page HTML via CDP")?;

        let final_url = self.evaluate_string("window.location.href").await.ok();
        debug!(url = %url, final_url = ?final_url, "completed CDP scrape");

        Ok(ScrapeOutput {
            url,
            content: Some(html),
            metadata: ScrapeMetadata {
                final_url,
                status_code: None,
                proxy,
            },
        })
    }

    /// Return cookies from the current browser context.
    pub async fn get_cookies(&self, urls: Option<Vec<String>>) -> Result<Vec<Cookie>> {
        let command = match urls {
            Some(urls) => NetworkGetCookies::for_urls(urls),
            None => NetworkGetCookies::new(),
        };

        let result: NetworkGetCookiesResult = self
            .send(command)
            .await
            .context("failed to get cookies via CDP")?;

        Ok(result.cookies)
    }

    async fn configure_proxy(&self, proxy: &ProxyConfig) -> Result<()> {
        debug!(proxy_url = %proxy.url, "per-request CDP proxy configuration is not active yet");
        // TODO: CDP proxy configuration is target/browser-context specific and
        // reach-browserd currently attaches to an existing target. Add browserd
        // support for creating a context with proxyServer/proxyBypassList before
        // using this fetcher for per-request proxy routing.
        Ok(())
    }

    async fn wait_for_load_and_network_idle(&self) -> Result<()> {
        let expression = format!(
            r#"
new Promise((resolve) => {{
  const quietWindowMs = {network_idle_ms};
  const timeoutMs = {load_timeout_ms};
  const startedAt = Date.now();
  let lastResourceCount = performance.getEntriesByType("resource").length;
  let quietSince = Date.now();

  const tick = () => {{
    const resourceCount = performance.getEntriesByType("resource").length;
    if (resourceCount !== lastResourceCount) {{
      lastResourceCount = resourceCount;
      quietSince = Date.now();
    }}

    if (document.readyState !== "complete") {{
      quietSince = Date.now();
    }}

    if (document.readyState === "complete" && Date.now() - quietSince >= quietWindowMs) {{
      resolve(true);
      return;
    }}

    if (Date.now() - startedAt >= timeoutMs) {{
      resolve(false);
      return;
    }}

    setTimeout(tick, 100);
  }};

  tick();
}})
"#,
            network_idle_ms = self.network_idle_ms,
            load_timeout_ms = self.load_timeout_ms,
        );

        let loaded = self
            .evaluate_json(
                RuntimeEvaluate::new(expression)
                    .with_await_promise(true)
                    .with_return_by_value(true)
                    .with_timeout_ms(self.load_timeout_ms + 1_000),
            )
            .await?
            .as_bool()
            .unwrap_or(false);

        if !loaded {
            bail!(
                "timed out after {}ms waiting for document load and network idle",
                self.load_timeout_ms
            );
        }

        trace!("document reached load and network-idle heuristic");
        Ok(())
    }

    async fn evaluate_string(&self, expression: &str) -> Result<String> {
        let value = self
            .evaluate_json(
                RuntimeEvaluate::new(expression)
                    .with_return_by_value(true)
                    .with_timeout_ms(self.load_timeout_ms),
            )
            .await?;

        value
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow!("Runtime.evaluate did not return a string"))
    }

    async fn evaluate_json(&self, command: RuntimeEvaluate) -> Result<serde_json::Value> {
        let result: RuntimeEvaluateResult = self.send(command).await?;

        if let Some(exception) = result.exception_details {
            bail!("Runtime.evaluate exception: {}", exception.text);
        }

        result
            .result
            .value
            .ok_or_else(|| anyhow!("Runtime.evaluate did not return a by-value result"))
    }

    async fn send<C, R>(&self, command: C) -> Result<R>
    where
        C: CdpCommand,
        R: DeserializeOwned,
    {
        let method = command.method();
        self.cdp
            .send::<_, R>(command)
            .await?
            .into_result()
            .map_err(|error| anyhow!("CDP command {method} failed: {}", error.message))
    }
}
