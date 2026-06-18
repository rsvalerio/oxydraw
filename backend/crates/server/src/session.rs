//! Login sessions for the scene library. Tokens are opaque random secrets; only
//! their SHA-256 hash is persisted (via [`SessionStore`]), so sessions survive restarts
//! while a leaked database never yields usable tokens.

use std::sync::Arc;

use oxydraw_core::model::Session;
use oxydraw_core::store::{Store, StoreError, TokenHash};
use rand::TryRngCore;
use sha2::{Digest, Sha256};

/// How long a login stays valid.
const SESSION_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// Cookie carrying the session token.
pub const SESSION_COOKIE: &str = "ext_session";

/// Store-backed session manager: mints, validates, and revokes login tokens.
#[derive(Clone)]
pub struct Sessions {
    store: Arc<dyn Store>,
}

impl Sessions {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }

    /// Mint a fresh token for `user_id`, pruning expired sessions while we're here.
    pub async fn mint(&self, user_id: &str) -> Result<String, oxydraw_core::store::StoreError> {
        // SEC-10: draw token entropy straight from the OS CSPRNG rather than a userspace
        // ThreadRng, so no userspace PRNG state (surviving fork/VM-clone) backs credential
        // material. This runs once per login, so the syscall cost is irrelevant.
        // ERR-5: the OS CSPRNG is near-infallible, but `try_fill_bytes` exists precisely
        // because it *can* fail — propagate that as a StoreError so a session mint on the
        // request path returns a controlled 500/retry instead of panicking the task.
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|e| StoreError::Backend(Box::new(e)))?;
        let token = hex_encode(&bytes);

        let now = chrono::Utc::now().timestamp();
        self.store.prune_sessions(now).await?;
        self.store
            .create_session(Session {
                token_hash: hash_token(&token),
                user_id: user_id.to_string(),
                expires_at: now + SESSION_TTL_SECS,
            })
            .await?;
        Ok(token)
    }

    /// The user id behind a live token, `Ok(None)` if the token is unknown/expired, or
    /// `Err` if the backend lookup itself failed — callers must distinguish the last case
    /// (a storage outage) from an absent session so it doesn't masquerade as a 401.
    pub async fn validate(&self, token: &str) -> Result<Option<String>, StoreError> {
        let session = match self.store.find_session(TokenHash(&hash_token(token))).await {
            Ok(session) => session,
            Err(StoreError::NotFound) => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok((session.expires_at > chrono::Utc::now().timestamp()).then_some(session.user_id))
    }

    /// Delete the session for `token`. Returns the store error so the caller can log it;
    /// a failed delete means the session stays valid until TTL and must not pass silently.
    pub async fn revoke(&self, token: &str) -> Result<(), StoreError> {
        self.store
            .delete_session(TokenHash(&hash_token(token)))
            .await
    }
}

/// SHA-256 hex of a session token — the only form that touches storage.
fn hash_token(token: &str) -> String {
    hex_encode(&Sha256::digest(token.as_bytes()))
}

/// Lowercase hex in one pre-sized buffer — a per-byte `format!` would allocate (and
/// drop) a `String` for every byte on each session mint/validate.
fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            // Writing to a String is infallible.
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// Extract the value of cookie `name` from a `Cookie` header value.
pub fn cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|pair| {
        let (n, value) = pair.trim().split_once('=')?;
        (n == name).then_some(value)
    })
}

/// Extract the session token from a `Cookie` header value.
pub fn session_token(cookie_header: &str) -> Option<&str> {
    cookie_value(cookie_header, SESSION_COOKIE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxydraw_storage::MemoryStore;

    fn sessions() -> Sessions {
        Sessions::new(Arc::new(MemoryStore::new()))
    }

    #[tokio::test]
    async fn minted_tokens_validate_until_revoked() {
        let sessions = sessions();
        let token = sessions.mint("user-1").await.unwrap();
        assert_eq!(token.len(), 64);
        assert_eq!(
            sessions.validate(&token).await.unwrap().as_deref(),
            Some("user-1")
        );
        assert_eq!(sessions.validate("not-a-token").await.unwrap(), None);

        sessions.revoke(&token).await.unwrap();
        assert_eq!(sessions.validate(&token).await.unwrap(), None);
    }

    #[tokio::test]
    async fn tokens_are_unique() {
        let sessions = sessions();
        assert_ne!(
            sessions.mint("u").await.unwrap(),
            sessions.mint("u").await.unwrap()
        );
    }

    #[tokio::test]
    async fn tokens_are_stored_hashed() {
        let store = Arc::new(MemoryStore::new());
        let sessions = Sessions::new(store.clone() as Arc<dyn Store>);
        let token = sessions.mint("u").await.unwrap();
        // The raw token must not be a valid lookup key in the store.
        use oxydraw_core::store::SessionStore;
        assert!(store.find_session(TokenHash(&token)).await.is_err());
        assert!(store
            .find_session(TokenHash(&hash_token(&token)))
            .await
            .is_ok());
    }

    #[test]
    fn parses_session_cookie_among_others() {
        assert_eq!(
            session_token("a=b; ext_session=tok123; c=d"),
            Some("tok123")
        );
        assert_eq!(session_token("ext_session=tok123"), Some("tok123"));
        assert_eq!(session_token("a=b; c=d"), None);
        assert_eq!(session_token(""), None);
    }
}
