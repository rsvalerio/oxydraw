//! Runtime configuration, sourced from environment variables (and an optional `.env`).

use figment::{
    value::{Dict, Map, Value},
    Figment, Metadata, Profile, Provider,
};
use secrecy::SecretString;
use serde::{de, Deserialize};

/// Server configuration. All fields come from environment variables of the same name
/// (case-insensitive).
///
/// Secret-bearing fields (`ext_password`, OAuth client secrets) are [`SecretString`],
/// so redaction is type-enforced: their `Debug` prints `[REDACTED]` unconditionally
/// (letting this struct derive `Debug` safely), the buffer is zeroized on drop, and
/// every raw use site is greppable via `.expose_secret()` (SEC-5/SEC-6).
///
/// `deny_unknown_fields` keeps [`ENV_KEYS`] honest: the only data reaching serde comes
/// from that allowlist, so an entry with no matching field is drift, not operator input
/// (`every_env_key_populates_a_config_field` guards the reverse direction).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Bind address, e.g. `0.0.0.0:3002`.
    #[serde(default = "default_listen")]
    pub listen: String,
    /// Storage backend selector. See [`StorageType`].
    #[serde(default)]
    pub storage_type: StorageType,
    /// SQLite path or `sqlite:` URL (default `oxydraw.db`).
    pub data_source_name: Option<String>,
    /// Password protecting the scene library (`/api/ext/*`).
    /// Unset leaves the library open — fine on a trusted LAN, set it anywhere else.
    pub ext_password: Option<SecretString>,
    /// Comma-separated allowlist of email addresses permitted to sign in via OAuth
    /// (e.g. `me@example.com,team@example.com`). Unset leaves OAuth open to every
    /// account the provider will authenticate — set it on any internet-facing
    /// deployment so "Sign in with Google/GitHub" does not admit the whole world
    /// (SEC-18). Matching is case-insensitive; the password fallback is unaffected.
    pub ext_allowed_emails: Option<String>,
    /// Public base URL of this server (e.g. `https://draw.example.com`), used to build
    /// OAuth redirect URIs; an `https` scheme also forces the `Secure` attribute on
    /// login cookies regardless of request headers. Unset derives both per request from
    /// `Host`/`x-forwarded-proto`, which is only safe behind a reverse proxy that
    /// overwrites those headers.
    pub public_url: Option<String>,
    /// Google OAuth client (Sign in with Google). Both must be set to enable.
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<SecretString>,
    /// OIDC issuer for Google logins. Default `https://accounts.google.com`; override
    /// only for tests or a mock provider.
    pub google_issuer_url: Option<String>,
    /// GitHub OAuth App (Sign in with GitHub). Both must be set to enable.
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<SecretString>,
    /// GitHub web + API base URLs. Defaults `https://github.com` / `https://api.github.com`;
    /// override for GitHub Enterprise or tests.
    pub github_base_url: Option<String>,
    pub github_api_url: Option<String>,
    /// Comma-separated origins allowed to call the API cross-origin (e.g.
    /// `https://app.example.com,https://tool.example`). Unset (the default) emits no
    /// CORS headers at all — the bundled UI is same-origin, so nothing legitimate needs
    /// them. The literal `*` restores allow-everything; dev/debug use only.
    pub cors_allowed_origins: Option<String>,
    /// Total-size ceiling, in bytes, for anonymous shared documents (`/api/v2/post/`).
    /// New shares are rejected once the table reaches it. Default 1 GiB.
    #[serde(
        default = "default_max_documents_bytes",
        deserialize_with = "u64_from_string"
    )]
    pub max_documents_bytes: u64,
    /// Total-size ceiling, in bytes, for durable scene image files (`/api/files/{id}`).
    /// New uploads are rejected once the table reaches it. Default 1 GiB.
    #[serde(
        default = "default_max_files_bytes",
        deserialize_with = "u64_from_string"
    )]
    pub max_files_bytes: u64,
    /// SEC-33: row cap on the durable `files` table. With object-name length bounded, a
    /// row's disk cost is bounded; capping the row count then bounds total durable
    /// footprint even when payloads are tiny (or empty) and the byte quota never trips.
    /// Default 100,000 — generous (saved scenes have a handful of images each), so it
    /// only bounds an abuse loop.
    #[serde(
        default = "default_max_files_count",
        deserialize_with = "u64_from_string"
    )]
    pub max_files_count: u64,
}

/// The storage backend named by `STORAGE_TYPE`. A closed set: a typo'd value fails
/// config extraction with an error naming the legal options, instead of reaching the
/// backend dispatch as an arbitrary string.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StorageType {
    /// Embedded SQLite via sqlx (the default).
    #[default]
    Sqlite,
    /// In-process, volatile (dev/tests).
    Memory,
}

