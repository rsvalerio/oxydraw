//! Store outages must not be masked as a benign status: read handlers that fall through
//! to the durable store return 500 (and log) when the backend errors — not 404 (a true
//! not-found is covered by `share.rs` / `storage.rs`) — and the auth path returns 500,
//! not a spurious 401 that would make every signed-in user look logged out (ERR-6).

use std::sync::Arc;

use async_trait::async_trait;
use oxydraw_core::model::{Document, Folder, Org, Scene, Session, StoredFile, Timestamp, User};
use oxydraw_core::store::{
    DocumentStore, FileStore, FolderGrant, FolderId, FolderMoveError, FolderStore, IdentityProfile,
    OrgId, OrgStore, Permission, PrincipalKind, Role, SceneStore, SessionStore, StoreError,
    TokenHash, UserId, UserStore,
};

mod common;

/// A store whose every operation fails with a backend error, simulating an outage
/// (connection drop, timeout, I/O failure).
struct BrokenStore;

fn outage() -> StoreError {
    StoreError::Backend("induced outage".into())
}

#[async_trait]
impl DocumentStore for BrokenStore {
    async fn find_id(&self, _id: &str) -> Result<Document, StoreError> {
        Err(outage())
    }
    async fn create(&self, _document: Document) -> Result<String, StoreError> {
        Err(outage())
    }
    async fn documents_total_bytes(&self) -> Result<u64, StoreError> {
        Err(outage())
    }
}

#[async_trait]
impl SceneStore for BrokenStore {
    async fn list_scenes(&self, _owner: &str) -> Result<Vec<Scene>, StoreError> {
        Err(outage())
    }
    async fn list_scenes_in_folder(
        &self,
        _owner: &str,
        _folder: Option<FolderId<'_>>,
    ) -> Result<Vec<Scene>, StoreError> {
        Err(outage())
    }
    async fn count_scenes(&self, _owner: &str) -> Result<u64, StoreError> {
        Err(outage())
    }
    async fn create_scene(&self, _scene: Scene) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn find_scene(&self, _id: &str) -> Result<Scene, StoreError> {
        Err(outage())
    }
    async fn move_scene(
        &self,
        _id: &str,
        _owner: OrgId<'_>,
        _folder: Option<FolderId<'_>>,
        _now: Timestamp,
    ) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn rename_scene(
        &self,
        _id: &str,
        _name: &str,
        _now: Timestamp,
    ) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn delete_scene(&self, _id: &str) -> Result<(), StoreError> {
        Err(outage())
    }
}

#[async_trait]
impl FolderStore for BrokenStore {
    async fn ensure_root_folder(
        &self,
        _org_id: OrgId<'_>,
        _now: Timestamp,
    ) -> Result<String, StoreError> {
        Err(outage())
    }
    async fn find_folder(&self, _id: FolderId<'_>) -> Result<Folder, StoreError> {
        Err(outage())
    }
    async fn list_folders(
        &self,
        _org_id: OrgId<'_>,
        _parent: Option<FolderId<'_>>,
    ) -> Result<Vec<Folder>, StoreError> {
        Err(outage())
    }
    async fn count_folders(&self, _org_id: OrgId<'_>) -> Result<u64, StoreError> {
        Err(outage())
    }
    async fn create_folder(&self, _folder: Folder) -> Result<(), FolderMoveError> {
        Err(FolderMoveError::Backend("induced outage".into()))
    }
    async fn rename_folder(
        &self,
        _id: FolderId<'_>,
        _name: &str,
        _now: Timestamp,
    ) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn move_folder(
        &self,
        _id: FolderId<'_>,
        _org: OrgId<'_>,
        _new_parent: Option<FolderId<'_>>,
        _now: Timestamp,
    ) -> Result<(), FolderMoveError> {
        Err(FolderMoveError::Backend("induced outage".into()))
    }
    async fn delete_folder(&self, _id: FolderId<'_>) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn effective_permission(
        &self,
        _folder: FolderId<'_>,
        _user: UserId<'_>,
    ) -> Result<Option<Permission>, StoreError> {
        Err(outage())
    }
    async fn set_permission(
        &self,
        _folder: FolderId<'_>,
        _grant: FolderGrant,
        _now: Timestamp,
    ) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn list_permissions(
        &self,
        _folder: FolderId<'_>,
    ) -> Result<Vec<FolderGrant>, StoreError> {
        Err(outage())
    }
    async fn remove_permission(
        &self,
        _folder: FolderId<'_>,
        _kind: PrincipalKind,
        _principal_id: &str,
    ) -> Result<(), StoreError> {
        Err(outage())
    }
}

