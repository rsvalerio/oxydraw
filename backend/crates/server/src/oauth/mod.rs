//! OAuth/OIDC sign-in for the scene library.
//!
//! Providers come in two buckets, both normalizing to [`RemoteUser`]:
//! - **OIDC** ([`google`]): issuer discovery + verified ID token. Adding another OIDC
//!   IdP (Microsoft, GitLab, …) is configuration, not code.
//! - **Plain OAuth2** ([`github`]): authorization-code flow + a provider-specific
//!   userinfo fetch. Adding one (Facebook, Discord, …) is a small module like
//!   [`github`].
//!
//! All flows use PKCE (S256) and a server-side pending-flow record keyed by the CSRF
//! `state`, with a short TTL — nothing flow-related is trusted from the client.

pub mod github;
pub mod google;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oxydraw_core::config::Config;
use oxydraw_core::sync::lock_unpoisoned;

/// How long an in-progress login may take between redirect-out and callback. Also the
/// lifetime of the browser-binding state cookie set by `auth_start`.
pub(crate) const FLOW_TTL: Duration = Duration::from_secs(10 * 60);

/// What a provider told us about the signed-in user, normalized across providers.
#[derive(Debug, Clone)]
pub struct RemoteUser {
    pub provider: &'static str,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Which step of a sign-in flow a provider error came from. Kept on [`AuthError::Provider`]
/// so logs and callers can tell transient network/exchange trouble from a verification
/// failure (bad signature, nonce mismatch — a possible attack signal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Client construction: URL parsing and (for OIDC) issuer discovery.
    Setup,
    /// Authorization-code → token exchange and userinfo fetches.
    Exchange,
    /// ID-token signature/issuer/audience/nonce verification.
    Verification,
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Stage::Setup => "setup",
            Stage::Exchange => "exchange",
            Stage::Verification => "verification",
        })
    }
}

