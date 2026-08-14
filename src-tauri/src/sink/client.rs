use crate::constraints::SINK_REQUEST_TIMEOUT_SECS;
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use url::Url;

#[derive(Clone)]
pub struct SinkClient {
    client: Client,
    url: Url,
    auth: Option<String>,
}

impl SinkClient {
    /// The production constructor. Every request it makes is bounded.
    pub fn new(url: Url, auth: Option<String>) -> Result<Self> {
        Self::build(url, auth, Some(Duration::from_secs(SINK_REQUEST_TIMEOUT_SECS)))
    }

    /// The same client with the bound chosen by the caller, for tests.
    ///
    /// A test proves the bound with a short one instead of waiting out the
    /// real `SINK_REQUEST_TIMEOUT_SECS`. `None` builds the client reqwest
    /// gives by default, which is the one with no bound at all; a test that
    /// drives a real socket on a paused clock needs it, because the paused
    /// clock jumps to any deadline that exists the moment the runtime idles.
    /// Production has no way to reach either, which is why this is `cfg(test)`.
    #[cfg(test)]
    pub fn with_timeout(url: Url, auth: Option<String>, timeout: Option<Duration>) -> Result<Self> {
        Self::build(url, auth, timeout)
    }

    fn build(url: Url, auth: Option<String>, timeout: Option<Duration>) -> Result<Self> {
        // The bound covers the whole exchange, connection and reply together.
        // reqwest sets no timeout, no read timeout, and no connect timeout by
        // default, so without one a sink that goes quiet is waited on for as
        // long as it stays quiet.
        let mut builder = Client::builder();
        if let Some(t) = timeout {
            builder = builder.timeout(t);
        }

        let client = builder.build().context("build reqwest client")?;
        Ok(Self { client, url, auth })
    }

    pub async fn post_json<T: Serialize>(&self, body: &T) -> Result<()> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("tarcie-v1"));

        if let Some(a) = &self.auth {
            let v = if a.starts_with("Bearer ") || a.starts_with("ApiKey ") {
                a.clone()
            } else {
                format!("Bearer {}", a)
            };
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&v).context("auth header")?);
        }

        let resp = self
            .client
            .post(self.url.clone())
            .headers(headers)
            .json(body)
            .send()
            .await
            .context("POST to sink")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("sink error {}: {}", status, text);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_sink::{spawn_silent_sink, spawn_sink, OK};
    use std::time::Instant;

    /// A short bound, so the proof costs a fraction of a second rather than
    /// the `SINK_REQUEST_TIMEOUT_SECS` a real run allows.
    const SHORT: Duration = Duration::from_millis(250);

    /// Long enough that no healthy loopback exchange comes near it.
    const GENEROUS: Duration = Duration::from_secs(5);

    fn client_for(url: &str, timeout: Option<Duration>) -> SinkClient {
        SinkClient::with_timeout(Url::parse(url).expect("parse url"), None, timeout)
            .expect("build the sink client")
    }

    // These run on a real clock. The bound is the thing under test, so a
    // paused clock — which fires every deadline the moment the runtime idles —
    // would report a timeout whatever the sink did.

    #[tokio::test]
    async fn a_sink_that_never_answers_is_given_up_on_at_the_bound() {
        let (url, _sink) = spawn_silent_sink();
        let client = client_for(&url, Some(SHORT));

        let started = Instant::now();
        let err = match client.post_json(&"{}").await {
            Ok(()) => panic!("a sink that never answers cannot have accepted the batch"),
            Err(e) => e,
        };
        let waited = started.elapsed();

        assert!(
            format!("{err:#}").to_lowercase().contains("timed out"),
            "the failure names the bound it hit: {err:#}"
        );
        assert!(
            waited < GENEROUS,
            "the wait ended at the bound rather than running on: {waited:?}"
        );
    }

    #[tokio::test]
    async fn a_sink_that_answers_inside_the_bound_is_not_cut_off() {
        // The counterweight. Without it the test above would pass against a
        // client that refused every request, timeout or no timeout.
        let (url, server) = spawn_sink(OK);
        let client = client_for(&url, Some(GENEROUS));

        client.post_json(&"{}").await.expect("a healthy sink is not cut off");

        let _ = server.join();
    }
}
