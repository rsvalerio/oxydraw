//! Google sign-in — the OIDC bucket. Identity comes from the **verified ID token**
//! (signature, issuer, audience, nonce), not from a userinfo call. The provider is
//! generic over its issuer, so a future Microsoft/GitLab login is this same struct with
//! a different issuer URL and scopes.

use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet,
    EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    TokenResponse,
};
use secrecy::{ExposeSecret, SecretString};
use tokio::sync::OnceCell;

use super::{AuthError, BeginFlow, Pending, RemoteUser};

/// The concrete client type produced by issuer discovery (auth endpoint guaranteed,
/// token/userinfo endpoints optional per the OIDC spec).
type DiscoveredClient = CoreClient<
    EndpointSet,      // auth url
    EndpointNotSet,   // device auth
    EndpointNotSet,   // introspection
    EndpointNotSet,   // revocation
    EndpointMaybeSet, // token url
    EndpointMaybeSet, // userinfo url
>;

pub const NAME: &str = "google";

const GOOGLE_ISSUER: &str = "https://accounts.google.com";

pub struct OidcProvider {
    name: &'static str,
    issuer: String,
    client_id: String,
    client_secret: SecretString,
    scopes: Vec<&'static str>,
    /// Discovery document, fetched lazily on first login (so the server boots offline)
    /// and cached for the process lifetime.
    metadata: OnceCell<CoreProviderMetadata>,
}

impl OidcProvider {
    pub fn google(client_id: String, client_secret: SecretString, issuer: Option<String>) -> Self {
        Self {
            name: NAME,
            issuer: issuer.unwrap_or_else(|| GOOGLE_ISSUER.to_string()),
            client_id,
            client_secret,
            scopes: vec!["email", "profile"],
            metadata: OnceCell::new(),
        }
    }

    async fn metadata(&self, http: &reqwest::Client) -> Result<&CoreProviderMetadata, AuthError> {
        self.metadata
            .get_or_try_init(|| async {
                let issuer = IssuerUrl::new(self.issuer.clone()).map_err(AuthError::setup)?;
                CoreProviderMetadata::discover_async(issuer, http)
                    .await
                    .map_err(AuthError::setup)
            })
            .await
    }

    async fn client(
        &self,
        http: &reqwest::Client,
        redirect_uri: &str,
    ) -> Result<DiscoveredClient, AuthError> {
        let metadata = self.metadata(http).await?.clone();
        Ok(CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(
                self.client_secret.expose_secret().to_owned(),
            )),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string()).map_err(AuthError::setup)?))
    }

    /// Authorization redirect plus the server-side flow record ([`BeginFlow`]).
    pub async fn begin(
        &self,
        http: &reqwest::Client,
        redirect_uri: &str,
    ) -> Result<BeginFlow, AuthError> {
        let client = self.client(http, redirect_uri).await?;
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let mut request = client.authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );
        for scope in &self.scopes {
            request = request.add_scope(Scope::new((*scope).to_string()));
        }
        let (url, csrf, nonce) = request.set_pkce_challenge(pkce_challenge).url();
        let state = csrf.secret().clone();
        let pending = Pending::new(
            self.name,
            pkce_verifier.secret().clone(),
            Some(nonce.secret().clone()),
        );
        Ok(BeginFlow {
            authorize_url: url.to_string(),
            state,
            pending,
        })
    }

    /// Token exchange + ID-token verification.
    pub async fn exchange(
        &self,
        http: &reqwest::Client,
        redirect_uri: &str,
        code: &str,
        pending: Pending,
    ) -> Result<RemoteUser, AuthError> {
        let client = self.client(http, redirect_uri).await?;
        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(AuthError::exchange)?
            .set_pkce_verifier(PkceCodeVerifier::new(pending.pkce_verifier))
            .request_async(http)
            .await
            .map_err(AuthError::exchange)?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| AuthError::verification("no ID token in response"))?;
        let nonce = pending_nonce(pending.nonce)?;
        let claims = id_token
            .claims(&client.id_token_verifier(), &nonce)
            .map_err(AuthError::verification)?;

        Ok(RemoteUser {
            provider: self.name,
            provider_user_id: claims.subject().to_string(),
            email: verified_email(
                claims.email().map(|e| e.to_string()),
                claims.email_verified(),
            ),
            // `name`/`picture` are localized claims; take the default locale.
            name: claims
                .name()
                .and_then(|n| n.get(None))
                .map(|n| n.to_string()),
            avatar_url: claims
                .picture()
                .and_then(|p| p.get(None))
                .map(|p| p.to_string()),
        })
    }
}

/// An OIDC `email` claim is only usable once the IdP also asserts `email_verified=true`
/// (SEC-11). The allowlist gate in `ext_routes::auth` keys solely on this email, so
/// accepting an unverified address would let an account holding an unverified — but
/// allowlisted — email pass the gate; `email_verified` is the canonical OIDC control
/// against exactly that. Anything other than an explicit `Some(true)` (absent claim or
/// `false`) fails closed to `None`.
fn verified_email(email: Option<String>, email_verified: Option<bool>) -> Option<String> {
    matches!(email_verified, Some(true))
        .then_some(email)
        .flatten()
}

/// Fail closed on a missing pending nonce (SEC-7): `begin` always stores one for OIDC,
/// so `None` here means the flow record is corrupt or a refactor broke the invariant —
/// verifying against a defaulted empty nonce would silently drop replay protection.
fn pending_nonce(nonce: Option<String>) -> Result<Nonce, AuthError> {
    nonce
        .map(Nonce::new)
        .ok_or_else(|| AuthError::verification("pending flow is missing the OIDC nonce"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_pending_nonce_rejects_the_flow() {
        let err = pending_nonce(None).expect_err("a missing nonce must fail verification");
        assert!(
            err.to_string().contains("verification"),
            "error is a verification failure: {err}"
        );
        let source = std::error::Error::source(&err).expect("error carries a source");
        assert!(
            source.to_string().contains("nonce"),
            "source names the missing nonce: {source}"
        );
    }

    #[test]
    fn present_nonce_is_passed_through() {
        let nonce = pending_nonce(Some("n-123".to_string())).expect("nonce accepted");
        assert_eq!(nonce.secret(), "n-123");
    }

    /// An unverified (or unasserted) email must not reach the allowlist gate (SEC-11):
    /// only an explicit `email_verified=true` yields a usable address.
    #[test]
    fn email_requires_verified_claim() {
        let email = || Some("alice@example.com".to_string());
        assert_eq!(verified_email(email(), Some(true)), email());
        // email_verified=false: an account with an unverified-but-allowlisted address
        // must not pass — fail closed to None.
        assert_eq!(verified_email(email(), Some(false)), None);
        // Claim absent entirely is treated the same as unverified.
        assert_eq!(verified_email(email(), None), None);
        // No email at all stays None regardless of the verified flag.
        assert_eq!(verified_email(None, Some(true)), None);
    }
}
