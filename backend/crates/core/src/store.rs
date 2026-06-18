//! Storage traits. Backends live in the `oxydraw-storage` crate.
//!
//! ASYNC-10 note: these traits use `#[async_trait]` rather than native `async fn in
//! trait` deliberately — the server consumes backends as `Arc<dyn Store>` (selected at
//! runtime by `STORAGE_TYPE`), and native async trait methods are not dyn-compatible.
//! The per-call `Box<dyn Future>` is the price of that dynamic dispatch.

use async_trait::async_trait;

use crate::model::{Document, Folder, Org, Scene, Session, StoredFile, Timestamp, User};

/// Errors a storage backend can return.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("not found")]
    NotFound,
    /// The boxed source keeps the backend's error chain walkable (`Error::source()`)
    /// without `oxydraw-core` depending on any backend crate.
    #[error("storage backend error")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// A row read from the store violated a domain invariant — e.g. an enum column held a
    /// string outside its closed set ([`Role`]/[`PrincipalKind`]/[`Permission`]). Distinct
    /// from [`Backend`](StoreError::Backend) (transport/IO/driver faults) so a single
    /// corrupt row is diagnosable as a data-integrity fault rather than a DB outage.
    #[error("corrupt stored data")]
    Decode(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("the `{0}` storage backend is not yet implemented")]
    Unimplemented(&'static str),
}

/// Lets sqlx-backed stores propagate driver errors with plain `?` instead of repeating
/// `.map_err(...)` on every call. Feature-gated so core only pulls sqlx when a backend
/// actually needs the conversion (the orphan rule blocks this impl in the storage crate).
#[cfg(feature = "sqlx-error")]
impl From<sqlx::Error> for StoreError {
    fn from(e: sqlx::Error) -> Self {
        StoreError::Backend(Box::new(e))
    }
}

/// Borrowed newtype wrappers for the domain ids that appear in multi-id store signatures,
/// so two adjacent ids can't be silently swapped at a call or impl site (API-2) — a swap
/// becomes a compile error rather than a runtime mix-up, at zero cost (`Copy` `&str`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserId<'a>(pub &'a str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrgId<'a>(pub &'a str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FolderId<'a>(pub &'a str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupId<'a>(pub &'a str);

/// The SHA-256 hash of a session token — never the raw token. Keeps the security-relevant
/// distinction (the store persists hashes; callers hold raw tokens) in the type system, so
/// a raw token can't be passed where a hash is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenHash<'a>(pub &'a str);

/// An org member's role — a closed set, so a typo'd role cannot be persisted and later
/// authorization checks can match exhaustively. String conversion happens only at the
/// storage serialization boundary ([`Role::as_str`] / [`FromStr`](std::str::FromStr)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Member,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Member => "member",
            Role::Admin => "admin",
        }
    }
}

impl std::str::FromStr for Role {
    type Err = InvalidRole;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "member" => Ok(Role::Member),
            "admin" => Ok(Role::Admin),
            other => Err(InvalidRole(other.to_string())),
        }
    }
}

/// Returned by [`Role`]'s `FromStr` for a string outside the closed role set.
#[derive(Debug, thiserror::Error)]
#[error("unknown role {0:?}; expected `member` or `admin`")]
pub struct InvalidRole(String);

/// What kind of principal an ACL grant ([`FolderGrant`]) names. A closed set, mirroring
/// [`Role`], so a typo cannot be persisted and authorization can match exhaustively.
/// `User` ids are `users.id`; `Group` ids are `groups.id` — so teams join the same ACL
/// table as user grants with no schema change (the `(kind, id)` pair is the key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalKind {
    User,
    Group,
}

impl PrincipalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PrincipalKind::User => "user",
            PrincipalKind::Group => "group",
        }
    }
}

impl std::str::FromStr for PrincipalKind {
    type Err = InvalidPrincipalKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(PrincipalKind::User),
            "group" => Ok(PrincipalKind::Group),
            other => Err(InvalidPrincipalKind(other.to_string())),
        }
    }
}

/// Returned by [`PrincipalKind`]'s `FromStr` for a string outside the closed set.
#[derive(Debug, thiserror::Error)]
#[error("unknown principal kind {0:?}; expected `user` or `group`")]
pub struct InvalidPrincipalKind(String);