impl StorageType {
    pub fn as_str(self) -> &'static str {
        match self {
            StorageType::Sqlite => "sqlite",
            StorageType::Memory => "memory",
        }
    }
}

impl std::fmt::Display for StorageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Hand-written (rather than serde-derived) for two env-var ergonomics: matching is
/// case-insensitive (`STORAGE_TYPE=Memory` works), and a set-but-empty variable behaves
/// like unset (the default) rather than erroring.
impl<'de> Deserialize<'de> for StorageType {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        match raw.to_ascii_lowercase().as_str() {
            "" | "sqlite" => Ok(StorageType::Sqlite),
            "memory" => Ok(StorageType::Memory),
            other => Err(de::Error::custom(format!(
                "unknown storage type {other:?}; expected `sqlite` or `memory`"
            ))),
        }
    }
}

fn default_listen() -> String {
    "0.0.0.0:3002".to_string()
}

fn default_max_documents_bytes() -> u64 {
    1 << 30 // 1 GiB
}

fn default_max_files_bytes() -> u64 {
    1 << 30 // 1 GiB
}

fn default_max_files_count() -> u64 {
    100_000
}

impl Default for Config {
    /// The same values an empty environment produces (serde defaults + `None`s).
    fn default() -> Self {
        Self {
            listen: default_listen(),
            storage_type: StorageType::default(),
            data_source_name: None,
            ext_password: None,
            ext_allowed_emails: None,
            public_url: None,
            google_client_id: None,
            google_client_secret: None,
            google_issuer_url: None,
            github_client_id: None,
            github_client_secret: None,
            github_base_url: None,
            github_api_url: None,
            cors_allowed_origins: None,
            max_documents_bytes: default_max_documents_bytes(),
            max_files_bytes: default_max_files_bytes(),
            max_files_count: default_max_files_count(),
        }
    }
}

/// Parse a `u64` field that [`RawEnv`] delivers as a verbatim string.
fn u64_from_string<'de, D: de::Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let raw = String::deserialize(deserializer)?;
    raw.parse()
        .map_err(|e| de::Error::custom(format!("expected an unsigned integer: {e}")))
}

/// Environment variables read into [`Config`]. Restricting to this set avoids slurping
/// unrelated process environment (PATH, HOME, …) into the figment.
const ENV_KEYS: &[&str] = &[
    "LISTEN",
    "STORAGE_TYPE",
    "DATA_SOURCE_NAME",
    "EXT_PASSWORD",
    "EXT_ALLOWED_EMAILS",
    "PUBLIC_URL",
    "GOOGLE_CLIENT_ID",
    "GOOGLE_CLIENT_SECRET",
    "GOOGLE_ISSUER_URL",
    "GITHUB_CLIENT_ID",
    "GITHUB_CLIENT_SECRET",
    "GITHUB_BASE_URL",
    "GITHUB_API_URL",
    "CORS_ALLOWED_ORIGINS",
    "MAX_DOCUMENTS_BYTES",
    "MAX_FILES_BYTES",
    "MAX_FILES_COUNT",
];

/// Env-var provider that keeps every value a verbatim string. Figment's own `Env`
/// provider type-infers values before serde sees them, which breaks string fields whose
/// value happens to look like another type: `EXT_PASSWORD=123456` arrived as an integer
/// (a type error for the `String` field) and `EXT_PASSWORD=007` would silently become
/// `7`. The `u64` fields parse explicitly via [`u64_from_string`].
struct RawEnv;

impl Provider for RawEnv {
    fn metadata(&self) -> Metadata {
        Metadata::named("environment variable(s)")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        let mut dict = Dict::new();
        for key in ENV_KEYS {
            match std::env::var(key) {
                Ok(value) => {
                    dict.insert(key.to_ascii_lowercase(), Value::from(value));
                }
                Err(std::env::VarError::NotPresent) => {}
                // A set-but-unreadable variable must not behave like "unset": for
                // EXT_PASSWORD or EXT_ALLOWED_EMAILS that would silently weaken auth
                // while the operator believes it took effect (SEC-31, fail closed).
                Err(std::env::VarError::NotUnicode(_)) => {
                    return Err(figment::Error::from(format!(
                        "environment variable {key} contains invalid UTF-8"
                    )));
                }
            }
        }
        Ok(Map::from([(Profile::Default, dict)]))
    }
}

impl Config {
    /// Load configuration: optional `.env`, then environment variables.
    ///
    /// A missing `.env` is fine — the file is optional. A `.env` that exists but fails
    /// to parse is a hard error: dotenvy stops loading at the bad line, so continuing
    /// would silently run with defaults instead of the operator's configuration
    /// (`EXT_PASSWORD` unset means an open scene library — SEC-30, fail fast).
    pub fn from_env() -> anyhow::Result<Self> {
        match dotenvy::dotenv() {
            Ok(_) => {}
            Err(e) if e.not_found() => {}
            Err(e) => return Err(anyhow::Error::new(e).context("failed to load .env")),
        }
        Self::from_figment(Figment::new())
    }

