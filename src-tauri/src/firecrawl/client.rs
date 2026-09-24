// Atlas OS — Firecrawl client factory (RFC 28 Section E).
//
// `firecrawl::Client` is constructed from (api_url, api_key?) — both
// strings — and is internally a `reqwest::Client` clone. We expose a
// thin wrapper that resolves those two pieces from the environment
// (env vars first, future pluggable `profiles::Profile::secret`
// resolution), so the facade and CLI don't re-read env state and so
// tests can construct the client from explicit values without env
// pollution.
//
// Both env vars obey conventional env-var shadowing:
//
//   * `FIRECRAWL_API_KEY`     — canonically the Firecrawl cloud key
//                                (matches the SDK's own env-var name
//                                from the quickstart docs).
//   * `ATLAS_FIRECRAWL_URL` — self-hosted base URL override. When
//                                set, the SDK is pointed at the
//                                self-hosted instance; when unset,
//                                `https://api.firecrawl.dev` is used.
//     `ATLAS_FIRECRAWL_URL` is honoured as a legacy fallback for
//     existing users.
//
// A `FirecrawlKey::None` (i.e. neither env var is set) is valid: the
// SDK supports a keyless free tier on the cloud endpoint. The facade
// will still surface `FirecrawlFacadeError::Api` if a given operation
// returns 401/402/403 because the keyless tier doesn't cover it.

use std::env;

use super::error::FirecrawlFacadeError;

/// Which Firecrawl deployment the facade is talking to. Encoded as a
/// field on [`FirecrawlClient`] so consumers can introspect without
/// re-parsing env vars.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirecrawlEndpoint {
    /// The Firecrawl-managed cloud at `https://api.firecrawl.dev`.
    /// Keyless free tier covers `scrape`/`search`/`interact`.
    Cloud,
    /// A self-hosted Firecrawl instance (any URL). Self-hosted may or
    /// may not require a bearer key; that is config-dependent.
    SelfHosted,
}

/// Resolved API key. Stored as `Option<String>` would lose the
/// distinction between "self-hosted without auth" (valid) and "cloud
/// without key" (valid for keyless free tier) — the enum keeps the
/// distinction around for diagnostics and future HUD surfacing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FirecrawlKey {
    /// Explicit bearer key. Recommended for non-keyless operations
    /// (`crawl`, `extract`, rate-limit sidegrade).
    Bearer(String),
    /// No key supplied. Cloud keyless free tier / self-hosted without
    /// auth. The SDK accepts both shapes (`Client::new_selfhosted`).
    None,
}

/// Resolved Firecrawl client + endpoint metadata. Built from
/// [`from_env`] or constructed directly in tests via
/// [`from_explicit`]. Cheap to clone (the inner SDK `Client` is itself
/// `Clone`).
#[derive(Clone, Debug)]
pub struct FirecrawlClient {
    pub endpoint: FirecrawlEndpoint,
    pub key: FirecrawlKey,
    pub(crate) inner: firecrawl::Client,
}

impl FirecrawlClient {
    /// Resolve a client from the process environment.
    ///
    /// Resolution order:
    ///
    /// 1. If `ATLAS_FIRECRAWL_URL` (or legacy `ATLAS_FIRECRAWL_URL`) is
    ///    set → self-hosted endpoint. `FIRECRAWL_API_KEY`, if set, is
    ///    forwarded as the bearer key.
    /// 2. Otherwise → cloud endpoint at the SDK default. Bearer key is
    ///    `FIRECRAWL_API_KEY` if set, else `None` (keyless free tier).
    ///
    /// Empty/whitespace-only values are treated as `None` to avoid a
    /// blank `Bearer` header that the SDK would then send.
    #[allow(clippy::result_large_err)]
    pub fn from_env() -> Result<Self, FirecrawlFacadeError> {
        let raw_url_env =
            env::var_os("ATLAS_FIRECRAWL_URL").or_else(|| env::var_os("ATLAS_FIRECRAWL_URL"));
        let raw_key_env = env::var_os("FIRECRAWL_API_KEY");
        let api_key: Option<String> = raw_key_env
            .and_then(|s| s.to_str().map(str::trim).map(String::from))
            .filter(|s| !s.is_empty());
        let url_env: Option<String> = raw_url_env
            .and_then(|s| s.to_str().map(str::trim).map(String::from))
            .filter(|s| !s.is_empty());

        Self::from_explicit(url_env.as_deref(), api_key.as_deref())
    }

