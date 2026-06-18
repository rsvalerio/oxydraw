//! Test-only fixtures shared by the `folders`/`scenes` access-helper unit tests: a
//! [`CurrentUser`](super::CurrentUser) constructor and a [`FaultyStore`] whose every method
//! fails with [`StoreError::Backend`], so the access helpers' "backend error → 500" arm can
//! be exercised ([`MemoryStore`](oxydraw_storage::MemoryStore) only ever yields `NotFound`).

use async_trait::async_trait;
use oxydraw_core::model::{Document, Folder, Org, Scene, Session, StoredFile, Timestamp, User};
use oxydraw_core::store::{
    DocumentStore, FileStore, FolderGrant, FolderId, FolderMoveError, FolderStore, IdentityProfile,
    OrgId, OrgStore, Permission, PrincipalKind, Role, SceneStore, SessionStore, StoreError,
    TokenHash, UserId, UserStore,
};

use super::CurrentUser;

/// A [`CurrentUser`] in `org` (anonymous, open-mode principal — `user_id: None`), enough for
/// the org-boundary access checks under test.
pub(crate) fn current_user(org: &str) -> CurrentUser {
    CurrentUser {
        user_id: None,
        org_id: org.to_string(),
        org_name: "Test Org".to_string(),
    }
}

fn backend_err() -> StoreError {
    StoreError::Backend(Box::new(std::io::Error::other("injected backend failure")))
}

fn folder_move_backend_err() -> FolderMoveError {
    FolderMoveError::Backend(Box::new(std::io::Error::other("injected backend failure")))
}

/// A [`Store`](oxydraw_core::store::Store) whose every operation fails with a backend error.
/// The access helpers only call `find_folder`/`find_scene`; the rest exist solely to satisfy
/// the trait and are never invoked.
pub(crate) struct FaultyStore;

#[async_trait]
impl DocumentStore for FaultyStore {
    async fn find_id(&self, _id: &str) -> Result<Document, StoreError> {
        Err(backend_err())
    }
    async fn create(&self, _document: Document) -> Result<String, StoreError> {
        Err(backend_err())
    }
    async fn documents_total_bytes(&self) -> Result<u64, StoreError> {
        Err(backend_err())
    }
}

#[async_trait]
impl SceneStore for FaultyStore {
    async fn list_scenes(&self, _owner: &str) -> Result<Vec<Scene>, StoreError> {
        Err(backend_err())
    }
    async fn list_scenes_in_folder(
        &self,
        _owner: &str,
        _folder: Option<FolderId<'_>>,
    ) -> Result<Vec<Scene>, StoreError> {
        Err(backend_err())
    }
    async fn count_scenes(&self, _owner: &str) -> Result<u64, StoreError> {
        Err(backend_err())
    }
    async fn create_scene(&self, _scene: Scene) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn find_scene(&self, _id: &str) -> Result<Scene, StoreError> {
        Err(backend_err())
    }
    async fn move_scene(
        &self,
        _id: &str,
        _owner: OrgId<'_>,
        _folder: Option<FolderId<'_>>,
        _now: Timestamp,
    ) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn rename_scene(
        &self,
        _id: &str,
        _name: &str,
        _now: Timestamp,
    ) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn delete_scene(&self, _id: &str) -> Result<(), StoreError> {
        Err(backend_err())
    }
}

#[async_trait]
impl FileStore for FaultyStore {
    async fn put_file(&self, _path: &str, _file: StoredFile) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn get_file(&self, _path: &str) -> Result<StoredFile, StoreError> {
        Err(backend_err())
    }
    async fn files_total_bytes(&self) -> Result<u64, StoreError> {
        Err(backend_err())
    }
    async fn count_files(&self) -> Result<u64, StoreError> {
        Err(backend_err())
    }
}

#[async_trait]
impl UserStore for FaultyStore {
    async fn upsert_user_for_identity(
        &self,
        _profile: &IdentityProfile,
        _now: Timestamp,
    ) -> Result<User, StoreError> {
        Err(backend_err())
    }
    async fn find_user(&self, _id: &str) -> Result<User, StoreError> {
        Err(backend_err())
    }
}

#[async_trait]
impl SessionStore for FaultyStore {
    async fn create_session(&self, _session: Session) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn find_session(&self, _token_hash: TokenHash<'_>) -> Result<Session, StoreError> {
        Err(backend_err())
    }
    async fn delete_session(&self, _token_hash: TokenHash<'_>) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn prune_sessions(&self, _now: i64) -> Result<(), StoreError> {
        Err(backend_err())
    }
}

#[async_trait]
impl FolderStore for FaultyStore {
    async fn ensure_root_folder(
        &self,
        _org_id: OrgId<'_>,
        _now: Timestamp,
    ) -> Result<String, StoreError> {
        Err(backend_err())
    }
    async fn find_folder(&self, _id: FolderId<'_>) -> Result<Folder, StoreError> {
        Err(backend_err())
    }
    async fn list_folders(
        &self,
        _org_id: OrgId<'_>,
        _parent: Option<FolderId<'_>>,
    ) -> Result<Vec<Folder>, StoreError> {
        Err(backend_err())
    }
    async fn count_folders(&self, _org_id: OrgId<'_>) -> Result<u64, StoreError> {
        Err(backend_err())
    }
    async fn create_folder(&self, _folder: Folder) -> Result<(), FolderMoveError> {
        Err(folder_move_backend_err())
    }
    async fn rename_folder(
        &self,
        _id: FolderId<'_>,
        _name: &str,
        _now: Timestamp,
    ) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn move_folder(
        &self,
        _id: FolderId<'_>,
        _org: OrgId<'_>,
        _new_parent: Option<FolderId<'_>>,
        _now: Timestamp,
    ) -> Result<(), FolderMoveError> {
        Err(folder_move_backend_err())
    }
    async fn delete_folder(&self, _id: FolderId<'_>) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn effective_permission(
        &self,
        _folder: FolderId<'_>,
        _user: UserId<'_>,
    ) -> Result<Option<Permission>, StoreError> {
        Err(backend_err())
    }
    async fn set_permission(
        &self,
        _folder: FolderId<'_>,
        _grant: FolderGrant,
        _now: Timestamp,
    ) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn list_permissions(
        &self,
        _folder: FolderId<'_>,
    ) -> Result<Vec<FolderGrant>, StoreError> {
        Err(backend_err())
    }
    async fn remove_permission(
        &self,
        _folder: FolderId<'_>,
        _kind: PrincipalKind,
        _principal_id: &str,
    ) -> Result<(), StoreError> {
        Err(backend_err())
    }
}

#[async_trait]
impl OrgStore for FaultyStore {
    async fn ensure_org(&self, _org: Org) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn add_member(
        &self,
        _org_id: OrgId<'_>,
        _user_id: UserId<'_>,
        _role: Role,
    ) -> Result<(), StoreError> {
        Err(backend_err())
    }
    async fn org_for_user(&self, _user_id: UserId<'_>) -> Result<Org, StoreError> {
        Err(backend_err())
    }
}