/// A folder access level. The variant order is ascending privilege, and `PartialOrd`/`Ord`
/// derive from that order, so an authorization check is a direct `effective >= required`
/// comparison. `Viewer` lists/opens scenes; `Editor` adds create/rename/move/delete of
/// scenes and subfolders; `Admin` adds managing the folder's ACL and the folder itself.
///
/// `#[must_use]`: this is an authorization decision (the level a caller is granted on a
/// folder). Dropping it as a statement — `effective_permission(..).await?;` without
/// comparing against the required level — silently bypasses the guard, so an accidental
/// discard is upgraded to a compile error under CI's `-D warnings`.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    Viewer,
    Editor,
    Admin,
}

impl Permission {
    pub fn as_str(self) -> &'static str {
        match self {
            Permission::Viewer => "viewer",
            Permission::Editor => "editor",
            Permission::Admin => "admin",
        }
    }
}

impl std::str::FromStr for Permission {
    type Err = InvalidPermission;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "viewer" => Ok(Permission::Viewer),
            "editor" => Ok(Permission::Editor),
            "admin" => Ok(Permission::Admin),
            other => Err(InvalidPermission(other.to_string())),
        }
    }
}

/// Returned by [`Permission`]'s `FromStr` for a string outside the closed set.
#[derive(Debug, thiserror::Error)]
#[error("unknown permission {0:?}; expected `viewer`, `editor`, or `admin`")]
pub struct InvalidPermission(String);

/// One ACL grant on a folder: a principal (user or group) and the level it confers.
/// `granted_by` is the user who created the grant, kept for audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderGrant {
    pub principal_kind: PrincipalKind,
    pub principal_id: String,
    pub permission: Permission,
    pub granted_by: Option<String>,
}

/// Persistence for anonymous, shareable documents (`/api/v2/post/`, `/api/v2/{id}`).
#[async_trait]
pub trait DocumentStore: Send + Sync {
    async fn find_id(&self, id: &str) -> Result<Document, StoreError>;
    async fn create(&self, document: Document) -> Result<String, StoreError>;
    /// Total bytes of stored document payloads, for quota enforcement on anonymous writes.
    async fn documents_total_bytes(&self) -> Result<u64, StoreError>;
}

/// Persistence for the scene library (`/api/ext/scenes`). Rows are
/// metadata only; scene content lives in [`DocumentStore`] as share-link blobs.
///
/// SEC-33 (result-set bound): the `list_*` queries return a full owner-scoped `Vec`
/// with no `LIMIT`/pagination, but the set is not unbounded — scene creation enforces a
/// per-owner cap (`MAX_SCENES_PER_OWNER` in the `scenes` create handler), so an owner can
/// hold at most that many rows and a list materializes at most that many. The cap is the
/// enforced upper bound; both store backends inherit it because writes funnel through the
/// same handler. Add keyset pagination here only if that cap is ever raised to a size a
/// single response should not carry.
#[async_trait]
pub trait SceneStore: Send + Sync {
    /// All scenes owned by `owner`, newest `updated_at` first. Bounded by the per-owner
    /// scene cap enforced at creation (see the trait-level SEC-33 note).
    async fn list_scenes(&self, owner: &str) -> Result<Vec<Scene>, StoreError>;
    /// Scenes owned by `owner` that live directly in `folder` (not descendants), newest
    /// `updated_at` first. `None` selects the org root — scenes with `folder_id IS NULL`.
    /// A subset of [`list_scenes`](Self::list_scenes), so the same per-owner cap bounds it.
    async fn list_scenes_in_folder(
        &self,
        owner: &str,
        folder: Option<FolderId<'_>>,
    ) -> Result<Vec<Scene>, StoreError>;
    /// Number of scenes owned by `owner`, for quota enforcement on scene creation.
    async fn count_scenes(&self, owner: &str) -> Result<u64, StoreError>;
    async fn create_scene(&self, scene: Scene) -> Result<(), StoreError>;
    async fn find_scene(&self, id: &str) -> Result<Scene, StoreError>;
    /// Move a scene owned by `owner` into `folder` (`None` = org root) and bump
    /// `updated_at`. The org (tenant) boundary is enforced at the store, not only by the
    /// caller (SEC-20): a scene not owned by `owner`, or a destination `folder` that exists
    /// but belongs to another org, is rejected as `NotFound` — so a future caller that omits
    /// the paired handler check still cannot relocate data across tenants.
    async fn move_scene(
        &self,
        id: &str,
        owner: OrgId<'_>,
        folder: Option<FolderId<'_>>,
        now: Timestamp,
    ) -> Result<(), StoreError>;
    /// Rename a scene and bump `updated_at`. `NotFound` when the scene does not exist.
    async fn rename_scene(&self, id: &str, name: &str, now: Timestamp) -> Result<(), StoreError>;
    async fn delete_scene(&self, id: &str) -> Result<(), StoreError>;
}