    fn from_figment(base: Figment) -> anyhow::Result<Self> {
        let cfg = base.merge(RawEnv).extract()?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    // The `Jail::expect_with` closure's Err type (figment::Error) is large; immaterial in tests.
    #![allow(clippy::result_large_err)]

    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn defaults_apply_with_empty_env() {
        figment::Jail::expect_with(|_jail| {
            let cfg = Config::from_figment(Figment::new()).unwrap();
            assert_eq!(cfg.listen, "0.0.0.0:3002");
            assert_eq!(cfg.storage_type, StorageType::Sqlite);
            assert!(cfg.data_source_name.is_none());
            assert_eq!(cfg.max_documents_bytes, 1 << 30);
            assert_eq!(cfg.max_files_bytes, 1 << 30);
            assert_eq!(cfg.max_files_count, 100_000);
            Ok(())
        });
    }

    #[test]
    fn debug_output_redacts_secrets() {
        let cfg = Config {
            ext_password: Some("hunter2".into()),
            google_client_secret: Some("g-secret".into()),
            github_client_secret: Some("gh-secret".into()),
            ..Config::default()
        };
        let out = format!("{cfg:?}");
        for secret in ["hunter2", "g-secret", "gh-secret"] {
            assert!(!out.contains(secret), "secret leaked into Debug: {out}");
        }
        // Presence is still visible, just not the value.
        assert!(out.contains("ext_password: Some("), "{out}");
        assert!(out.contains("REDACTED"), "{out}");
    }

    /// Values that *look* like another type must stay verbatim strings — a numeric
    /// password must neither fail extraction (`123456`) nor be corrupted (`007` → `7`).
    #[test]
    fn numeric_looking_strings_stay_verbatim() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("EXT_PASSWORD", "007");
            jail.set_env("GITHUB_CLIENT_ID", "123456");
            jail.set_env("GOOGLE_CLIENT_SECRET", "true");
            let cfg = Config::from_figment(Figment::new()).unwrap();
            assert_eq!(
                cfg.ext_password.as_ref().map(|s| s.expose_secret()),
                Some("007")
            );
            assert_eq!(cfg.github_client_id.as_deref(), Some("123456"));
            assert_eq!(
                cfg.google_client_secret.as_ref().map(|s| s.expose_secret()),
                Some("true")
            );
            Ok(())
        });
    }

    /// AC for the `StorageType` enum: a typo'd backend name fails at config extraction
    /// (not at backend dispatch), and the error names the legal options.
    #[test]
    fn invalid_storage_type_fails_extraction_naming_the_options() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("STORAGE_TYPE", "sqllite");
            let err = Config::from_figment(Figment::new())
                .expect_err("typo'd STORAGE_TYPE must not load");
            let msg = err.to_string();
            for needle in ["sqllite", "sqlite", "memory"] {
                assert!(msg.contains(needle), "{msg}");
            }
            Ok(())
        });
    }

    /// Set-but-empty behaves like unset; matching is case-insensitive.
    #[test]
    fn storage_type_tolerates_empty_and_mixed_case() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("STORAGE_TYPE", "");
            let cfg = Config::from_figment(Figment::new()).unwrap();
            assert_eq!(cfg.storage_type, StorageType::Sqlite);

            jail.set_env("STORAGE_TYPE", "Memory");
            let cfg = Config::from_figment(Figment::new()).unwrap();
            assert_eq!(cfg.storage_type, StorageType::Memory);
            Ok(())
        });
    }

    #[test]
    fn rejects_non_numeric_size_ceilings() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("MAX_DOCUMENTS_BYTES", "a lot");
            assert!(Config::from_figment(Figment::new()).is_err());
            Ok(())
        });
    }

    /// `ENV_KEYS` and the `Config` field list are two hand-maintained copies of the
    /// same fact; this test fails when they drift in either direction. Every key is set
    /// to a distinct value and checked end-to-end; the exhaustive destructuring (no
    /// `..`) breaks compilation when a field is added without extending the test, and
    /// `deny_unknown_fields` makes a key without a field fail extraction.
    #[test]
    fn every_env_key_populates_a_config_field() {
        figment::Jail::expect_with(|jail| {
            // Distinct per-key values; numeric strings so the u64 fields parse too.
            // STORAGE_TYPE is the one closed-domain field, so it gets a legal variant.
            let value_for = |key: &str| {
                let i = ENV_KEYS.iter().position(|k| *k == key).expect(key);
                (1000 + i).to_string()
            };
            for key in ENV_KEYS {
                if *key == "STORAGE_TYPE" {
                    jail.set_env(key, "memory");
                } else {
                    jail.set_env(key, value_for(key));
                }
            }
            let Config {
                listen,
                storage_type,
                data_source_name,
                ext_password,
                ext_allowed_emails,
                public_url,
                google_client_id,
                google_client_secret,
                google_issuer_url,
                github_client_id,
                github_client_secret,
                github_base_url,
                github_api_url,
                cors_allowed_origins,
                max_documents_bytes,
                max_files_bytes,
                max_files_count,
            } = Config::from_figment(Figment::new()).unwrap();
            assert_eq!(listen, value_for("LISTEN"));
            assert_eq!(storage_type, StorageType::Memory);
            assert_eq!(data_source_name, Some(value_for("DATA_SOURCE_NAME")));
            assert_eq!(
                ext_password.as_ref().map(|s| s.expose_secret()),
                Some(value_for("EXT_PASSWORD").as_str())
            );
            assert_eq!(ext_allowed_emails, Some(value_for("EXT_ALLOWED_EMAILS")));
            assert_eq!(public_url, Some(value_for("PUBLIC_URL")));
            assert_eq!(google_client_id, Some(value_for("GOOGLE_CLIENT_ID")));
            assert_eq!(
                google_client_secret.as_ref().map(|s| s.expose_secret()),
                Some(value_for("GOOGLE_CLIENT_SECRET").as_str())
            );
            assert_eq!(google_issuer_url, Some(value_for("GOOGLE_ISSUER_URL")));
            assert_eq!(github_client_id, Some(value_for("GITHUB_CLIENT_ID")));
            assert_eq!(
                github_client_secret.as_ref().map(|s| s.expose_secret()),
                Some(value_for("GITHUB_CLIENT_SECRET").as_str())
            );
            assert_eq!(github_base_url, Some(value_for("GITHUB_BASE_URL")));
            assert_eq!(github_api_url, Some(value_for("GITHUB_API_URL")));
            assert_eq!(
                cors_allowed_origins,
                Some(value_for("CORS_ALLOWED_ORIGINS"))
            );
            assert_eq!(
                max_documents_bytes.to_string(),
                value_for("MAX_DOCUMENTS_BYTES")
            );
            assert_eq!(max_files_bytes.to_string(), value_for("MAX_FILES_BYTES"));
            assert_eq!(max_files_count.to_string(), value_for("MAX_FILES_COUNT"));
            Ok(())
        });
    }

    /// A non-Unicode value must fail loading, not silently count as unset — "unset"
    /// here means an open scene library / disabled OAuth allowlist (SEC-31).
    #[cfg(unix)]
    #[test]
    fn non_unicode_env_value_is_an_error() {
        use std::os::unix::ffi::OsStrExt;
        // Jail for its serialization lock; its `set_env` only takes UTF-8, so set the
        // raw bytes directly and clean up before asserting.
        figment::Jail::expect_with(|_jail| {
            std::env::set_var("EXT_PASSWORD", std::ffi::OsStr::from_bytes(b"\xff\xfe"));
            let result = Config::from_figment(Figment::new());
            std::env::remove_var("EXT_PASSWORD");
            let err = result.expect_err("non-Unicode EXT_PASSWORD must not load");
            assert!(err.to_string().contains("EXT_PASSWORD"), "{err}");
            Ok(())
        });
    }

    /// A `.env` that exists but cannot be parsed must fail loading, not silently fall
    /// back to defaults (SEC-30). The jail's temp cwd is where `dotenvy` looks first.
    #[test]
    fn malformed_dotenv_is_an_error() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(".env", "NOT A VALID DOTENV LINE")?;
            let err = Config::from_env().expect_err("malformed .env must not load");
            assert!(err.to_string().contains(".env"), "{err}");
            Ok(())
        });
    }

    #[test]
    fn reads_env_overrides() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("STORAGE_TYPE", "memory");
            jail.set_env("LISTEN", "127.0.0.1:8080");
            jail.set_env("DATA_SOURCE_NAME", "/var/lib/oxydraw/oxydraw.db");
            jail.set_env("MAX_DOCUMENTS_BYTES", "1048576");
            jail.set_env("MAX_FILES_BYTES", "2097152");
            jail.set_env("MAX_FILES_COUNT", "500");
            let cfg = Config::from_figment(Figment::new()).unwrap();
            assert_eq!(cfg.storage_type, StorageType::Memory);
            assert_eq!(cfg.listen, "127.0.0.1:8080");
            assert_eq!(
                cfg.data_source_name.as_deref(),
                Some("/var/lib/oxydraw/oxydraw.db")
            );
            assert_eq!(cfg.max_documents_bytes, 1_048_576);
            assert_eq!(cfg.max_files_bytes, 2_097_152);
            assert_eq!(cfg.max_files_count, 500);
            Ok(())
        });
    }
}
