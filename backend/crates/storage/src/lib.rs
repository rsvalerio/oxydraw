//! Storage backends for oxydraw.
//!
//! [`select_store`] picks a backend from [`Config::storage_type`]:
//! - `sqlite` — embedded SQLite via sqlx (the default)
//! - `memory` — in-process, volatile (dev/tests)

mod memory;
#[cfg(feature = "sqlite")]
mod sqlite;

use std::sync::Arc;

use oxydraw_core::config::{Config, StorageType};
use oxydraw_core::store::{Store, StoreError};

pub use memory::MemoryStore;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStore;

/// Build the storage backend named by `cfg.storage_type`. Invalid names cannot reach
/// here — [`StorageType`] is a closed enum, rejected at config extraction.
pub async fn select_store(cfg: &Config) -> Result<Arc<dyn Store>, StoreError> {
    match cfg.storage_type {
        StorageType::Sqlite => connect_sqlite(cfg).await,
        StorageType::Memory => Ok(Arc::new(MemoryStore::new())),
    }
}

#[cfg(feature = "sqlite")]
async fn connect_sqlite(cfg: &Config) -> Result<Arc<dyn Store>, StoreError> {
    let path = cfg
        .data_source_name
        .clone()
        .unwrap_or_else(|| "oxydraw.db".to_string());
    // Accept either a bare path or a full `sqlite:` URL; create the file if missing.
    let url = if path.starts_with("sqlite:") {
        path
    } else {
        format!("sqlite://{path}?mode=rwc")
    };
    Ok(Arc::new(SqliteStore::connect(&url).await?))
}

#[cfg(not(feature = "sqlite"))]
async fn connect_sqlite(_cfg: &Config) -> Result<Arc<dyn Store>, StoreError> {
    Err(StoreError::Unimplemented("sqlite"))
}

/// [`select_store`] dispatch and `connect_sqlite` DSN normalization — the path every
/// production startup runs (the store-contract suites construct backends directly and
/// bypass this layer).
#[cfg(test)]
mod tests {
    use oxydraw_core::config::{Config, StorageType};
    use oxydraw_core::model::Document;
    use oxydraw_core::store::Store;

    use super::select_store;

    fn cfg(storage_type: StorageType, data_source_name: Option<String>) -> Config {
        Config {
            storage_type,
            data_source_name,
            ..Config::default()
        }
    }

    /// The selected store must be usable, not merely constructed.
    async fn assert_store_works(store: &dyn Store) {
        let id = store
            .create(Document {
                data: b"probe".to_vec(),
            })
            .await
            .unwrap();
        assert_eq!(store.find_id(&id).await.unwrap().data, b"probe");
    }

    #[tokio::test]
    async fn memory_storage_type_returns_a_working_store() {
        let store = select_store(&cfg(StorageType::Memory, None)).await.unwrap();
        assert_store_works(store.as_ref()).await;
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn sqlite_storage_type_creates_missing_file_from_bare_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fresh.db");
        let store = select_store(&cfg(StorageType::Sqlite, Some(path.display().to_string())))
            .await
            .unwrap();
        assert!(path.is_file(), "bare path creates the database file");
        assert_store_works(store.as_ref()).await;
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn sqlite_prefixed_dsn_is_accepted_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("url.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let store = select_store(&cfg(StorageType::Sqlite, Some(url)))
            .await
            .unwrap();
        assert!(
            path.is_file(),
            "full sqlite: URL connects to the named file"
        );
        assert_store_works(store.as_ref()).await;
    }
}

/// Shared behavioral contract run against any [`Store`] implementation, one helper per
/// store concern, so every backend is held to the same contract and a failure names the
/// concern that regressed.
#[cfg(test)]
pub(crate) mod test_support {
    use oxydraw_core::model::{Document, Folder, Org, Scene, Session, StoredFile, Timestamp, User};
    use oxydraw_core::store::{
        FolderGrant, FolderId, FolderMoveError, IdentityProfile, OrgId, Permission, PrincipalKind,
        Role, Store, StoreError, TokenHash, UserId,
    };

    /// Shorthand for the fixed timestamps these contract tests revolve around.
    fn ts(s: &str) -> Timestamp {
        Timestamp::parse(s).unwrap()
    }