#[async_trait]
impl FileStore for BrokenStore {
    async fn put_file(&self, _path: &str, _file: StoredFile) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn get_file(&self, _path: &str) -> Result<StoredFile, StoreError> {
        Err(outage())
    }
    async fn files_total_bytes(&self) -> Result<u64, StoreError> {
        Err(outage())
    }
    async fn count_files(&self) -> Result<u64, StoreError> {
        Err(outage())
    }
}

#[async_trait]
impl UserStore for BrokenStore {
    async fn upsert_user_for_identity(
        &self,
        _profile: &IdentityProfile,
        _now: Timestamp,
    ) -> Result<User, StoreError> {
        Err(outage())
    }
    async fn find_user(&self, _id: &str) -> Result<User, StoreError> {
        Err(outage())
    }
}

#[async_trait]
impl SessionStore for BrokenStore {
    async fn create_session(&self, _session: Session) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn find_session(&self, _token_hash: TokenHash<'_>) -> Result<Session, StoreError> {
        Err(outage())
    }
    async fn delete_session(&self, _token_hash: TokenHash<'_>) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn prune_sessions(&self, _now: i64) -> Result<(), StoreError> {
        Err(outage())
    }
}

#[async_trait]
impl OrgStore for BrokenStore {
    async fn ensure_org(&self, _org: Org) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn add_member(
        &self,
        _org_id: OrgId<'_>,
        _user_id: UserId<'_>,
        _role: Role,
    ) -> Result<(), StoreError> {
        Err(outage())
    }
    async fn org_for_user(&self, _user_id: UserId<'_>) -> Result<Org, StoreError> {
        Err(outage())
    }
}

async fn spawn_broken() -> std::net::SocketAddr {
    common::spawn_app_with_store(common::test_config(), Arc::new(BrokenStore)).await
}

/// The password configured by [`spawn_broken_with_password`].
const BROKEN_PASSWORD: &str = "broken-store-password";

/// Like [`spawn_broken`], but with `ext_password` set so the `/api/ext/*` session gate
/// is active — the auth-path outage contracts need a guarded deployment.
async fn spawn_broken_with_password() -> std::net::SocketAddr {
    let config = oxydraw_core::config::Config {
        ext_password: Some(BROKEN_PASSWORD.into()),
        ..common::test_config()
    };
    common::spawn_app_with_store(config, Arc::new(BrokenStore)).await
}

#[tokio::test]
async fn document_read_returns_500_on_store_error() {
    let addr = spawn_broken().await;
    let r = reqwest::get(format!("http://{addr}/api/v2/some-id"))
        .await
        .unwrap();
    assert_eq!(
        r.status(),
        500,
        "a store outage must surface as 500, not 404"
    );
}

/// `require_session` must report a session-store outage as 500, not the spurious 401
/// that would make every signed-in user look logged out (auth.rs `Err` arm, the
/// TASK-0036 / ERR-6 fix).
#[tokio::test]
async fn guarded_route_returns_500_not_401_on_session_store_error() {
    let addr = spawn_broken_with_password().await;
    let r = common::client()
        .get(format!("http://{addr}/api/ext/scenes"))
        .header("cookie", "ext_session=any-token")
        .send()
        .await
        .unwrap();
    assert_eq!(
        r.status(),
        500,
        "a session-store outage must surface as 500, not 401"
    );
}

/// A correct password during a store outage must fail with 500 — `establish_session`
/// cannot upsert the user or mint a session — never a 401 blaming the credentials.
#[tokio::test]
async fn login_returns_500_on_store_error() {
    let addr = spawn_broken_with_password().await;
    let r = common::client()
        .post(format!("http://{addr}/api/ext/login"))
        .json(&serde_json::json!({ "password": BROKEN_PASSWORD }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        r.status(),
        500,
        "a store outage during login must surface as 500, not unauthorized"
    );
}

/// The scene-create quota count falls through to the store; an outage there is a 500,
/// not a quota or validation failure. Open mode (no password) on purpose: the anonymous
/// default-org principal reaches the handler without a session lookup, so this pins the
/// handler's own error arm rather than the middleware's.
#[tokio::test]
async fn create_scene_returns_500_on_store_error() {
    let addr = spawn_broken().await;
    let r = common::client()
        .post(format!("http://{addr}/api/ext/scenes"))
        .json(&serde_json::json!({
            "name": "scene",
            "document_id": "doc-1",
            "key": "key-1",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        r.status(),
        500,
        "a store outage during the scene quota count must surface as 500"
    );
}

#[tokio::test]
async fn file_download_returns_500_on_store_error() {
    let addr = spawn_broken().await;
    // `GET /api/files/{id}` reads from the store, which is down — that is an outage, not a
    // missing object.
    let r = reqwest::get(format!("http://{addr}/api/files/f1"))
        .await
        .unwrap();
    assert_eq!(
        r.status(),
        500,
        "a store outage must surface as 500, not 404"
    );
}
