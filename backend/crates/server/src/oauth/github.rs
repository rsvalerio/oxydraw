//! GitHub sign-in — the plain-OAuth2 bucket (GitHub does not implement OIDC). The shape
//! of this module is the template for other OAuth2-only providers (Facebook, Discord…):
//! authorization-code + PKCE via the `oauth2` crate, then a userinfo fetch mapped to
//! [`RemoteUser`].

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use super::{AuthError, BeginFlow, Pending, RemoteUser};

pub const NAME: &str = "github";

const DEFAULT_BASE_URL: &str = "https://github.com";
const DEFAULT_API_URL: &str = "https://api.github.com";

pub struct GithubProvider {
    client_id: String,
    client_secret: SecretString,
    /// Web base (authorize + token endpoints live here). Overridable for GitHub
    /// Enterprise and tests.
    base_url: String,
    /// REST API base (`/user`, `/user/emails`).
    api_url: String,
}

impl GithubProvider {
    pub fn new(
        client_id: String,
        client_secret: SecretString,
        base_url: Option<String>,
        api_url: Option<String>,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            api_url: api_url.unwrap_or_else(|| DEFAULT_API_URL.to_string()),
        }
    }

    #[allow(clippy::type_complexity)]
    fn client(
        &self,
        redirect_uri: &str,
    ) -> Result<
        BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
        AuthError,
    > {
        Ok(BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(
                self.client_secret.expose_secret().to_owned(),
            ))
            .set_auth_uri(
                AuthUrl::new(format!("{}/login/oauth/authorize", self.base_url))
                    .map_err(AuthError::setup)?,
            )
            .set_token_uri(
                TokenUrl::new(format!("{}/login/oauth/access_token", self.base_url))
                    .map_err(AuthError::setup)?,
            )
            .set_redirect_uri(
                RedirectUrl::new(redirect_uri.to_string()).map_err(AuthError::setup)?,
            ))
    }

    /// Authorization redirect plus the server-side flow record ([`BeginFlow`]).
    pub fn begin(&self, redirect_uri: &str) -> Result<BeginFlow, AuthError> {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, csrf) = self
            .client(redirect_uri)?
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("read:user".to_string()))
            .add_scope(Scope::new("user:email".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();
        let state = csrf.secret().clone();
        let pending = Pending::new(NAME, pkce_verifier.secret().clone(), None);
        Ok(BeginFlow {
            authorize_url: url.to_string(),
            state,
            pending,
        })
    }

    /// Token exchange + userinfo fetch.
    pub async fn exchange(
        &self,
        http: &reqwest::Client,
        redirect_uri: &str,
        code: &str,
        pending: Pending,
    ) -> Result<RemoteUser, AuthError> {
        let token = self
            .client(redirect_uri)?
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(PkceCodeVerifier::new(pending.pkce_verifier))
            .request_async(http)
            .await
            .map_err(AuthError::exchange)?;
        // SEC-21: hold the live provider access token in a redacting wrapper so it is
        // never directly `Display`/`Debug`-formattable — a future `error!`/`warn!`
        // interpolation can't accidentally leak it. It is exposed only at the single
        // `bearer_auth` call site in `api_get`.
        let access_token = SecretString::from(token.access_token().secret().to_owned());

        let user: GithubUser = self
            .api_get(http, &access_token, "/user")
            .await?
            .json()
            .await
            .map_err(AuthError::exchange)?;

        // SEC-11: the profile `email` from `/user` is the publicly settable, *unverified*
        // address — feeding it straight to the allowlist gate would let an attacker set
        // their public profile to an allowlisted address they do not control. Resolve the
        // gating email only from `/user/emails`, where GitHub reports `verified` (we hold
        // the `user:email` scope). This mirrors the `email_verified` guard in `google.rs`.
        let emails = self.fetch_emails(http, &access_token).await?;
        let email = gating_email(user.email, &emails);

        Ok(RemoteUser {
            provider: NAME,
            provider_user_id: user.id.to_string(),
            email,
            name: user.name.or(Some(user.login)),
            avatar_url: user.avatar_url,
        })
    }

    async fn api_get(
        &self,
        http: &reqwest::Client,
        access_token: &SecretString,
        path: &str,
    ) -> Result<reqwest::Response, AuthError> {
        let response = http
            .get(format!("{}{}", self.api_url, path))
            // The sole point the bearer token is exposed in plaintext.
            .bearer_auth(access_token.expose_secret())
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            // GitHub rejects requests without a User-Agent.
            .header(reqwest::header::USER_AGENT, "oxydraw")
            .send()
            .await
            .map_err(AuthError::exchange)?;
        response.error_for_status().map_err(AuthError::exchange)
    }

    async fn fetch_emails(
        &self,
        http: &reqwest::Client,
        access_token: &SecretString,
    ) -> Result<Vec<GithubEmail>, AuthError> {
        self.api_get(http, access_token, "/user/emails")
            .await?
            .json()
            .await
            .map_err(AuthError::exchange)
    }
}