    /// Assert a store call returned `StoreError::NotFound` specifically — not just any
    /// error. The HTTP layer maps NotFound to 404 but other errors to 500, so a backend
    /// returning `Backend` for a missing row (which a bare `.is_err()` would accept) would
    /// silently turn unknown share links into 500s (TEST-11).
    macro_rules! assert_not_found {
        ($expr:expr) => {
            assert!(
                matches!($expr, Err(StoreError::NotFound)),
                "expected StoreError::NotFound"
            )
        };
    }

    /// Generate one `#[tokio::test]` per contract concern. `$fresh` is an async fn
    /// returning `(guard, store)` — the guard keeps backend resources (e.g. the SQLite
    /// temp file) alive for the duration of the test; each test gets a fresh store.
    macro_rules! store_contract_tests {
        ($fresh:ident) => {
            #[tokio::test]
            async fn documents_round_trip_and_count_quota_bytes() {
                let (_guard, store) = $fresh().await;
                crate::test_support::documents(&store).await;
            }

            #[tokio::test]
            async fn scenes_round_trip_and_list_per_owner_newest_first() {
                let (_guard, store) = $fresh().await;
                crate::test_support::scenes(&store).await;
            }

            #[tokio::test]
            async fn files_overwrite_by_path_and_count_quota_bytes() {
                let (_guard, store) = $fresh().await;
                crate::test_support::files(&store).await;
            }

            #[tokio::test]
            async fn users_resolve_identity_to_one_user_and_refresh_profile() {
                let (_guard, store) = $fresh().await;
                crate::test_support::users(&store).await;
            }

            #[tokio::test]
            async fn sessions_round_trip_delete_and_prune_expired() {
                let (_guard, store) = $fresh().await;
                crate::test_support::sessions(&store).await;
            }

            #[tokio::test]
            async fn orgs_ensure_idempotently_and_find_membership() {
                let (_guard, store) = $fresh().await;
                crate::test_support::orgs(&store).await;
            }

            #[tokio::test]
            async fn folders_nest_rename_move_and_reject_cycles() {
                let (_guard, store) = $fresh().await;
                crate::test_support::folders(&store).await;
            }

            #[tokio::test]
            async fn folder_move_is_atomic_under_concurrent_reparents() {
                let (_guard, store) = $fresh().await;
                crate::test_support::folder_move_is_atomic_under_concurrency(&store).await;
            }

            #[tokio::test]
            async fn move_rejects_cross_org_destination() {
                let (_guard, store) = $fresh().await;
                crate::test_support::move_rejects_cross_org_destination(&store).await;
            }

            #[tokio::test]
            async fn folder_delete_cascades_subtree_scenes_and_grants() {
                let (_guard, store) = $fresh().await;
                crate::test_support::folder_delete_cascade(&store).await;
            }

            #[tokio::test]
            async fn folder_permissions_resolve_ownership_membership_and_inheritance() {
                let (_guard, store) = $fresh().await;
                crate::test_support::folder_permissions(&store).await;
            }
        };
    }
    pub(crate) use store_contract_tests;

    /// A user-principal ACL grant with no `granted_by`, for the permission tests.
    fn grant(principal_id: &str, permission: Permission) -> FolderGrant {
        FolderGrant {
            principal_kind: PrincipalKind::User,
            principal_id: principal_id.to_string(),
            permission,
            granted_by: None,
        }
    }

    /// A folder with the common test defaults; override fields as needed.
    fn folder(id: &str, parent_id: Option<&str>, org_id: &str) -> Folder {
        Folder {
            id: id.to_string(),
            name: format!("folder {id}"),
            parent_id: parent_id.map(str::to_string),
            org_id: org_id.to_string(),
            owner_user_id: None,
            created_at: ts("2026-01-01T00:00:00Z"),
            updated_at: ts("2026-01-01T00:00:00Z"),
        }
    }

    /// Documents round-trip, mint distinct ids, and are quota-accounted.
    pub async fn documents(store: &dyn Store) {
        // Quota accounting starts at zero.
        assert_eq!(store.documents_total_bytes().await.unwrap(), 0);

        // Documents round-trip; missing ids are NotFound.
        let id = store
            .create(Document {
                data: b"hello".to_vec(),
            })
            .await
            .unwrap();
        assert_eq!(store.find_id(&id).await.unwrap().data, b"hello");
        assert_not_found!(store.find_id("does-not-exist").await);

        // Each create mints a distinct id; both stay readable.
        let other = store
            .create(Document {
                data: b"world".to_vec(),
            })
            .await
            .unwrap();
        assert_ne!(id, other);
        assert_eq!(store.find_id(&other).await.unwrap().data, b"world");
        assert_eq!(store.find_id(&id).await.unwrap().data, b"hello");

        // Quota accounting tracks document payload bytes ("hello" + "world").
        assert_eq!(store.documents_total_bytes().await.unwrap(), 10);
    }

