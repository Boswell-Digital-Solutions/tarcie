use crate::constraints::*;
use anyhow::{Context, Result};
use std::env;
use url::Url;

#[derive(Clone)]
pub struct SinkConfig {
    pub url: Url,
    pub auth: Option<String>,
    pub allow_remote: bool,
    pub flush_interval_secs: u64,
    pub batch_max: usize,
    pub queue_max_events: usize,
}

pub const DEFAULT_SINK_URL: &str = "http://127.0.0.1:8080/ingest/tarcie";

impl SinkConfig {
    pub fn from_env() -> Result<Self> {
        Self::resolve(|key| env::var(key).ok())
    }

    /// Resolve configuration from an arbitrary variable lookup.
    ///
    /// `from_env` supplies the process environment. Taking the lookup as an
    /// argument keeps resolution pure, so a caller can exercise every branch
    /// without mutating process-wide state.
    pub fn resolve<F>(get: F) -> Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let url_raw = get("TARCIE_SINK_URL").unwrap_or_else(|| DEFAULT_SINK_URL.to_string());

        let url = Url::parse(&url_raw).context("parse TARCIE_SINK_URL")?;

        let allow_remote = get("TARCIE_ALLOW_REMOTE_SINK")
            .map(|v| v == "true" || v == "1" || v.eq_ignore_ascii_case("yes"))
            .unwrap_or(false);

        if !allow_remote && !is_localhost(&url) {
            anyhow::bail!(
                "remote sink disallowed. Set TARCIE_ALLOW_REMOTE_SINK=true to allow: {}",
                url
            );
        }

        let auth = get("TARCIE_SINK_AUTH");

        let flush_interval_secs = get("TARCIE_FLUSH_INTERVAL_SECS")
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_FLUSH_INTERVAL_SECS);

        let batch_max = get("TARCIE_BATCH_MAX")
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_BATCH_MAX);

        let queue_max_events = get("TARCIE_QUEUE_MAX_EVENTS")
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_QUEUE_MAX_EVENTS);

        Ok(Self {
            url,
            auth,
            allow_remote,
            flush_interval_secs,
            batch_max: batch_max.max(1),
            queue_max_events: queue_max_events.max(100),
        })
    }
}

