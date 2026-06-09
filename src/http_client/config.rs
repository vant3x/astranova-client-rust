use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_REDIRECTS: u32 = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestConfig {
    pub timeout: Duration,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub redirect_policy: RedirectPolicy,
    pub retry: RetryConfig,
    pub proxy_url: Option<String>,
    pub verify_ssl: bool,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            follow_redirects: true,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            redirect_policy: RedirectPolicy::Follow,
            retry: RetryConfig::default(),
            proxy_url: None,
            verify_ssl: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub backoff_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 0,
            backoff_ms: 1000,
        }
    }
}

impl fmt::Display for RetryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} retries, {}ms backoff",
            self.max_retries, self.backoff_ms
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RedirectPolicy {
    #[default]
    Follow,
    NoFollow,
    Limited(u32),
}

impl RedirectPolicy {
    #[allow(dead_code)]
    pub const ALL: [RedirectPolicy; 3] = [
        RedirectPolicy::Follow,
        RedirectPolicy::NoFollow,
        RedirectPolicy::Limited(DEFAULT_MAX_REDIRECTS),
    ];
}

#[allow(dead_code)]
pub struct ProxyConfig {
    pub url: String,
    pub auth: Option<(String, String)>,
}

#[allow(dead_code)]
pub struct TlsConfig {
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
    pub verify_ssl: bool,
}

#[allow(dead_code)]
impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            verify_ssl: true,
        }
    }
}

impl fmt::Display for RedirectPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RedirectPolicy::Follow => write!(f, "Follow"),
            RedirectPolicy::NoFollow => write!(f, "No Follow"),
            RedirectPolicy::Limited(n) => write!(f, "Limited ({})", n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_retry_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 0);
        assert_eq!(config.backoff_ms, 1000);
    }

    #[test]
    fn retry_config_display() {
        let config = RetryConfig {
            max_retries: 3,
            backoff_ms: 500,
        };
        assert_eq!(config.to_string(), "3 retries, 500ms backoff");
    }

    #[test]
    fn default_request_config() {
        let config = RequestConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.redirect_policy, RedirectPolicy::Follow);
        assert_eq!(config.retry.max_retries, 0);
    }

    #[test]
    fn redirect_policy_display() {
        assert_eq!(RedirectPolicy::Follow.to_string(), "Follow");
        assert_eq!(RedirectPolicy::NoFollow.to_string(), "No Follow");
        assert_eq!(RedirectPolicy::Limited(5).to_string(), "Limited (5)");
    }

    #[test]
    fn redirect_policy_all_has_3_variants() {
        assert_eq!(RedirectPolicy::ALL.len(), 3);
    }

    #[test]
    fn redirect_policy_default_is_follow() {
        assert_eq!(RedirectPolicy::default(), RedirectPolicy::Follow);
    }

    #[test]
    fn default_tls_config() {
        let config = TlsConfig::default();
        assert!(config.ca_cert_path.is_none());
        assert!(config.client_cert_path.is_none());
        assert!(config.client_key_path.is_none());
        assert!(config.verify_ssl);
    }

    #[test]
    fn proxy_config_stores_credentials() {
        let config = ProxyConfig {
            url: "http://proxy:8080".to_string(),
            auth: Some(("user".to_string(), "pass".to_string())),
        };
        assert_eq!(config.url, "http://proxy:8080");
        assert!(config.auth.is_some());
    }
}