    /// Scenes round-trip, list per-owner newest first, and delete.
    pub async fn scenes(store: &dyn Store) {
        let scene = |id: &str, owner: &str, updated_at: &str| Scene {
            id: id.to_string(),
            name: format!("scene {id}"),
            document_id: format!("doc-{id}"),
            key: format!("key-{id}"),
            owner: owner.to_string(),
            folder_id: None,
            owner_user_id: None,
            created_at: ts("2026-01-01T00:00:00Z"),
            updated_at: ts(updated_at),
        };
        store
            .create_scene(scene("s1", "alice", "2026-01-01T00:00:00Z"))
            .await
            .unwrap();
        store
            .create_scene(scene("s2", "alice", "2026-02-01T00:00:00Z"))
            .await
            .unwrap();
        store
            .create_scene(scene("s3", "bob", "2026-03-01T00:00:00Z"))
            .await
            .unwrap();

        let found = store.find_scene("s1").await.unwrap();
        assert_eq!(found, scene("s1", "alice", "2026-01-01T00:00:00Z"));
        assert_not_found!(store.find_scene("missing").await);

        let listed = store.list_scenes("alice").await.unwrap();
        assert_eq!(
            listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["s2", "s1"]
        );
        assert_eq!(store.list_scenes("nobody").await.unwrap(), []);

        // Folder-filtered listing: new scenes are at the root (folder_id IS NULL).
        let root = store.list_scenes_in_folder("alice", None).await.unwrap();
        assert_eq!(
            root.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["s2", "s1"]
        );

        // Moving a scene into a folder pulls it out of the root listing and into the
        // folder's, and bumps updated_at (folder need not exist — scenes carry no FK).
        store
            .move_scene(
                "s2",
                OrgId("alice"),
                Some(FolderId("f1")),
                ts("2026-04-01T00:00:00Z"),
            )
            .await
            .unwrap();
        assert_eq!(
            store
                .list_scenes_in_folder("alice", None)
                .await
                .unwrap()
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            ["s1"]
        );
        let in_f1 = store
            .list_scenes_in_folder("alice", Some(FolderId("f1")))
            .await
            .unwrap();
        assert_eq!(
            in_f1.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["s2"]
        );
        assert_eq!(
            store.find_scene("s2").await.unwrap().folder_id.as_deref(),
            Some("f1")
        );

        // Rename changes the name; both mutators are NotFound for unknown ids.
        store
            .rename_scene("s2", "renamed", ts("2026-05-01T00:00:00Z"))
            .await
            .unwrap();
        assert_eq!(store.find_scene("s2").await.unwrap().name, "renamed");
        assert_not_found!(
            store
                .move_scene("missing", OrgId("alice"), None, ts("2026-01-01T00:00:00Z"))
                .await
        );
        assert_not_found!(
            store
                .rename_scene("missing", "x", ts("2026-01-01T00:00:00Z"))
                .await
        );

        // Deleting removes the row; missing ids are NotFound.
        store.delete_scene("s1").await.unwrap();
        assert_not_found!(store.find_scene("s1").await);
        assert_not_found!(store.delete_scene("s1").await);
        assert_eq!(store.list_scenes("alice").await.unwrap().len(), 1);
    }