/// Errors from the sign-in flow. The full source chain is logged server-side; clients
/// only see a generic failure.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("unknown provider")]
    UnknownProvider,
    #[error("login expired or state mismatch")]
    InvalidState,
    #[error("provider error during {stage}")]
    Provider {
        stage: Stage,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl AuthError {
    pub fn setup(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Provider {
            stage: Stage::Setup,
            source: e.into(),
        }
    }

    pub fn exchange(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Provider {
            stage: Stage::Exchange,
            source: e.into(),
        }
    }

    pub fn verification(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Provider {
            stage: Stage::Verification,
            source: e.into(),
        }
    }
}

/// The output of starting a sign-in flow: where to redirect the browser, the CSRF
/// `state` that ties the eventual callback back to this flow, and the server-side
/// pending record. A named struct (not a `(String, String, Pending)` tuple) so the two
/// adjacent `String`s — `authorize_url` and `state` — cannot be silently swapped at a
/// call or impl site (FN-4).
pub struct BeginFlow {
    pub authorize_url: String,
    pub state: String,
    pub pending: Pending,
}

/// A configured identity provider.
pub enum Provider {
    // Boxed: the OIDC provider embeds its cached discovery metadata (~1.7 kB).
    Oidc(Box<google::OidcProvider>),
    Github(github::GithubProvider),
}

impl Provider {
    /// Build the authorization redirect and the server-side flow record.
    pub async fn begin(
        &self,
        http: &reqwest::Client,
        redirect_uri: &str,
    ) -> Result<BeginFlow, AuthError> {
        match self {
            Provider::Oidc(p) => p.begin(http, redirect_uri).await,
            Provider::Github(p) => p.begin(redirect_uri),
        }
    }

    /// Exchange the callback `code` for a verified [`RemoteUser`].
    pub async fn exchange(
        &self,
        http: &reqwest::Client,
        redirect_uri: &str,
        code: &str,
        pending: Pending,
    ) -> Result<RemoteUser, AuthError> {
        match self {
            Provider::Oidc(p) => p.exchange(http, redirect_uri, code, pending).await,
            Provider::Github(p) => p.exchange(http, redirect_uri, code, pending).await,
        }
    }
}

/// The configured providers, plus the shared HTTP client used for all provider calls.
pub struct Registry {
    providers: HashMap<&'static str, Provider>,
    pub http: reqwest::Client,
}

impl Registry {
    /// Build the registry from configuration. A provider is enabled iff both its client
    /// id and secret are set.
    pub fn from_config(cfg: &Config) -> anyhow::Result<Self> {
        // Redirects stay disabled: every provider URL is an exact endpoint, and a
        // redirect from a token/userinfo endpoint would be an SSRF vector.
        // Timeouts: discovery/token/userinfo are external calls on the login path; a
        // stalled IdP must fail the login within a bound instead of pinning request
        // tasks indefinitely (the token endpoint is hit on every login attempt).
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()?;

        let mut providers = HashMap::new();
        if let (Some(id), Some(secret)) = (&cfg.google_client_id, &cfg.google_client_secret) {
            providers.insert(
                google::NAME,
                Provider::Oidc(Box::new(google::OidcProvider::google(
                    id.clone(),
                    secret.clone(),
                    cfg.google_issuer_url.clone(),
                ))),
            );
        }
        if let (Some(id), Some(secret)) = (&cfg.github_client_id, &cfg.github_client_secret) {
            providers.insert(
                github::NAME,
                Provider::Github(github::GithubProvider::new(
                    id.clone(),
                    secret.clone(),
                    cfg.github_base_url.clone(),
                    cfg.github_api_url.clone(),
                )),
            );
        }
        Ok(Self { providers, http })
    }

    pub fn get(&self, name: &str) -> Option<&Provider> {
        self.providers.get(name)
    }

    /// Provider names, stable order (for the scene library's sign-in buttons).
    pub fn names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.providers.keys().copied().collect();
        names.sort_unstable();
        names
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

/// Server-side record of an in-progress login, keyed by the CSRF `state` value.
pub struct Pending {
    pub provider: &'static str,
    /// PKCE code verifier to present at token exchange.
    pub pkce_verifier: String,
    /// OIDC nonce (None for plain OAuth2 providers).
    pub nonce: Option<String>,
    created: Instant,
}

impl Pending {
    pub fn new(provider: &'static str, pkce_verifier: String, nonce: Option<String>) -> Self {
        Self {
            provider,
            pkce_verifier,
            nonce,
            created: Instant::now(),
        }
    }
}

/// Maximum live pending flows. `auth_start` is unauthenticated, so without a cap an
/// attacker could insert at request speed for a whole [`FLOW_TTL`] window before the
/// prune touches anything (the same memory-DoS class the scene-snapshot store bounds with
/// [`crate::bounded_map::BoundedMap`]). Legitimate flows live seconds, so the cap costs real
/// users nothing.
const MAX_PENDING_FLOWS: usize = 4096;

/// In-memory map of pending flows. Flows live seconds and this is a single-process
/// server, so no persistence is needed; entries expire after [`FLOW_TTL`] and the map
/// is capped at [`MAX_PENDING_FLOWS`] (oldest evicted first).
#[derive(Clone, Default)]
pub struct PendingFlows {
    flows: Arc<Mutex<HashMap<String, Pending>>>,
}

impl PendingFlows {
    /// Stash a flow under its `state`, pruning stale entries while we hold the lock and
    /// evicting the oldest live entries past [`MAX_PENDING_FLOWS`].
    pub fn insert(&self, state: String, pending: Pending) {
        let mut guard = lock_unpoisoned(&self.flows);
        let now = Instant::now();
        guard.retain(|_, p| now.duration_since(p.created) < FLOW_TTL);
        while guard.len() >= MAX_PENDING_FLOWS {
            // O(n) scan, but n is capped and eviction only runs at the bound.
            let Some(oldest) = guard
                .iter()
                .min_by_key(|(_, p)| p.created)
                .map(|(state, _)| state.clone())
            else {
                break;
            };
            guard.remove(&oldest);
        }
        guard.insert(state, pending);
    }

    /// Take (consume) the flow for `state`; `None` if unknown or expired. Single use by
    /// construction — a replayed callback finds nothing.
    pub fn take(&self, state: &str) -> Option<Pending> {
        let mut guard = lock_unpoisoned(&self.flows);
        let pending = guard.remove(state)?;
        (Instant::now().duration_since(pending.created) < FLOW_TTL).then_some(pending)
    }

    /// Number of live flows (test-only introspection).
    #[cfg(test)]
    fn len(&self) -> usize {
        lock_unpoisoned(&self.flows).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `Pending` with an explicit age, so eviction order is deterministic in tests.
    fn pending_aged(now: Instant, age: Duration) -> Pending {
        Pending {
            provider: "github",
            pkce_verifier: "verifier".to_string(),
            nonce: None,
            created: now - age,
        }
    }

    #[test]
    fn pending_flows_stay_bounded_under_insert_pressure() {
        let flows = PendingFlows::default();
        let now = Instant::now();
        // Fill to the cap with live-but-older entries (well within the TTL, so the
        // TTL prune cannot touch them — exactly the attack window).
        for i in 0..MAX_PENDING_FLOWS {
            flows.insert(
                format!("state-{i}"),
                pending_aged(now, Duration::from_secs(60)),
            );
        }
        assert_eq!(flows.len(), MAX_PENDING_FLOWS);

        // A fresh flow at the cap evicts the oldest entry instead of growing the map.
        flows.insert("live".to_string(), pending_aged(now, Duration::ZERO));
        assert_eq!(
            flows.len(),
            MAX_PENDING_FLOWS,
            "cap holds at insert pressure"
        );

        // Sustained pressure keeps evicting oldest-first; the fresh flow survives and
        // still completes.
        for i in 0..100 {
            flows.insert(format!("extra-{i}"), pending_aged(now, Duration::ZERO));
        }
        assert_eq!(flows.len(), MAX_PENDING_FLOWS, "cap holds under pressure");
        assert!(
            flows.take("live").is_some(),
            "a live flow completes despite insert pressure"
        );
    }

    #[test]
    fn pending_flows_expire_after_ttl() {
        let flows = PendingFlows::default();
        let now = Instant::now();
        // A flow aged to the full TTL is past expiry by the time `take` runs (a hair of
        // real time elapses), so it must be rejected — the OAuth state-lifetime control.
        flows.insert("expired".to_string(), pending_aged(now, FLOW_TTL));
        assert!(
            flows.take("expired").is_none(),
            "a flow aged >= FLOW_TTL must not be taken"
        );
        // Boundary companion: a flow just inside the window is still taken.
        flows.insert(
            "live".to_string(),
            pending_aged(now, FLOW_TTL - Duration::from_secs(1)),
        );
        assert!(
            flows.take("live").is_some(),
            "a flow within the TTL is still taken"
        );
    }

    #[test]
    fn pending_flows_are_single_use() {
        let flows = PendingFlows::default();
        flows.insert(
            "state-1".to_string(),
            Pending::new("github", "verifier".to_string(), None),
        );
        assert!(flows.take("state-1").is_some());
        assert!(flows.take("state-1").is_none(), "second take must fail");
        assert!(flows.take("never-stored").is_none());
    }
}