/// Durable persistence for scene image files (`/api/files/{id}`), keyed by their
/// content-addressed id. The bytes are opaque to the server.
#[async_trait]
pub trait FileStore: Send + Sync {
    async fn put_file(&self, path: &str, file: StoredFile) -> Result<(), StoreError>;
    async fn get_file(&self, path: &str) -> Result<StoredFile, StoreError>;
    /// Total bytes of stored file payloads, for quota enforcement on anonymous writes.
    async fn files_total_bytes(&self) -> Result<u64, StoreError>;
    /// Number of stored file rows, for the durable-files row cap. Together with an
    /// object-name length bound this stops unauthenticated tiny-payload/long-name uploads
    /// from growing the table without limit when the byte quota never trips (SEC-33).
    async fn count_files(&self) -> Result<u64, StoreError>;
}

/// What an identity provider told us about a user, normalized across providers. Input
/// to [`UserStore::upsert_user_for_identity`].
///
/// `Debug` is derived even though `email`/`name` are PII: they are display-profile
/// fields, not credentials (unlike [`Scene::key`](crate::model::Scene) or `Config`
/// secrets, which redact), and the same fields appear on `User` and `Identity` with
/// derived `Debug` — redacting only this type would give false assurance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityProfile {
    /// Registry name, e.g. `google`, `github` (or `local` for the password fallback).
    pub provider: String,
    /// The provider's stable user id (`sub` claim / GitHub numeric id).
    pub provider_user_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Accounts and their links to identity providers.
#[async_trait]
pub trait UserStore: Send + Sync {
    /// Resolve `(provider, provider_user_id)` to its user, creating user + identity on
    /// first login and refreshing profile fields on every login. `now` becomes the
    /// `created_at` of newly created users (timestamps are set by the caller, as with
    /// [`Scene`]).
    async fn upsert_user_for_identity(
        &self,
        profile: &IdentityProfile,
        now: Timestamp,
    ) -> Result<User, StoreError>;
    async fn find_user(&self, id: &str) -> Result<User, StoreError>;
}

/// Persisted login sessions. Rows hold the SHA-256 hash of the bearer token, never the
/// token itself; expiry checks live in the caller (`server::session`).
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self, session: Session) -> Result<(), StoreError>;
    async fn find_session(&self, token_hash: TokenHash<'_>) -> Result<Session, StoreError>;
    async fn delete_session(&self, token_hash: TokenHash<'_>) -> Result<(), StoreError>;
    /// Drop every session with `expires_at <= now` (unix seconds).
    async fn prune_sessions(&self, now: i64) -> Result<(), StoreError>;
}

/// Why a folder create/move was rejected. Distinct from [`StoreError`] because the HTTP
/// layer maps `StoreError` (other than `NotFound`) to 500, whereas a cycle or excessive
/// nesting is a *client* error: the handler maps `Cycle` → 409, `TooDeep` → 422,
/// `NotFound` → 404, and `Backend` → 500.
///
/// `#[must_use]`: every variant is a security-relevant rejection (`Cycle`/`TooDeep`/
/// `NotFound`); discarding the `Err` would let a rejected create/move appear to succeed.
/// `Result` is already `#[must_use]`, but marking the error type too keeps the guarantee
/// if it is ever returned or matched outside a `Result`.
#[must_use]
#[derive(Debug, thiserror::Error)]
pub enum FolderMoveError {
    #[error("folder operation would create a cycle")]
    Cycle,
    #[error("folder nesting would exceed the maximum depth")]
    TooDeep,
    #[error("not found")]
    NotFound,
    #[error("storage backend error")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<StoreError> for FolderMoveError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NotFound => FolderMoveError::NotFound,
            other => FolderMoveError::Backend(Box::new(other)),
        }
    }
}