    /// Files round-trip by path, overwrite in place, and are quota-accounted.
    pub async fn files(store: &dyn Store) {
        // Quota accounting starts at zero.
        assert_eq!(store.files_total_bytes().await.unwrap(), 0);

        // Files round-trip by path; re-putting the same path overwrites.
        let file = StoredFile {
            content_type: "application/octet-stream".to_string(),
            data: b"encrypted".to_vec(),
        };
        store
            .put_file("files/shareLinks/d1/f1", file.clone())
            .await
            .unwrap();
        assert_eq!(
            store.get_file("files/shareLinks/d1/f1").await.unwrap(),
            file
        );
        assert_not_found!(store.get_file("files/shareLinks/d1/nope").await);

        let replacement = StoredFile {
            content_type: "image/png".to_string(),
            data: b"replaced".to_vec(),
        };
        store
            .put_file("files/shareLinks/d1/f1", replacement.clone())
            .await
            .unwrap();
        assert_eq!(
            store.get_file("files/shareLinks/d1/f1").await.unwrap(),
            replacement
        );

        // Quota accounting tracks file payload bytes (the overwrite, "replaced", only).
        assert_eq!(store.files_total_bytes().await.unwrap(), 8);
    }

    /// The profile + first-login user shared by the user/session/org helpers.
    async fn seed_user(store: &dyn Store) -> (IdentityProfile, User) {
        let profile = IdentityProfile {
            provider: "github".to_string(),
            provider_user_id: "12345".to_string(),
            email: Some("alice@example.com".to_string()),
            name: Some("Alice".to_string()),
            avatar_url: None,
        };
        let user = store
            .upsert_user_for_identity(&profile, ts("2026-01-01T00:00:00Z"))
            .await
            .unwrap();
        (profile, user)
    }

    /// First login creates user + identity; re-login resolves to the same user and
    /// refreshes profile fields; a different identity is a new user.
    pub async fn users(store: &dyn Store) {
        let (profile, user) = seed_user(store).await;
        assert_eq!(user.email.as_deref(), Some("alice@example.com"));
        assert_eq!(user.created_at, ts("2026-01-01T00:00:00Z"));

        let renamed = IdentityProfile {
            name: Some("Alice Doe".to_string()),
            ..profile.clone()
        };
        let again = store
            .upsert_user_for_identity(&renamed, ts("2026-02-01T00:00:00Z"))
            .await
            .unwrap();
        assert_eq!(again.id, user.id, "same identity resolves to same user");
        assert_eq!(again.name.as_deref(), Some("Alice Doe"));
        assert_eq!(again.created_at, ts("2026-01-01T00:00:00Z"));

        // A different identity (even with the same email) is a new user.
        let other_provider = IdentityProfile {
            provider: "google".to_string(),
            ..profile.clone()
        };
        let other_user = store
            .upsert_user_for_identity(&other_provider, ts("2026-01-01T00:00:00Z"))
            .await
            .unwrap();
        assert_ne!(other_user.id, user.id);

        assert_eq!(store.find_user(&user.id).await.unwrap().id, user.id);
        assert_not_found!(store.find_user("missing").await);
    }

    /// Sessions round-trip, delete, and expiry pruning.
    pub async fn sessions(store: &dyn Store) {
        let (_, user) = seed_user(store).await;
        store
            .create_session(Session {
                token_hash: "hash-live".to_string(),
                user_id: user.id.clone(),
                expires_at: 2_000,
            })
            .await
            .unwrap();
        store
            .create_session(Session {
                token_hash: "hash-expired".to_string(),
                user_id: user.id.clone(),
                expires_at: 1_000,
            })
            .await
            .unwrap();
        assert_eq!(
            store
                .find_session(TokenHash("hash-live"))
                .await
                .unwrap()
                .user_id,
            user.id
        );
        store.prune_sessions(1_000).await.unwrap();
        assert_not_found!(store.find_session(TokenHash("hash-expired")).await);
        assert!(store.find_session(TokenHash("hash-live")).await.is_ok());
        store.delete_session(TokenHash("hash-live")).await.unwrap();
        assert_not_found!(store.find_session(TokenHash("hash-live")).await);
    }

    /// Org ensure is idempotent (keeps the original name); membership is idempotent;
    /// org_for_user finds the membership.
    pub async fn orgs(store: &dyn Store) {
        let (_, user) = seed_user(store).await;
        let org = Org {
            id: "default".to_string(),
            name: "Default".to_string(),
            created_at: ts("2026-01-01T00:00:00Z"),
        };
        store.ensure_org(org.clone()).await.unwrap();
        store
            .ensure_org(Org {
                name: "Renamed".to_string(),
                ..org.clone()
            })
            .await
            .unwrap();
        assert_not_found!(store.org_for_user(UserId(&user.id)).await);
        store
            .add_member(OrgId("default"), UserId(&user.id), Role::Member)
            .await
            .unwrap();
        store
            .add_member(OrgId("default"), UserId(&user.id), Role::Admin)
            .await
            .unwrap(); // no-op
        let found = store.org_for_user(UserId(&user.id)).await.unwrap();
        assert_eq!(found.id, "default");
        assert_eq!(found.name, "Default", "ensure_org keeps the original row");
    }