fn is_localhost(url: &Url) -> bool {
    match url.host_str() {
        Some("127.0.0.1") | Some("localhost") => true,
        Some(host) if host.starts_with("127.") => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const REMOTE: &str = "https://example.com/ingest";

    /// A lookup over a fixed set of variables, standing in for the process
    /// environment so tests stay independent of each other.
    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    fn unset(_: &str) -> Option<String> {
        None
    }

    // --- 6. Sink URL validation -------------------------------------------

    #[test]
    fn the_default_sink_is_loopback_and_is_permitted() {
        let cfg = SinkConfig::resolve(unset).expect("defaults resolve");
        assert_eq!(cfg.url.as_str(), DEFAULT_SINK_URL);
        assert!(!cfg.allow_remote);
    }

    #[test]
    fn a_remote_sink_is_refused_unless_it_is_opted_into() {
        // Matched rather than unwrapped: SinkConfig deliberately does not
        // derive Debug, because it holds the sink auth token.
        let err = match SinkConfig::resolve(vars(&[("TARCIE_SINK_URL", REMOTE)])) {
            Ok(_) => panic!("a remote sink must be refused by default"),
            Err(e) => e,
        };
        assert!(
            err.to_string().contains("remote sink disallowed"),
            "the refusal names its reason: {err}"
        );
    }

    #[test]
    fn a_remote_sink_is_permitted_with_an_explicit_opt_in() {
        for flag in ["true", "1", "yes", "YES", "Yes"] {
            SinkConfig::resolve(vars(&[
                ("TARCIE_SINK_URL", REMOTE),
                ("TARCIE_ALLOW_REMOTE_SINK", flag),
            ]))
            .unwrap_or_else(|e| panic!("{flag:?} should opt in: {e}"));
        }
    }

    #[test]
    fn unrecognised_opt_in_values_leave_the_remote_sink_closed() {
        // Note the asymmetry: "yes" is matched case-insensitively but "true"
        // is not, so "TRUE" does not open a remote sink.
        for flag in ["false", "0", "no", "TRUE", "True", ""] {
            assert!(
                SinkConfig::resolve(vars(&[
                    ("TARCIE_SINK_URL", REMOTE),
                    ("TARCIE_ALLOW_REMOTE_SINK", flag),
                ]))
                .is_err(),
                "{flag:?} must not open a remote sink"
            );
        }
    }

    #[test]
    fn loopback_hosts_are_treated_as_local() {
        for url in [
            "http://127.0.0.1:9/x",
            "http://localhost:9/x",
            "http://127.5.5.5:9/x",
        ] {
            SinkConfig::resolve(vars(&[("TARCIE_SINK_URL", url)]))
                .unwrap_or_else(|e| panic!("{url} should count as local: {e}"));
        }
    }

    #[test]
    fn a_hostname_beginning_with_127_passes_as_loopback() {
        // KNOWN DEVIATION: the loopback test is a string prefix check, so a
        // registrable domain that merely starts with "127." is accepted as
        // local and reaches the network without an opt-in.
        assert!(
            SinkConfig::resolve(vars(&[("TARCIE_SINK_URL", "http://127.example.com/ingest")]))
                .is_ok()
        );
    }

    #[test]
    fn the_ipv6_loopback_address_is_not_recognised_as_local() {
        // KNOWN DEVIATION: [::1] is loopback, but it does not match the host
        // check, so it is refused as though it were remote.
        assert!(
            SinkConfig::resolve(vars(&[("TARCIE_SINK_URL", "http://[::1]:8080/ingest")])).is_err()
        );
    }

    #[test]
    fn a_url_that_cannot_be_parsed_is_an_error() {
        assert!(SinkConfig::resolve(vars(&[("TARCIE_SINK_URL", "not a url")])).is_err());
    }

    // --- Configuration bounds ---------------------------------------------

    #[test]
    fn numeric_overrides_are_applied() {
        let cfg = SinkConfig::resolve(vars(&[
            ("TARCIE_FLUSH_INTERVAL_SECS", "30"),
            ("TARCIE_BATCH_MAX", "7"),
            ("TARCIE_QUEUE_MAX_EVENTS", "500"),
        ]))
        .expect("overrides resolve");

        assert_eq!(cfg.flush_interval_secs, 30);
        assert_eq!(cfg.batch_max, 7);
        assert_eq!(cfg.queue_max_events, 500);
    }

    #[test]
    fn an_unparsable_numeric_override_falls_back_to_its_default() {
        let cfg = SinkConfig::resolve(vars(&[("TARCIE_BATCH_MAX", "not-a-number")]))
            .expect("config resolves");
        assert_eq!(cfg.batch_max, DEFAULT_BATCH_MAX);
    }

    #[test]
    fn batch_and_queue_sizes_are_floored_at_workable_values() {
        let cfg = SinkConfig::resolve(vars(&[
            ("TARCIE_BATCH_MAX", "0"),
            ("TARCIE_QUEUE_MAX_EVENTS", "1"),
        ]))
        .expect("config resolves");

        assert_eq!(cfg.batch_max, 1, "a zero batch would never drain the queue");
        assert_eq!(cfg.queue_max_events, 100);
    }

    #[test]
    fn auth_is_carried_through_when_set() {
        let cfg = SinkConfig::resolve(vars(&[("TARCIE_SINK_AUTH", "s3cret")]))
            .expect("config resolves");
        assert_eq!(cfg.auth.as_deref(), Some("s3cret"));
    }

    #[test]
    fn auth_is_absent_when_unset() {
        assert!(SinkConfig::resolve(unset).expect("config resolves").auth.is_none());
    }
}