    /// Test/build a client from explicit pieces. `api_url=None` means
    /// use the SDK cloud endpoint; `api_url=Some(..)` enables the
    /// self-hosted code path. `api_key=None` means keyless tier (valid
    /// for cloud scrape/search and for self-hosted instances without
    /// auth).
    ///
    /// The SDK's `Client::new` accepts empty key strings (keyless
    /// free tier for cloud `scrape`/`search`); we mirror that
    /// acceptance for empty/whitespace caller inputs by falling back
    /// to the keyless `Client::new_selfhosted(..., None::<&str>)`
    /// shape — either way the SDK omits the `Authorization` header.
    #[allow(clippy::result_large_err)]
    pub fn from_explicit(
        api_url: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<Self, FirecrawlFacadeError> {
        let url_trim = api_url.map(str::trim).filter(|s| !s.is_empty());
        let key_trim = api_key.map(str::trim).filter(|s| !s.is_empty());
        let key = match key_trim {
            Some(k) => FirecrawlKey::Bearer(k.to_string()),
            None => FirecrawlKey::None,
        };
        let (endpoint, inner) = match url_trim {
            Some(url) => {
                let client = firecrawl::Client::new_selfhosted(url, key_trim)
                    .map_err(|e| super::error::lift_sdk_error("Client::new_selfhosted", e))?;
                (FirecrawlEndpoint::SelfHosted, client)
            }
            None => {
                let client =
                    firecrawl::Client::new_selfhosted("https://api.firecrawl.dev", key_trim)
                        .map_err(|e| super::error::lift_sdk_error("Client::new_selfhosted", e))?;
                (FirecrawlEndpoint::Cloud, client)
            }
        };
        Ok(Self {
            endpoint,
            key,
            inner,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Some env tests would corrupt the process env if other tests ran
    // concurrently — gate them through a process-wide lock so they
    // run one-at-a-time against the live process env vars.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_clean_env<F: FnOnce()>(f: F) {
        let _g = ENV_LOCK.lock().unwrap();
        let atlas_b = env::var_os("ATLAS_FIRECRAWL_URL");
        let opencode_b = env::var_os("ATLAS_FIRECRAWL_URL");
        let key_b = env::var_os("FIRECRAWL_API_KEY");
        env::remove_var("ATLAS_FIRECRAWL_URL");
        env::remove_var("ATLAS_FIRECRAWL_URL");
        env::remove_var("FIRECRAWL_API_KEY");
        f();
        if let Some(v) = atlas_b {
            env::set_var("ATLAS_FIRECRAWL_URL", v);
        }
        if let Some(v) = opencode_b {
            env::set_var("ATLAS_FIRECRAWL_URL", v);
        }
        if let Some(v) = key_b {
            env::set_var("FIRECRAWL_API_KEY", v);
        }
    }

    #[test]
    fn cloud_keyless_when_no_env_is_set() {
        with_clean_env(|| {
            let c = FirecrawlClient::from_env().expect("env ok");
            assert_eq!(c.endpoint, FirecrawlEndpoint::Cloud);
            assert_eq!(c.key, FirecrawlKey::None);
        });
    }

    #[test]
    fn cloud_bearer_when_only_key_is_set() {
        with_clean_env(|| {
            env::set_var("FIRECRAWL_API_KEY", "fc-test-key");
            let c = FirecrawlClient::from_env().expect("env ok");
            assert_eq!(c.endpoint, FirecrawlEndpoint::Cloud);
            match c.key {
                FirecrawlKey::Bearer(ref k) => assert_eq!(k, "fc-test-key"),
                FirecrawlKey::None => panic!("expected Bearer"),
            }
        });
    }

    #[test]
    #[cfg_attr(
        not(feature = "firecrawl"),
        ignore = "ignored from manifest but always runs"
    )]
    fn self_hosted_when_url_is_set() {
        with_clean_env(|| {
            env::set_var("ATLAS_FIRECRAWL_URL", "http://localhost:3000");
            let c = FirecrawlClient::from_env().expect("env ok");
            assert_eq!(c.endpoint, FirecrawlEndpoint::SelfHosted);
        });
    }

    #[test]
    fn explicit_none_api_key_yields_keyless_cloud() {
        let c = FirecrawlClient::from_explicit(None, None).expect("ok");
        assert_eq!(c.endpoint, FirecrawlEndpoint::Cloud);
        assert_eq!(c.key, FirecrawlKey::None);
    }

    #[test]
    fn explicit_self_hosted_with_bearer() {
        let c =
            FirecrawlClient::from_explicit(Some("http://localhost:3000"), Some("abc")).expect("ok");
        assert_eq!(c.endpoint, FirecrawlEndpoint::SelfHosted);
        match c.key {
            FirecrawlKey::Bearer(ref k) => assert_eq!(k, "abc"),
            FirecrawlKey::None => panic!("expected Bearer"),
        }
    }

    #[test]
    fn explicit_self_hosted_without_auth() {
        let c = FirecrawlClient::from_explicit(Some("http://localhost:3000"), None).expect("ok");
        assert_eq!(c.endpoint, FirecrawlEndpoint::SelfHosted);
        assert_eq!(c.key, FirecrawlKey::None);
    }

    #[test]
    fn blank_key_string_is_treated_as_none() {
        let c = FirecrawlClient::from_explicit(None, Some("   ")).expect("ok");
        assert_eq!(c.key, FirecrawlKey::None);
    }

    #[test]
    fn blank_url_falls_back_to_cloud() {
        let c = FirecrawlClient::from_explicit(Some("   "), None).expect("ok");
        assert_eq!(c.endpoint, FirecrawlEndpoint::Cloud);
    }
}