    /// Folders nest under a seeded root, list per-parent, rename and move — and a move
    /// that would make a folder its own ancestor (or itself) is rejected as a cycle.
    pub async fn folders(store: &dyn Store) {
        let root = store
            .ensure_root_folder(OrgId("o"), ts("2026-01-01T00:00:00Z"))
            .await
            .unwrap();
        assert_eq!(root, "root:o");
        // ensure_root_folder is idempotent.
        assert_eq!(
            store
                .ensure_root_folder(OrgId("o"), ts("2026-02-01T00:00:00Z"))
                .await
                .unwrap(),
            "root:o"
        );

        store
            .create_folder(folder("a", Some(&root), "o"))
            .await
            .unwrap();
        store
            .create_folder(folder("b", Some("a"), "o"))
            .await
            .unwrap();

        // Listing is per-parent: root is the only top-level folder; `a` is its child.
        let roots = store.list_folders(OrgId("o"), None).await.unwrap();
        assert_eq!(
            roots.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            ["root:o"]
        );
        let under_root = store
            .list_folders(OrgId("o"), Some(FolderId(&root)))
            .await
            .unwrap();
        assert_eq!(
            under_root.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            ["a"]
        );
        assert_eq!(store.count_folders(OrgId("o")).await.unwrap(), 3);
        assert_not_found!(store.find_folder(FolderId("missing")).await);

        // Rename.
        store
            .rename_folder(FolderId("a"), "renamed", ts("2026-03-01T00:00:00Z"))
            .await
            .unwrap();
        assert_eq!(
            store.find_folder(FolderId("a")).await.unwrap().name,
            "renamed"
        );

        // Creating under a missing parent is NotFound.
        assert!(matches!(
            store.create_folder(folder("x", Some("missing"), "o")).await,
            Err(FolderMoveError::NotFound)
        ));

        // Cycle rejection: a folder cannot move under itself or under a descendant.
        assert!(matches!(
            store
                .move_folder(
                    FolderId("a"),
                    OrgId("o"),
                    Some(FolderId("a")),
                    ts("2026-03-01T00:00:00Z")
                )
                .await,
            Err(FolderMoveError::Cycle)
        ));
        assert!(matches!(
            store
                .move_folder(
                    FolderId("a"),
                    OrgId("o"),
                    Some(FolderId("b")),
                    ts("2026-03-01T00:00:00Z")
                )
                .await,
            Err(FolderMoveError::Cycle)
        ));
        assert!(matches!(
            store
                .move_folder(
                    FolderId("missing"),
                    OrgId("o"),
                    None,
                    ts("2026-03-01T00:00:00Z")
                )
                .await,
            Err(FolderMoveError::NotFound)
        ));

        // A valid move: reparent `b` directly under root.
        store
            .move_folder(
                FolderId("b"),
                OrgId("o"),
                Some(FolderId(&root)),
                ts("2026-03-01T00:00:00Z"),
            )
            .await
            .unwrap();
        let under_root = store
            .list_folders(OrgId("o"), Some(FolderId(&root)))
            .await
            .unwrap();
        let mut ids = under_root.iter().map(|f| f.id.as_str()).collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, ["a", "b"]);
    }

