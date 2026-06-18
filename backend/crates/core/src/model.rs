//! Domain types.

/// A wall-clock timestamp in one canonical, fixed-width RFC 3339 UTC form
/// (`2026-01-02T03:04:05.123456Z` — microsecond precision, `Z` suffix), so its string
/// representation sorts lexicographically in chronological order. That property is
/// load-bearing: [`SceneStore::list_scenes`](crate::store::SceneStore::list_scenes)
/// promises "newest `updated_at` first", which the SQLite backend implements as a plain
/// `ORDER BY` over the TEXT column.
///
/// Construction goes through [`Timestamp::now`] or the validating [`Timestamp::parse`],
/// which normalizes any valid RFC 3339 input (offsets, other precisions) into the
/// canonical form — a misformatted or non-UTC value cannot enter the domain model.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(String);

impl Timestamp {
    /// The current time, in the canonical form.
    pub fn now() -> Self {
        Self::from_datetime(chrono::Utc::now())
    }

    /// Parse any valid RFC 3339 string, normalizing to the canonical UTC form.
    /// Errors only on input that is not RFC 3339.
    pub fn parse(s: &str) -> Result<Self, InvalidTimestamp> {
        let parsed =
            chrono::DateTime::parse_from_rfc3339(s).map_err(|source| InvalidTimestamp {
                input: s.to_string(),
                source,
            })?;
        Ok(Self::from_datetime(parsed.with_timezone(&chrono::Utc)))
    }