/// Resolve the allowlist-gating email to a *verified* GitHub address (SEC-11). The
/// `profile_email` from `/user` is publicly settable and carries no verification, so it
/// is honored only when `/user/emails` reports that exact address as `verified`;
/// otherwise we fall back to the primary verified address. An account with no verified
/// email yields `None` and fails the gate closed.
fn gating_email(profile_email: Option<String>, emails: &[GithubEmail]) -> Option<String> {
    if let Some(profile) = profile_email {
        if emails.iter().any(|e| e.verified && e.email == profile) {
            return Some(profile);
        }
    }
    emails
        .iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email.clone())
}

#[derive(Deserialize)]
struct GithubUser {
    id: u64,
    login: String,
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GithubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(email: &str, primary: bool, verified: bool) -> GithubEmail {
        GithubEmail {
            email: email.to_string(),
            primary,
            verified,
        }
    }

    /// A profile email that is set but unverified must not reach the allowlist gate
    /// (SEC-11): only an address `/user/emails` reports as verified is usable, mirroring
    /// the `email_verified` coverage in `google.rs`.
    #[test]
    fn unverified_profile_email_does_not_gate() {
        let profile = || Some("attacker-claimed@example.com".to_string());

        // Profile email present but absent from /user/emails: not verified, drop it and
        // fall back to the primary verified address.
        let emails = vec![entry("real@example.com", true, true)];
        assert_eq!(
            gating_email(profile(), &emails),
            Some("real@example.com".to_string())
        );

        // Profile email present in /user/emails but flagged unverified: still not usable;
        // fall back to the primary verified address.
        let emails = vec![
            entry("attacker-claimed@example.com", false, false),
            entry("real@example.com", true, true),
        ];
        assert_eq!(
            gating_email(profile(), &emails),
            Some("real@example.com".to_string())
        );

        // No verified address at all fails closed to None.
        let emails = vec![entry("attacker-claimed@example.com", true, false)];
        assert_eq!(gating_email(profile(), &emails), None);
    }

    /// A profile email that /user/emails reports as verified is honored as-is.
    #[test]
    fn verified_profile_email_is_used() {
        let profile = || Some("alice@example.com".to_string());
        let emails = vec![
            entry("alice@example.com", false, true),
            entry("other@example.com", true, true),
        ];
        assert_eq!(gating_email(profile(), &emails), profile());
    }

    /// With no profile email (kept private), resolve the primary verified address.
    #[test]
    fn missing_profile_email_falls_back_to_primary_verified() {
        let emails = vec![
            entry("secondary@example.com", false, true),
            entry("primary@example.com", true, true),
        ];
        assert_eq!(
            gating_email(None, &emails),
            Some("primary@example.com".to_string())
        );
        // No verified address yields None.
        assert_eq!(
            gating_email(None, &[entry("x@example.com", true, false)]),
            None
        );
    }
}