    /// CONC-2: concurrent reparents cannot interleave into a cycle. Two individually-acyclic
    /// moves (a→under→b and b→under→a) must serialize so at most one lands — never both,
    /// which would corrupt the tree into an a⇄b cycle.
    pub async fn folder_move_is_atomic_under_concurrency(store: &dyn Store) {
        let now = || ts("2026-07-01T00:00:00Z");
        store.ensure_root_folder(OrgId("o"), now()).await.unwrap();
        store
            .create_folder(folder("a", Some("root:o"), "o"))
            .await
            .unwrap();
        store
            .create_folder(folder("b", Some("root:o"), "o"))
            .await
            .unwrap();

        let (ra, rb) = tokio::join!(
            store.move_folder(FolderId("a"), OrgId("o"), Some(FolderId("b")), now()),
            store.move_folder(FolderId("b"), OrgId("o"), Some(FolderId("a")), now()),
        );
        let oks = [ra.is_ok(), rb.is_ok()]
            .into_iter()
            .filter(|ok| *ok)
            .count();
        assert!(
            oks <= 1,
            "both concurrent reparents succeeded — tree driven into a cycle"
        );

        // Whichever landed, the tree is still acyclic: a and b cannot be each other's parent.
        let a_parent = store.find_folder(FolderId("a")).await.unwrap().parent_id;
        let b_parent = store.find_folder(FolderId("b")).await.unwrap().parent_id;
        assert!(
            !(a_parent.as_deref() == Some("b") && b_parent.as_deref() == Some("a")),
            "a and b form a parent cycle"
        );
    }

    /// SEC-20: the store enforces the org (tenant) boundary on both move paths — a folder or
    /// scene cannot be relocated into another org's folder even via the store API directly,
    /// independent of any handler-level check.
    pub async fn move_rejects_cross_org_destination(store: &dyn Store) {
        let now = || ts("2026-07-01T00:00:00Z");
        store
            .ensure_root_folder(OrgId("org-a"), now())
            .await
            .unwrap();
        store
            .ensure_root_folder(OrgId("org-b"), now())
            .await
            .unwrap();
        store
            .create_folder(folder("fa", Some("root:org-a"), "org-a"))
            .await
            .unwrap();
        store
            .create_folder(folder("fb", Some("root:org-b"), "org-b"))
            .await
            .unwrap();

        // Reparenting org-a's folder under org-b's folder is rejected — even though the
        // destination exists — as NotFound (no cross-tenant existence disclosure).
        assert!(matches!(
            store
                .move_folder(FolderId("fa"), OrgId("org-a"), Some(FolderId("fb")), now())
                .await,
            Err(FolderMoveError::NotFound)
        ));
        assert_eq!(
            store
                .find_folder(FolderId("fa"))
                .await
                .unwrap()
                .parent_id
                .as_deref(),
            Some("root:org-a"),
            "rejected move must not have reparented fa"
        );

        // A scene owned by org-a cannot be moved into org-b's folder, and another org cannot
        // move org-a's scene at all (the owner predicate on the write).
        store
            .create_scene(Scene {
                id: "sa".to_string(),
                name: "s".to_string(),
                document_id: "d".to_string(),
                key: "k".to_string(),
                owner: "org-a".to_string(),
                folder_id: None,
                owner_user_id: None,
                created_at: now(),
                updated_at: now(),
            })
            .await
            .unwrap();
        assert_not_found!(
            store
                .move_scene("sa", OrgId("org-a"), Some(FolderId("fb")), now())
                .await
        );
        assert!(
            store.find_scene("sa").await.unwrap().folder_id.is_none(),
            "rejected scene move must not have changed folder_id"
        );
        assert_not_found!(store.move_scene("sa", OrgId("org-b"), None, now()).await);
    }

    /// Deleting a folder removes its whole subtree — descendant folders, their scenes, and
    /// their ACL grants — while leaving sibling/root content intact.
    pub async fn folder_delete_cascade(store: &dyn Store) {
        let scene_in = |id: &str, folder_id: Option<&str>| Scene {
            id: id.to_string(),
            name: format!("scene {id}"),
            document_id: format!("doc-{id}"),
            key: format!("key-{id}"),
            owner: "o".to_string(),
            folder_id: folder_id.map(str::to_string),
            owner_user_id: None,
            created_at: ts("2026-01-01T00:00:00Z"),
            updated_at: ts("2026-01-01T00:00:00Z"),
        };
        let root = store
            .ensure_root_folder(OrgId("o"), ts("2026-01-01T00:00:00Z"))
            .await
            .unwrap();
        store
            .create_folder(folder("a", Some(&root), "o"))
            .await
            .unwrap();
        store
            .create_folder(folder("b", Some("a"), "o"))
            .await
            .unwrap();
        store.create_scene(scene_in("sa", Some("a"))).await.unwrap();
        store.create_scene(scene_in("sb", Some("b"))).await.unwrap();
        store.create_scene(scene_in("sr", None)).await.unwrap();
        store
            .set_permission(
                FolderId("b"),
                grant("x", Permission::Viewer),
                ts("2026-01-01T00:00:00Z"),
            )
            .await
            .unwrap();

        store.delete_folder(FolderId("a")).await.unwrap();

        // Subtree folders and their scenes are gone; the root folder and root scene remain.
        assert_not_found!(store.find_folder(FolderId("a")).await);
        assert_not_found!(store.find_folder(FolderId("b")).await);
        assert!(store.find_folder(FolderId(&root)).await.is_ok());
        assert_not_found!(store.find_scene("sa").await);
        assert_not_found!(store.find_scene("sb").await);
        assert!(store.find_scene("sr").await.is_ok());
        // The deleted subtree's ACL grants are gone too.
        assert_eq!(store.list_permissions(FolderId("b")).await.unwrap(), []);

        assert_not_found!(store.delete_folder(FolderId("missing")).await);
    }