    fn from_datetime(dt: chrono::DateTime<chrono::Utc>) -> Self {
        Timestamp(dt.to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The Unix epoch — keeps `#[derive(Default)]` usable on the domain structs without
/// admitting a non-canonical (empty) value.
impl Default for Timestamp {
    fn default() -> Self {
        Self::from_datetime(chrono::DateTime::UNIX_EPOCH)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<Timestamp> for String {
    fn from(t: Timestamp) -> Self {
        t.0
    }
}

/// Returned by [`Timestamp::parse`] for input that is not valid RFC 3339.
#[derive(Debug, thiserror::Error)]
#[error("invalid RFC 3339 timestamp {input:?}")]
pub struct InvalidTimestamp {
    input: String,
    #[source]
    source: chrono::ParseError,
}

/// An anonymous, shareable document (the classic Excalidraw "export to link" payload).
/// `data` is the encrypted scene blob, kept opaque — the server never inspects contents.
#[derive(Debug, Clone, Default)]
pub struct Document {
    pub data: Vec<u8>,
}

/// A stored file object (scene images): the encrypted bytes plus the content type the
/// uploader declared. Like [`Document`], the data is opaque to the server.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredFile {
    pub content_type: String,
    pub data: Vec<u8>,
}

/// A saved-scene library entry (the scene library's scene list). The scene content itself
/// lives in `documents` as a share-link blob; this row carries the metadata the client needs
/// to reopen it — including the client-generated AES key (`#json=<id>,<key>`).
///
/// `Debug` is implemented manually so the AES key cannot leak into logs through a
/// casual `{:?}` — the key plus the (server-held) blob yields plaintext scene content,
/// so the type enforces redaction rather than vigilance at every log site (same
/// rationale as `Config`).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Scene {
    pub id: String,
    pub name: String,
    /// The `documents` row holding the encrypted scene blob.
    pub document_id: String,
    /// Scene AES key (JWK `.k`, base64url). Exposed only behind the scene library's auth.
    pub key: String,
    /// The organization this scene belongs to (`orgs.id`).
    pub owner: String,
    /// The [`Folder`] this scene lives in (`folders.id`); `None` means the org root
    /// ("Unfiled"). Existing flat scenes predate folders and read back as `None`.
    pub folder_id: Option<String>,
    /// The user who created the scene (`users.id`); `None` for scenes created in open
    /// (anonymous) mode or before per-user ownership existed. Recorded now so the future
    /// "owned by me" / per-user permission features have their column; not yet a filter.
    pub owner_user_id: Option<String>,
    /// Set by the caller; format and sort-order guarantees live on [`Timestamp`].
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl std::fmt::Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("document_id", &self.document_id)
            .field("key", &"***")
            .field("owner", &self.owner)
            .field("folder_id", &self.folder_id)
            .field("owner_user_id", &self.owner_user_id)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// A folder in the scene library's tree. Folders nest via `parent_id` (a `None` parent is
/// a root folder — each org has one seeded root, id `root:{org_id}`) and scope to an org
/// (the tenant boundary). `owner_user_id` records the creator for the future per-folder
/// permission/sharing features; it is not yet used as an access filter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Folder {
    pub id: String,
    pub name: String,
    /// The parent folder (`folders.id`); `None` for a root folder.
    pub parent_id: Option<String>,
    /// The organization this folder belongs to (`orgs.id`).
    pub org_id: String,
    /// The user who created the folder (`users.id`); `None` for the seeded org root and
    /// folders created in open (anonymous) mode.
    pub owner_user_id: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// A signed-in account. Profile fields come from whichever identity provider the user
/// last signed in with (or are empty for the built-in password-login user).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    /// Set by the store on creation, from the caller-supplied `now`.
    pub created_at: Timestamp,
}

/// A link between a [`User`] and an external identity provider account. The pair
/// `(provider, provider_user_id)` is the primary key, so one user can hold a Google
/// *and* a GitHub identity, and re-logins resolve to the same user.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    /// Registry name, e.g. `google`, `github` (or `local` for the password fallback).
    pub provider: String,
    /// The provider's stable user id (`sub` claim / GitHub numeric id).
    pub provider_user_id: String,
    pub user_id: String,
    pub email: Option<String>,
}

/// A persisted login session. Only the SHA-256 hash of the bearer token is stored, so a
/// leaked database does not yield usable session tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Session {
    pub token_hash: String,
    pub user_id: String,
    /// Unix seconds — integer so expiry comparisons work in SQL and in code.
    pub expires_at: i64,
}

/// An organization. Scenes belong to an org and members share its library. Today a
/// single default org exists and every user joins it; the schema supports more later.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Org {
    pub id: String,
    pub name: String,
    pub created_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two load-bearing guarantees: any valid RFC 3339 input lands in one canonical
    /// UTC form, and that form's lexicographic order is chronological order.
    #[test]
    fn timestamp_normalizes_to_a_sortable_canonical_form() {
        let plain = Timestamp::parse("2026-01-01T00:00:00Z").unwrap();
        assert_eq!(plain.as_str(), "2026-01-01T00:00:00.000000Z");

        // An offset input normalizes to the same UTC instant and representation.
        let offset = Timestamp::parse("2026-01-01T02:00:00+02:00").unwrap();
        assert_eq!(offset, plain);

        // Earlier instant with a "larger-looking" offset form still sorts earlier.
        let earlier = Timestamp::parse("2025-12-31T23:59:59+00:00").unwrap();
        assert!(earlier < plain);
        assert!(earlier.as_str() < plain.as_str());
    }

    #[test]
    fn timestamp_rejects_non_rfc3339_input() {
        for bad in ["", "12/06/2026", "2026-01-01", "not a date"] {
            assert!(Timestamp::parse(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn scene_debug_redacts_key() {
        let scene = Scene {
            id: "scene-1".to_string(),
            name: "demo".to_string(),
            document_id: "doc-1".to_string(),
            key: "u8ziBPV1JuPliY5wXkRWMQ".to_string(),
            owner: "org-1".to_string(),
            folder_id: None,
            owner_user_id: None,
            created_at: Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
            updated_at: Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
        };
        let out = format!("{scene:?}");
        assert!(
            !out.contains("u8ziBPV1JuPliY5wXkRWMQ"),
            "AES key leaked into Debug: {out}"
        );
        // Presence is still visible, just not the value; other fields stay readable.
        assert!(out.contains("key: \"***\""), "{out}");
        assert!(out.contains("name: \"demo\""), "{out}");
    }
}