/// Maximum folder nesting depth (root counts as depth 0). Enforced by create/move so the
/// recursive ancestor/subtree walks stay bounded and the tree cannot grow pathologically
/// deep. A generous bound — real folder hierarchies are shallow.
pub const MAX_FOLDER_DEPTH: usize = 32;

/// Persistence for the scene library's folder tree and its access-control list. Folders
/// nest via `parent_id`, scope to an org, and carry an ACL (`folder_permissions`).
///
/// The ACL methods ([`effective_permission`](FolderStore::effective_permission),
/// `set_permission`, `list_permissions`, `remove_permission`) are part of the contract
/// now so both backends and the contract tests cover the foundation; the write methods
/// are exercised by the future sharing endpoints, while part-1 handlers only read
/// `effective_permission` and the folder CRUD.
///
/// SEC-33 (result-set bound): like [`SceneStore`], the `list_*` queries return a full
/// `Vec` with no `LIMIT`, but neither set is unbounded. Folder creation enforces a
/// per-org folder cap (`MAX_FOLDERS_PER_OWNER` in the `folders` create handler), so
/// `list_folders` materializes at most that many rows across the whole org. ACL grants
/// (`list_permissions`) are created only via `set_permission`, which has no HTTP route
/// yet — so grants cannot grow from untrusted input today; the future sharing endpoint
/// that exposes it must enforce a grants-per-folder cap (the ACL analog of the scene/folder
/// caps) so this list stays bounded once writes are reachable.
#[async_trait]
pub trait FolderStore: Send + Sync {
    /// Create the org's root folder if absent (id `root:{org_id}`, `parent_id = None`),
    /// returning its id. Idempotent — safe to call on every org bootstrap.
    async fn ensure_root_folder(
        &self,
        org_id: OrgId<'_>,
        now: Timestamp,
    ) -> Result<String, StoreError>;
    async fn find_folder(&self, id: FolderId<'_>) -> Result<Folder, StoreError>;
    /// Direct children of `parent` within `org_id`, ordered by name. `None` selects the
    /// org's root folders (`parent_id IS NULL`). Bounded by the per-org folder cap enforced
    /// at creation (see the trait-level SEC-33 note).
    async fn list_folders(
        &self,
        org_id: OrgId<'_>,
        parent: Option<FolderId<'_>>,
    ) -> Result<Vec<Folder>, StoreError>;
    /// Number of folders in `org_id`, for quota enforcement on folder creation.
    async fn count_folders(&self, org_id: OrgId<'_>) -> Result<u64, StoreError>;
    /// Create a folder. Rejects a parent that does not exist (`NotFound`) or that would
    /// exceed [`MAX_FOLDER_DEPTH`] (`TooDeep`).
    async fn create_folder(&self, folder: Folder) -> Result<(), FolderMoveError>;
    async fn rename_folder(
        &self,
        id: FolderId<'_>,
        name: &str,
        now: Timestamp,
    ) -> Result<(), StoreError>;
    /// Reparent a folder (owned by `org`) under `new_parent` (`None` = a root folder) and
    /// bump `updated_at`. Rejects making the folder its own ancestor (`Cycle`) or exceeding
    /// [`MAX_FOLDER_DEPTH`] (`TooDeep`); `NotFound` if the folder or new parent is absent.
    /// The org (tenant) boundary is enforced at the store, not only by the caller (SEC-20):
    /// a folder or a `new_parent` outside `org` is rejected as `NotFound`, so the store
    /// cannot reparent across tenants even if a future caller omits the handler check. The
    /// validation reads and the write run in one transaction, so concurrent reparents cannot
    /// interleave into a cycle or past `MAX_FOLDER_DEPTH` (CONC-2).
    async fn move_folder(
        &self,
        id: FolderId<'_>,
        org: OrgId<'_>,
        new_parent: Option<FolderId<'_>>,
        now: Timestamp,
    ) -> Result<(), FolderMoveError>;
    /// Delete a folder and its whole subtree, cascade-deleting the scenes inside (metadata
    /// rows only; the content-addressed blobs are shared with share-links and untouched)
    /// and the subtree's ACL grants. `NotFound` if the folder does not exist.
    async fn delete_folder(&self, id: FolderId<'_>) -> Result<(), StoreError>;

    /// The effective [`Permission`] `user` has on `folder`, or `None` for no access. The
    /// maximum of: ownership (`owner_user_id == user` ⇒ `Admin`), org membership (any
    /// member of the folder's org ⇒ `Editor`, preserving today's shared-library model),
    /// and the strongest ACL grant on the folder or any ancestor naming the user directly
    /// or via a group they belong to.
    async fn effective_permission(
        &self,
        folder: FolderId<'_>,
        user: UserId<'_>,
    ) -> Result<Option<Permission>, StoreError>;
    /// Upsert an ACL grant on `folder` (the `(principal_kind, principal_id)` pair is the
    /// key — re-granting changes the level). `NotFound` if the folder does not exist.
    async fn set_permission(
        &self,
        folder: FolderId<'_>,
        grant: FolderGrant,
        now: Timestamp,
    ) -> Result<(), StoreError>;
    /// The ACL grants set directly on `folder` (not inherited), ordered by principal.
    /// Unbounded only once a sharing endpoint exposes `set_permission`; see the trait-level
    /// SEC-33 note for the grants-per-folder cap that endpoint must enforce.
    async fn list_permissions(&self, folder: FolderId<'_>) -> Result<Vec<FolderGrant>, StoreError>;
    /// Remove an ACL grant from `folder`; absent grants are a no-op.
    async fn remove_permission(
        &self,
        folder: FolderId<'_>,
        kind: PrincipalKind,
        principal_id: &str,
    ) -> Result<(), StoreError>;
}

/// Organizations and their membership. Scenes are owned by an org (`Scene::owner`).
#[async_trait]
pub trait OrgStore: Send + Sync {
    /// Create the org if it does not exist; existing rows are left untouched.
    async fn ensure_org(&self, org: Org) -> Result<(), StoreError>;
    /// Add a member, idempotently.
    async fn add_member(
        &self,
        org_id: OrgId<'_>,
        user_id: UserId<'_>,
        role: Role,
    ) -> Result<(), StoreError>;
    /// The org the user belongs to (first membership; single-org world today).
    async fn org_for_user(&self, user_id: UserId<'_>) -> Result<Org, StoreError>;
}

/// The unified store the server depends on; future features add their own trait halves
/// here.
pub trait Store:
    DocumentStore + SceneStore + FileStore + UserStore + SessionStore + OrgStore + FolderStore
{
}

impl<
        T: DocumentStore + SceneStore + FileStore + UserStore + SessionStore + OrgStore + FolderStore,
    > Store for T
{
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `as_str`/`FromStr` must stay inverses — they are the only spellings of role
    /// values, shared by the write path today and any future read path.
    #[test]
    fn role_round_trips_through_its_storage_string() {
        for role in [Role::Member, Role::Admin] {
            assert_eq!(role.as_str().parse::<Role>().unwrap(), role);
        }
        for bad in ["", "Member", "memmber", "owner"] {
            assert!(bad.parse::<Role>().is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn principal_kind_round_trips_through_its_storage_string() {
        for kind in [PrincipalKind::User, PrincipalKind::Group] {
            assert_eq!(kind.as_str().parse::<PrincipalKind>().unwrap(), kind);
        }
        for bad in ["", "User", "users", "org"] {
            assert!(bad.parse::<PrincipalKind>().is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn permission_round_trips_and_orders_by_privilege() {
        for perm in [Permission::Viewer, Permission::Editor, Permission::Admin] {
            assert_eq!(perm.as_str().parse::<Permission>().unwrap(), perm);
        }
        for bad in ["", "Viewer", "owner", "write"] {
            assert!(bad.parse::<Permission>().is_err(), "accepted {bad:?}");
        }
        // The Ord derive must express ascending privilege so authorization can compare
        // `effective >= required` directly.
        assert!(Permission::Viewer < Permission::Editor);
        assert!(Permission::Editor < Permission::Admin);
        assert!(Permission::Admin >= Permission::Editor);
    }
}