    /// Effective permission is the max of ownership (Admin), org membership (Editor), and
    /// the strongest ACL grant on the folder or an ancestor; grants inherit downward and
    /// `set_permission` on a missing folder is NotFound.
    pub async fn folder_permissions(store: &dyn Store) {
        let root = store
            .ensure_root_folder(OrgId("org"), ts("2026-01-01T00:00:00Z"))
            .await
            .unwrap();
        let mut owned = folder("f", Some(&root), "org");
        owned.owner_user_id = Some("alice".to_string());
        store.create_folder(owned).await.unwrap();
        store
            .create_folder(folder("c", Some("f"), "org"))
            .await
            .unwrap();

        // Ownership ⇒ Admin; an unrelated user ⇒ no access.
        assert_eq!(
            store
                .effective_permission(FolderId("f"), UserId("alice"))
                .await
                .unwrap(),
            Some(Permission::Admin)
        );
        assert_eq!(
            store
                .effective_permission(FolderId("f"), UserId("bob"))
                .await
                .unwrap(),
            None
        );

        // Org membership ⇒ Editor. The member must be a real user row (FK), so mint one.
        let (_, member) = seed_user(store).await;
        store
            .ensure_org(Org {
                id: "org".to_string(),
                name: "Org".to_string(),
                created_at: ts("2026-01-01T00:00:00Z"),
            })
            .await
            .unwrap();
        store
            .add_member(OrgId("org"), UserId(&member.id), Role::Member)
            .await
            .unwrap();
        assert_eq!(
            store
                .effective_permission(FolderId("f"), UserId(&member.id))
                .await
                .unwrap(),
            Some(Permission::Editor)
        );

        // A direct ACL grant confers its level regardless of membership.
        store
            .set_permission(
                FolderId("f"),
                grant("carol", Permission::Admin),
                ts("2026-01-01T00:00:00Z"),
            )
            .await
            .unwrap();
        assert_eq!(
            store
                .effective_permission(FolderId("f"), UserId("carol"))
                .await
                .unwrap(),
            Some(Permission::Admin)
        );

        // Grants inherit to descendants: a grant on `f` applies to its child `c`.
        store
            .set_permission(
                FolderId("f"),
                grant("dave", Permission::Viewer),
                ts("2026-01-01T00:00:00Z"),
            )
            .await
            .unwrap();
        assert_eq!(
            store
                .effective_permission(FolderId("c"), UserId("dave"))
                .await
                .unwrap(),
            Some(Permission::Viewer)
        );

        // set_permission on a missing folder is NotFound.
        assert_not_found!(
            store
                .set_permission(
                    FolderId("missing"),
                    grant("x", Permission::Viewer),
                    ts("2026-01-01T00:00:00Z"),
                )
                .await
        );

        // list_permissions returns the directly-set grants; remove drops one.
        let grants = store.list_permissions(FolderId("f")).await.unwrap();
        assert_eq!(
            grants
                .iter()
                .map(|g| (g.principal_id.as_str(), g.permission))
                .collect::<Vec<_>>(),
            [("carol", Permission::Admin), ("dave", Permission::Viewer)]
        );
        store
            .remove_permission(FolderId("f"), PrincipalKind::User, "carol")
            .await
            .unwrap();
        assert_eq!(
            store.list_permissions(FolderId("f")).await.unwrap().len(),
            1
        );
        assert_eq!(
            store
                .effective_permission(FolderId("f"), UserId("carol"))
                .await
                .unwrap(),
            None
        );
    }
}
