//! Embedded SQLite backend (sqlx). Scene data is stored as BLOB; placeholders are `?`.

use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use oxydraw_core::model::{Document, Folder, Org, Scene, Session, StoredFile, Timestamp, User};
use oxydraw_core::store::{
    DocumentStore, FileStore, FolderGrant, FolderId, FolderMoveError, FolderStore, IdentityProfile,
    OrgId, OrgStore, Permission, PrincipalKind, Role, SceneStore, SessionStore, StoreError,
    TokenHash, UserId, UserStore, MAX_FOLDER_DEPTH,
};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow, SqliteSynchronous,
};
use sqlx::{Row, SqlitePool};

/// How long a request may wait for a pooled connection before erroring. Explicit and
/// short on purpose: with an embedded single-file database, waiting longer than this
/// means the pool is wedged, and failing fast beats stacking blocked request tasks
/// behind sqlx's implicit 30s default.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// Upper bound on waiting for SQLite's file lock (`busy_timeout`) — the realistic stall
/// mode for an embedded database (there is no network hop to time out on), so a writer
/// holding the lock cannot pin other connections indefinitely.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        // WAL keeps readers concurrent with the (single) writer — the default rollback
        // journal takes an exclusive file lock for every write+fsync, turning write load
        // into 5s BUSY_TIMEOUT stalls for all readers. `synchronous(Normal)` is the
        // standard WAL pairing: it fsyncs on checkpoint rather than every commit, which
        // in WAL mode still survives an application crash (a power loss can lose the
        // last commits, never corrupt the database).
        let options = SqliteConnectOptions::from_str(url)?
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(BUSY_TIMEOUT);
        let db_path = options.get_filename().to_path_buf();
        // SEC-29/SEC-25: the database holds user emails, identity mappings, and
        // session-token hashes. Letting SQLite create the file (world-readable under a
        // typical umask) and chmodding afterwards leaves a window in which a local
        // process can open — and keep — a readable fd that survives the chmod.
        // Pre-create the file with owner-only permissions instead, so it is never
        // group/other-readable; the -wal/-shm side files inherit the main file's mode.
        // A database that already exists (possibly created by an older version with
        // looser permissions) is tightened in place before any new data is written.
        // `:memory:` databases have no file to protect.
        //
        // CONC-5: the syscalls below are blocking std::fs rather than tokio::fs, but
        // `connect` runs exactly once at startup before the server binds a listener, so
        // blocking the worker here cannot stall request handling — not worth pulling in
        // tokio's `fs` feature for two startup metadata ops.
        #[cfg(unix)]
        if !db_path.as_os_str().is_empty() && db_path != std::path::Path::new(":memory:") {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .open(&db_path)
                .map_err(|e| StoreError::Backend(Box::new(e)))?;
            std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| StoreError::Backend(Box::new(e)))?;
        }
        #[cfg(not(unix))]
        let _ = db_path;
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(ACQUIRE_TIMEOUT)
            .connect_with(options)
            .await?;
        let store = Self { pool };
        store.init().await?;
        Ok(store)
    }

    async fn init(&self) -> Result<(), StoreError> {
        // FN-1: a flat, single-abstraction-level sequence of idempotent DDL, so it reads
        // (and grows) as a data list rather than ~10 lines of `sqlx::query(...).execute`
        // boilerplate per table. Executed in order; later statements may reference tables
        // created by earlier ones.
        // Migrate the pre-existing `scenes` table *before* the schema runs: one of the
        // `CREATE INDEX` statements below references `folder_id`, so the column must exist
        // first. On a fresh database `scenes` does not exist yet, the probe is a no-op, and
        // the `CREATE TABLE scenes` body in SCHEMA supplies the columns instead.
        self.migrate_scene_columns().await?;
        for stmt in SCHEMA {
            sqlx::query(stmt).execute(&self.pool).await?;
        }
        Ok(())
    }

    /// Add the folder columns to a pre-existing `scenes` table. `CREATE TABLE IF NOT
    /// EXISTS` never alters an existing table (see the [`SCHEMA`] doc comment), so a
    /// database created before folders existed keeps the old 7-column `scenes` definition
    /// — the new columns must be added with `ALTER TABLE`. This is the project's first
    /// schema *migration* (as opposed to additive `CREATE … IF NOT EXISTS`): it is
    /// deliberately minimal — two nullable columns, no default, no row rewrite (SQLite
    /// `ADD COLUMN` is a metadata-only operation) — and idempotent, guarded by a
    /// `PRAGMA table_info` probe so a fresh database (which already has the columns from
    /// the updated `CREATE TABLE scenes` body) and a re-run are both no-ops.
    async fn migrate_scene_columns(&self) -> Result<(), StoreError> {
        let existing: Vec<String> = sqlx::query("PRAGMA table_info(scenes)")
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|r| r.try_get::<String, _>("name"))
            .collect::<Result<_, _>>()?;
        // A fresh database has no `scenes` table yet (empty probe) — nothing to alter; the
        // SCHEMA `CREATE TABLE scenes` creates it with the columns already present.
        if existing.is_empty() {
            return Ok(());
        }
        for column in ["folder_id", "owner_user_id"] {
            if !existing.iter().any(|c| c == column) {
                sqlx::query(&format!("ALTER TABLE scenes ADD COLUMN {column} TEXT"))
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }
}

/// The database schema, as the ordered list of `CREATE TABLE`/`CREATE INDEX` statements
/// [`SqliteStore::init`] applies at startup.
///
/// Referential integrity (sqlx enables the `foreign_keys` pragma by default): every
/// inter-table relationship declares `REFERENCES` except the two below, which are
/// intentionally unenforced:
/// - `scenes.document_id` → `documents(id)`: the store contract treats it as an opaque
///   key — scenes may reference documents that were never persisted through this store
///   (and `MemoryStore` enforces no constraints), so declaring it here would make the
///   SQLite backend stricter than the contract its callers and tests rely on.
/// - `scenes.owner` → `orgs(id)`: `DEFAULT 'default'` predates any org row — scenes can
///   be created before the default org is ensured.
/// - `scenes.folder_id` → `folders(id)`: not declared so the column can be `ALTER`-added
///   to a pre-existing `scenes` table ([`SqliteStore::migrate_scene_columns`]); cascade
///   on folder delete is handled explicitly in [`SqliteStore::delete_folder`] instead.
///
/// `CREATE TABLE IF NOT EXISTS` never alters an existing table, so a `REFERENCES`
/// clause (or a new column) added here applies to freshly created databases only;
/// existing deployments keep their original table definitions unless rebuilt. The one
/// column addition folders need on the pre-existing `scenes` table is therefore applied
/// out of band by [`SqliteStore::migrate_scene_columns`].
const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS documents (id TEXT PRIMARY KEY, data BLOB NOT NULL)",
    "CREATE TABLE IF NOT EXISTS scenes (
        id            TEXT PRIMARY KEY,
        name          TEXT NOT NULL,
        document_id   TEXT NOT NULL,
        key           TEXT NOT NULL,
        owner         TEXT NOT NULL DEFAULT 'default',
        folder_id     TEXT,
        owner_user_id TEXT,
        created_at    TEXT NOT NULL,
        updated_at    TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS files (
        path         TEXT PRIMARY KEY,
        content_type TEXT NOT NULL,
        data         BLOB NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS users (
        id         TEXT PRIMARY KEY,
        email      TEXT,
        name       TEXT,
        avatar_url TEXT,
        created_at TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS identities (
        provider         TEXT NOT NULL,
        provider_user_id TEXT NOT NULL,
        user_id          TEXT NOT NULL REFERENCES users(id),
        email            TEXT,
        PRIMARY KEY (provider, provider_user_id)
    )",
    "CREATE TABLE IF NOT EXISTS sessions (
        token_hash TEXT PRIMARY KEY,
        user_id    TEXT NOT NULL REFERENCES users(id),
        expires_at INTEGER NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS orgs (
        id         TEXT PRIMARY KEY,
        name       TEXT NOT NULL,
        created_at TEXT NOT NULL
    )",
    // `role` carries no SQL DEFAULT on purpose: every insert goes through `add_member`,
    // which always binds a `Role`-derived string, so the only spellings of role values
    // live on the `Role` enum.
    "CREATE TABLE IF NOT EXISTS org_members (
        org_id  TEXT NOT NULL REFERENCES orgs(id),
        user_id TEXT NOT NULL REFERENCES users(id),
        role    TEXT NOT NULL,
        PRIMARY KEY (org_id, user_id)
    )",
    // The scene-library folder tree. `parent_id` is self-referential (NULL = a root
    // folder); `ON DELETE CASCADE` lets a subtree delete remove descendant folder rows
    // (and, via the `folder_permissions` FK below, their ACL grants) in one statement.
    // `folders` is a new table, so it always gets the constraint — unlike `scenes`.
    "CREATE TABLE IF NOT EXISTS folders (
        id            TEXT PRIMARY KEY,
        name          TEXT NOT NULL,
        parent_id     TEXT REFERENCES folders(id) ON DELETE CASCADE,
        org_id        TEXT NOT NULL,
        owner_user_id TEXT,
        created_at    TEXT NOT NULL,
        updated_at    TEXT NOT NULL
    )",
    // Folder access-control list. Keyed by `(folder_id, principal_kind, principal_id)` so
    // user and group grants share one table; the future sharing endpoints write here while
    // part-1 reads it via `effective_permission`. `role` has no SQL DEFAULT (same rationale
    // as `org_members`): every write binds a `Permission`/`PrincipalKind`-derived string.
    "CREATE TABLE IF NOT EXISTS folder_permissions (
        folder_id      TEXT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
        principal_kind TEXT NOT NULL,
        principal_id   TEXT NOT NULL,
        permission     TEXT NOT NULL,
        granted_by     TEXT,
        created_at     TEXT NOT NULL,
        PRIMARY KEY (folder_id, principal_kind, principal_id)
    )",
    // Groups (teams) and their membership — the part-2 ACL principals. Tables exist now
    // (cheap, additive) so the permission foundation and its contract tests can exercise
    // group grants; the management endpoints arrive with the sharing feature.
    "CREATE TABLE IF NOT EXISTS groups (
        id         TEXT PRIMARY KEY,
        name       TEXT NOT NULL,
        org_id     TEXT NOT NULL REFERENCES orgs(id),
        created_at TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS group_members (
        group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
        user_id  TEXT NOT NULL REFERENCES users(id),
        role     TEXT NOT NULL,
        PRIMARY KEY (group_id, user_id)
    )",
    // Serves `list_scenes` (`WHERE owner = ? ORDER BY updated_at DESC`) fully —
    // filter and order — instead of a table scan plus sort on every library listing.
    "CREATE INDEX IF NOT EXISTS idx_scenes_owner_updated_at
     ON scenes (owner, updated_at DESC)",
    // Serves `list_scenes_in_folder` (`WHERE owner = ? AND folder_id … ORDER BY
    // updated_at DESC`) — the per-folder listing the tree UI issues on every navigation.
    "CREATE INDEX IF NOT EXISTS idx_scenes_owner_folder_updated_at
     ON scenes (owner, folder_id, updated_at DESC)",
    // Serves `list_folders` (children of a parent within an org).
    "CREATE INDEX IF NOT EXISTS idx_folders_org_parent ON folders (org_id, parent_id)",
    // Serves the group-grant side of `effective_permission` (grants naming a principal).
    "CREATE INDEX IF NOT EXISTS idx_folder_perms_principal
     ON folder_permissions (principal_kind, principal_id)",
    // Makes `prune_sessions` (`DELETE … WHERE expires_at <= ?`) proportional to the
    // number of expired rows rather than the whole table, on every prune tick.
    "CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions (expires_at)",
];

/// Map a `fetch_optional` result row, turning an absent row into
/// [`StoreError::NotFound`]. Every single-row lookup must route through this: the HTTP
/// layer maps `NotFound` to 404 and all other errors to 500, so the empty-row contract
/// is enforced here by construction instead of by copy-paste discipline at each site.
fn require_row<T>(
    row: Option<SqliteRow>,
    map: impl FnOnce(&SqliteRow) -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    match row {
        Some(r) => map(&r),
        None => Err(StoreError::NotFound),
    }
}

/// Read a TEXT timestamp column back into the domain's [`Timestamp`]. The validation
/// re-runs on every read on purpose: a row written by something other than this code
/// (manual SQL, an older version) surfaces as a `Backend` error instead of a silently
/// mis-sorted scene list.
fn timestamp_from_row(r: &SqliteRow, column: &str) -> Result<Timestamp, StoreError> {
    let raw: String = r.try_get(column)?;
    Timestamp::parse(&raw).map_err(|e| StoreError::Backend(Box::new(e)))
}

fn scene_from_row(r: &SqliteRow) -> Result<Scene, StoreError> {
    Ok(Scene {
        id: r.try_get("id")?,
        name: r.try_get("name")?,
        document_id: r.try_get("document_id")?,
        key: r.try_get("key")?,
        owner: r.try_get("owner")?,
        folder_id: r.try_get("folder_id")?,
        owner_user_id: r.try_get("owner_user_id")?,
        created_at: timestamp_from_row(r, "created_at")?,
        updated_at: timestamp_from_row(r, "updated_at")?,
    })
}

/// The column list every `scene_from_row` SELECT must project, in one place so the row
/// mapper and its queries cannot drift.
const SCENE_COLUMNS: &str =
    "id, name, document_id, key, owner, folder_id, owner_user_id, created_at, updated_at";

fn folder_from_row(r: &SqliteRow) -> Result<Folder, StoreError> {
    Ok(Folder {
        id: r.try_get("id")?,
        name: r.try_get("name")?,
        parent_id: r.try_get("parent_id")?,
        org_id: r.try_get("org_id")?,
        owner_user_id: r.try_get("owner_user_id")?,
        created_at: timestamp_from_row(r, "created_at")?,
        updated_at: timestamp_from_row(r, "updated_at")?,
    })
}

#[async_trait]
impl DocumentStore for SqliteStore {
    async fn find_id(&self, id: &str) -> Result<Document, StoreError> {
        let row = sqlx::query("SELECT data FROM documents WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        require_row(row, |r| {
            Ok(Document {
                data: r.try_get::<Vec<u8>, _>("data")?,
            })
        })
    }

    async fn create(&self, document: Document) -> Result<String, StoreError> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO documents (id, data) VALUES (?, ?)")
            .bind(&id)
            .bind(&document.data)
            .execute(&self.pool)
            .await?;
        Ok(id)
    }

    async fn documents_total_bytes(&self) -> Result<u64, StoreError> {
        fetch_scalar_u64(
            &self.pool,
            sqlx::query_scalar("SELECT COALESCE(SUM(LENGTH(data)), 0) FROM documents"),
        )
        .await
    }
}

/// Fetch a single scalar that is non-negative by construction — `COUNT(*)` or
/// `COALESCE(SUM(LENGTH(...)), 0)` — and widen it to `u64`. The sole conversion point
/// for count/quota queries: a negative value would mean backend corruption, surfaced as
/// [`StoreError::Backend`] rather than silently wrapped.
async fn fetch_scalar_u64<'q>(
    pool: &SqlitePool,
    query: sqlx::query::QueryScalar<'q, sqlx::Sqlite, i64, sqlx::sqlite::SqliteArguments<'q>>,
) -> Result<u64, StoreError> {
    let value: i64 = query.fetch_one(pool).await?;
    u64::try_from(value).map_err(|e| StoreError::Backend(Box::new(e)))
}

#[async_trait]
impl SceneStore for SqliteStore {
    async fn list_scenes(&self, owner: &str) -> Result<Vec<Scene>, StoreError> {
        // SEC-33: no SQL `LIMIT`, but the result is bounded by the per-owner scene cap
        // enforced at creation — see the `SceneStore` trait docs.
        let rows = sqlx::query(&format!(
            "SELECT {SCENE_COLUMNS} FROM scenes WHERE owner = ? ORDER BY updated_at DESC"
        ))
        .bind(owner)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(scene_from_row).collect()
    }

    async fn list_scenes_in_folder(
        &self,
        owner: &str,
        folder: Option<FolderId<'_>>,
    ) -> Result<Vec<Scene>, StoreError> {
        // `folder_id = ?` never matches NULL, so the root (folder = None) needs an
        // explicit `IS NULL` arm. Binding `None` to the same `?` and guarding it with
        // `? IS NULL` keeps it a single prepared statement for both cases.
        let folder = folder.map(|f| f.0);
        let rows = sqlx::query(&format!(
            "SELECT {SCENE_COLUMNS} FROM scenes
             WHERE owner = ? AND (folder_id = ? OR (? IS NULL AND folder_id IS NULL))
             ORDER BY updated_at DESC"
        ))
        .bind(owner)
        .bind(folder)
        .bind(folder)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(scene_from_row).collect()
    }

    async fn count_scenes(&self, owner: &str) -> Result<u64, StoreError> {
        fetch_scalar_u64(
            &self.pool,
            sqlx::query_scalar("SELECT COUNT(*) FROM scenes WHERE owner = ?").bind(owner),
        )
        .await
    }

    async fn create_scene(&self, scene: Scene) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO scenes
                (id, name, document_id, key, owner, folder_id, owner_user_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&scene.id)
        .bind(&scene.name)
        .bind(&scene.document_id)
        .bind(&scene.key)
        .bind(&scene.owner)
        .bind(&scene.folder_id)
        .bind(&scene.owner_user_id)
        .bind(scene.created_at.as_str())
        .bind(scene.updated_at.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_scene(&self, id: &str) -> Result<Scene, StoreError> {
        let row = sqlx::query(&format!("SELECT {SCENE_COLUMNS} FROM scenes WHERE id = ?"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        require_row(row, scene_from_row)
    }

    async fn move_scene(
        &self,
        id: &str,
        owner: OrgId<'_>,
        folder: Option<FolderId<'_>>,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        // Org isolation at the store (SEC-20): a destination folder that exists must belong
        // to `owner`, and the `owner` predicate on the UPDATE refuses to relocate a scene the
        // caller does not own. Both reads and the write share one BEGIN IMMEDIATE transaction
        // so a concurrent reparent of the destination can't move it cross-org between the
        // check and the write. A non-existent destination stays allowed — scenes carry no FK.
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if let Some(folder) = folder {
            if let Some(dest_org) = folder_org(&mut *tx, folder.0).await? {
                if dest_org != owner.0 {
                    return Err(StoreError::NotFound);
                }
            }
        }
        let result = sqlx::query(
            "UPDATE scenes SET folder_id = ?, updated_at = ? WHERE id = ? AND owner = ?",
        )
        .bind(folder.map(|f| f.0))
        .bind(now.as_str())
        .bind(id)
        .bind(owner.0)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        tx.commit().await?;
        Ok(())
    }

    async fn rename_scene(&self, id: &str, name: &str, now: Timestamp) -> Result<(), StoreError> {
        let result = sqlx::query("UPDATE scenes SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(now.as_str())
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn delete_scene(&self, id: &str) -> Result<(), StoreError> {
        let result = sqlx::query("DELETE FROM scenes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
}

/// Bound on every recursive folder walk. A correct tree never nests deeper than
/// [`MAX_FOLDER_DEPTH`]; the `+ 2` headroom lets a walk still *observe* a one-level
/// violation (so it can be rejected) and stops a corrupt/cyclic `parent_id` graph from
/// looping forever.
const FOLDER_WALK_LIMIT: i64 = MAX_FOLDER_DEPTH as i64 + 2;

/// Sentinel returned by `folder_depth`'s `COALESCE(MAX(depth), -1)` when no row matches;
/// distinguishes "no such folder" from a genuine out-of-range conversion failure.
const FOLDER_ABSENT_SENTINEL: i64 = -1;

/// Depth of `id` from its root (a root folder is depth 0), or `None` if no such folder
/// exists. Walks `parent_id` upward, bounded by [`FOLDER_WALK_LIMIT`]. Generic over the
/// executor so create/move can run it on the same transaction as their write (CONC-2).
async fn folder_depth<'e, E>(executor: E, id: &str) -> Result<Option<usize>, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let depth: i64 = sqlx::query_scalar(
        "WITH RECURSIVE anc(id, parent_id, depth) AS (
            SELECT id, parent_id, 0 FROM folders WHERE id = ?
            UNION ALL
            SELECT f.id, f.parent_id, anc.depth + 1
            FROM folders f JOIN anc ON f.id = anc.parent_id
            WHERE anc.depth < ?
         )
         SELECT COALESCE(MAX(depth), -1) FROM anc",
    )
    .bind(id)
    .bind(FOLDER_WALK_LIMIT)
    .fetch_one(executor)
    .await?;
    // `-1` is the `COALESCE(MAX(depth), -1)` sentinel for "no such folder" → `None`. Any
    // other out-of-range scalar is a genuine conversion failure and surfaces as a backend
    // error rather than being silently collapsed into the NotFound path — matching how
    // `subtree_height` treats the same scalar (READ-6).
    if depth == FOLDER_ABSENT_SENTINEL {
        return Ok(None);
    }
    usize::try_from(depth)
        .map(Some)
        .map_err(|e| StoreError::Backend(Box::new(e)))
}

/// Height of the subtree rooted at `id`: 0 for a leaf, N for a chain of N descendants.
async fn subtree_height<'e, E>(executor: E, id: &str) -> Result<usize, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let height: i64 = sqlx::query_scalar(
        "WITH RECURSIVE sub(id, depth) AS (
            SELECT id, 0 FROM folders WHERE id = ?
            UNION ALL
            SELECT f.id, sub.depth + 1
            FROM folders f JOIN sub ON f.parent_id = sub.id
            WHERE sub.depth < ?
         )
         SELECT COALESCE(MAX(depth), 0) FROM sub",
    )
    .bind(id)
    .bind(FOLDER_WALK_LIMIT)
    .fetch_one(executor)
    .await?;
    usize::try_from(height).map_err(|e| StoreError::Backend(Box::new(e)))
}

/// Whether `target` is `start` itself or one of its ancestors — the cycle test for
/// reparenting `target` under `start`.
async fn is_ancestor_or_self<'e, E>(
    executor: E,
    start: &str,
    target: &str,
) -> Result<bool, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let hit: bool = sqlx::query_scalar(
        "WITH RECURSIVE anc(id, parent_id, depth) AS (
            SELECT id, parent_id, 0 FROM folders WHERE id = ?
            UNION ALL
            SELECT f.id, f.parent_id, anc.depth + 1
            FROM folders f JOIN anc ON f.id = anc.parent_id
            WHERE anc.depth < ?
         )
         SELECT EXISTS(SELECT 1 FROM anc WHERE id = ?)",
    )
    .bind(start)
    .bind(FOLDER_WALK_LIMIT)
    .bind(target)
    .fetch_one(executor)
    .await?;
    Ok(hit)
}

/// Fetch a folder's `org_id`, or `None` if no such folder exists. The org-isolation probe
/// shared by the transactional move paths (SEC-20).
async fn folder_org<'e, E>(executor: E, id: &str) -> Result<Option<String>, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    Ok(
        sqlx::query_scalar("SELECT org_id FROM folders WHERE id = ?")
            .bind(id)
            .fetch_optional(executor)
            .await?,
    )
}

#[async_trait]
impl FolderStore for SqliteStore {
    async fn ensure_root_folder(
        &self,
        org_id: OrgId<'_>,
        now: Timestamp,
    ) -> Result<String, StoreError> {
        let id = format!("root:{}", org_id.0);
        sqlx::query(
            "INSERT OR IGNORE INTO folders
                (id, name, parent_id, org_id, owner_user_id, created_at, updated_at)
             VALUES (?, 'Root', NULL, ?, NULL, ?, ?)",
        )
        .bind(&id)
        .bind(org_id.0)
        .bind(now.as_str())
        .bind(now.as_str())
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn find_folder(&self, id: FolderId<'_>) -> Result<Folder, StoreError> {
        let row = sqlx::query(
            "SELECT id, name, parent_id, org_id, owner_user_id, created_at, updated_at
             FROM folders WHERE id = ?",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?;
        require_row(row, folder_from_row)
    }

    async fn list_folders(
        &self,
        org_id: OrgId<'_>,
        parent: Option<FolderId<'_>>,
    ) -> Result<Vec<Folder>, StoreError> {
        let parent = parent.map(|p| p.0);
        // SEC-33: no SQL `LIMIT`, but bounded by the per-org folder cap enforced at
        // creation — see the `FolderStore` trait docs.
        let rows = sqlx::query(
            "SELECT id, name, parent_id, org_id, owner_user_id, created_at, updated_at
             FROM folders
             WHERE org_id = ? AND (parent_id = ? OR (? IS NULL AND parent_id IS NULL))
             ORDER BY name",
        )
        .bind(org_id.0)
        .bind(parent)
        .bind(parent)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(folder_from_row).collect()
    }

    async fn count_folders(&self, org_id: OrgId<'_>) -> Result<u64, StoreError> {
        fetch_scalar_u64(
            &self.pool,
            sqlx::query_scalar("SELECT COUNT(*) FROM folders WHERE org_id = ?").bind(org_id.0),
        )
        .await
    }

    async fn create_folder(&self, folder: Folder) -> Result<(), FolderMoveError> {
        // The parent-depth check and the INSERT run in one BEGIN IMMEDIATE transaction so a
        // concurrent reparent of the parent cannot push this new folder past
        // MAX_FOLDER_DEPTH between the check and the write (CONC-2).
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(StoreError::from)?;
        if let Some(parent) = &folder.parent_id {
            match folder_depth(&mut *tx, parent).await? {
                None => return Err(FolderMoveError::NotFound),
                Some(parent_depth) if parent_depth + 1 > MAX_FOLDER_DEPTH => {
                    return Err(FolderMoveError::TooDeep)
                }
                Some(_) => {}
            }
        }
        sqlx::query(
            "INSERT INTO folders
                (id, name, parent_id, org_id, owner_user_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&folder.id)
        .bind(&folder.name)
        .bind(&folder.parent_id)
        .bind(&folder.org_id)
        .bind(&folder.owner_user_id)
        .bind(folder.created_at.as_str())
        .bind(folder.updated_at.as_str())
        .execute(&mut *tx)
        .await
        .map_err(StoreError::from)?;
        tx.commit().await.map_err(StoreError::from)?;
        Ok(())
    }

    async fn rename_folder(
        &self,
        id: FolderId<'_>,
        name: &str,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        let result = sqlx::query("UPDATE folders SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(now.as_str())
            .bind(id.0)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn move_folder(
        &self,
        id: FolderId<'_>,
        org: OrgId<'_>,
        new_parent: Option<FolderId<'_>>,
        now: Timestamp,
    ) -> Result<(), FolderMoveError> {
        // All validation reads (existence, org, cycle, depth) and the UPDATE share one BEGIN
        // IMMEDIATE transaction (CONC-2): a concurrent reparent cannot change the tree between
        // a check and the write, so two individually-acyclic moves cannot combine into a
        // cycle or push past MAX_FOLDER_DEPTH.
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(StoreError::from)?;
        // Org isolation at the store (SEC-20): the moved folder must belong to `org`.
        match folder_org(&mut *tx, id.0).await? {
            None => return Err(FolderMoveError::NotFound),
            Some(folder_org) if folder_org != org.0 => return Err(FolderMoveError::NotFound),
            Some(_) => {}
        }
        if let Some(parent) = new_parent {
            // ... and so must the destination — never reparent across tenants.
            match folder_org(&mut *tx, parent.0).await? {
                None => return Err(FolderMoveError::NotFound),
                Some(parent_org) if parent_org != org.0 => return Err(FolderMoveError::NotFound),
                Some(_) => {}
            }
            // Reparenting under self or a descendant would orphan the subtree into a cycle.
            if is_ancestor_or_self(&mut *tx, parent.0, id.0).await? {
                return Err(FolderMoveError::Cycle);
            }
            let parent_depth = folder_depth(&mut *tx, parent.0)
                .await?
                .ok_or(FolderMoveError::NotFound)?;
            let height = subtree_height(&mut *tx, id.0).await?;
            if parent_depth + 1 + height > MAX_FOLDER_DEPTH {
                return Err(FolderMoveError::TooDeep);
            }
        }
        sqlx::query("UPDATE folders SET parent_id = ?, updated_at = ? WHERE id = ?")
            .bind(new_parent.map(|p| p.0))
            .bind(now.as_str())
            .bind(id.0)
            .execute(&mut *tx)
            .await
            .map_err(StoreError::from)?;
        tx.commit().await.map_err(StoreError::from)?;
        Ok(())
    }

    async fn delete_folder(&self, id: FolderId<'_>) -> Result<(), StoreError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        // Cascade scene deletion across the whole subtree (folder rows and their ACL
        // grants cascade via the FKs once the root row is deleted, but scenes carry no
        // declared FK — see the SCHEMA doc comment — so they are removed explicitly).
        sqlx::query(
            "DELETE FROM scenes WHERE folder_id IN (
                WITH RECURSIVE sub(id, depth) AS (
                    SELECT id, 0 FROM folders WHERE id = ?
                    UNION ALL
                    SELECT f.id, sub.depth + 1 FROM folders f JOIN sub ON f.parent_id = sub.id
                    WHERE sub.depth < ?
                )
                SELECT id FROM sub
            )",
        )
        .bind(id.0)
        .bind(FOLDER_WALK_LIMIT)
        .execute(&mut *tx)
        .await?;
        let result = sqlx::query("DELETE FROM folders WHERE id = ?")
            .bind(id.0)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            // No such folder: nothing was deleted, so the (no-op) scene delete rolls back too.
            return Err(StoreError::NotFound);
        }
        tx.commit().await?;
        Ok(())
    }

    async fn effective_permission(
        &self,
        folder: FolderId<'_>,
        user: UserId<'_>,
    ) -> Result<Option<Permission>, StoreError> {
        // An authorization decision must rest on one consistent snapshot: the folder lookup,
        // the membership probe, and the ACL walk run in a single transaction (CONC-2) so a
        // concurrent grant/move cannot yield a torn decision built from two different states.
        let mut tx = self.pool.begin().await?;
        let folder_row = {
            let row = sqlx::query(
                "SELECT id, name, parent_id, org_id, owner_user_id, created_at, updated_at
                 FROM folders WHERE id = ?",
            )
            .bind(folder.0)
            .fetch_optional(&mut *tx)
            .await?;
            require_row(row, folder_from_row)?
        };
        let mut best: Option<Permission> = None;
        let mut consider = |p: Permission| {
            best = Some(best.map_or(p, |b| b.max(p)));
        };

        // Ownership ⇒ Admin.
        if folder_row.owner_user_id.as_deref() == Some(user.0) {
            consider(Permission::Admin);
        }
        // Org membership ⇒ Editor (part-1 shared-library model).
        let is_member: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM org_members WHERE org_id = ? AND user_id = ?)",
        )
        .bind(&folder_row.org_id)
        .bind(user.0)
        .fetch_one(&mut *tx)
        .await?;
        if is_member {
            consider(Permission::Editor);
        }
        // ACL grants on this folder or any ancestor, for the user or a group they're in.
        let grants: Vec<String> = sqlx::query_scalar(
            "WITH RECURSIVE anc(id, parent_id, depth) AS (
                SELECT id, parent_id, 0 FROM folders WHERE id = ?
                UNION ALL
                SELECT f.id, f.parent_id, anc.depth + 1
                FROM folders f JOIN anc ON f.id = anc.parent_id
                WHERE anc.depth < ?
             )
             SELECT fp.permission FROM folder_permissions fp
             JOIN anc ON fp.folder_id = anc.id
             WHERE (fp.principal_kind = 'user' AND fp.principal_id = ?)
                OR (fp.principal_kind = 'group' AND fp.principal_id IN
                    (SELECT group_id FROM group_members WHERE user_id = ?))",
        )
        .bind(folder.0)
        .bind(FOLDER_WALK_LIMIT)
        .bind(user.0)
        .bind(user.0)
        .fetch_all(&mut *tx)
        .await?;
        // Read-only snapshot: nothing to persist, so close it before parsing the results.
        tx.commit().await?;
        for raw in grants {
            let perm = raw
                .parse::<Permission>()
                .map_err(|e| StoreError::Decode(Box::new(e)))?;
            consider(perm);
        }
        Ok(best)
    }

    async fn set_permission(
        &self,
        folder: FolderId<'_>,
        grant: FolderGrant,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        // The FK would reject an orphan grant, but as a generic constraint error (→ 500);
        // probe first so a missing folder is the contract's NotFound (→ 404).
        self.find_folder(folder).await?;
        sqlx::query(
            "INSERT INTO folder_permissions
                (folder_id, principal_kind, principal_id, permission, granted_by, created_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(folder_id, principal_kind, principal_id)
             DO UPDATE SET permission = excluded.permission, granted_by = excluded.granted_by",
        )
        .bind(folder.0)
        .bind(grant.principal_kind.as_str())
        .bind(&grant.principal_id)
        .bind(grant.permission.as_str())
        .bind(&grant.granted_by)
        .bind(now.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_permissions(&self, folder: FolderId<'_>) -> Result<Vec<FolderGrant>, StoreError> {
        // SEC-33: no SQL `LIMIT`; grants only enter via `set_permission`, which has no
        // HTTP route yet — see the `FolderStore` trait docs for the cap the future sharing
        // endpoint must enforce.
        let rows = sqlx::query(
            "SELECT principal_kind, principal_id, permission, granted_by
             FROM folder_permissions WHERE folder_id = ?
             ORDER BY principal_kind, principal_id",
        )
        .bind(folder.0)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                let kind: String = r.try_get("principal_kind")?;
                let permission: String = r.try_get("permission")?;
                Ok(FolderGrant {
                    principal_kind: kind.parse().map_err(|e| StoreError::Decode(Box::new(e)))?,
                    principal_id: r.try_get("principal_id")?,
                    permission: permission
                        .parse()
                        .map_err(|e| StoreError::Decode(Box::new(e)))?,
                    granted_by: r.try_get("granted_by")?,
                })
            })
            .collect()
    }

    async fn remove_permission(
        &self,
        folder: FolderId<'_>,
        kind: PrincipalKind,
        principal_id: &str,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "DELETE FROM folder_permissions
             WHERE folder_id = ? AND principal_kind = ? AND principal_id = ?",
        )
        .bind(folder.0)
        .bind(kind.as_str())
        .bind(principal_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl FileStore for SqliteStore {
    async fn put_file(&self, path: &str, file: StoredFile) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO files (path, content_type, data) VALUES (?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET content_type = excluded.content_type,
                                             data = excluded.data",
        )
        .bind(path)
        .bind(&file.content_type)
        .bind(&file.data)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_file(&self, path: &str) -> Result<StoredFile, StoreError> {
        let row = sqlx::query("SELECT content_type, data FROM files WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;
        require_row(row, |r| {
            Ok(StoredFile {
                content_type: r.try_get("content_type")?,
                data: r.try_get("data")?,
            })
        })
    }

    async fn files_total_bytes(&self) -> Result<u64, StoreError> {
        fetch_scalar_u64(
            &self.pool,
            sqlx::query_scalar("SELECT COALESCE(SUM(LENGTH(data)), 0) FROM files"),
        )
        .await
    }

    async fn count_files(&self) -> Result<u64, StoreError> {
        fetch_scalar_u64(&self.pool, sqlx::query_scalar("SELECT COUNT(*) FROM files")).await
    }
}

fn user_from_row(r: &SqliteRow) -> Result<User, StoreError> {
    Ok(User {
        id: r.try_get("id")?,
        email: r.try_get("email")?,
        name: r.try_get("name")?,
        avatar_url: r.try_get("avatar_url")?,
        created_at: timestamp_from_row(r, "created_at")?,
    })
}

/// Returning login: refresh the user's profile fields (and the identity's email) with
/// whatever the provider sent this time.
async fn refresh_existing_user(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    user_id: &str,
    profile: &IdentityProfile,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE users SET email = ?, name = ?, avatar_url = ? WHERE id = ?")
        .bind(&profile.email)
        .bind(&profile.name)
        .bind(&profile.avatar_url)
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("UPDATE identities SET email = ? WHERE provider = ? AND provider_user_id = ?")
        .bind(&profile.email)
        .bind(&profile.provider)
        .bind(&profile.provider_user_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// First login: mint a user id and create the user + identity rows; `now` becomes the
/// user's `created_at`.
async fn create_user_with_identity(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    profile: &IdentityProfile,
    now: Timestamp,
) -> Result<String, StoreError> {
    let user_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (id, email, name, avatar_url, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(&profile.email)
    .bind(&profile.name)
    .bind(&profile.avatar_url)
    .bind(now.as_str())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO identities (provider, provider_user_id, user_id, email)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&profile.provider)
    .bind(&profile.provider_user_id)
    .bind(&user_id)
    .bind(&profile.email)
    .execute(&mut **tx)
    .await?;
    Ok(user_id)
}

#[async_trait]
impl UserStore for SqliteStore {
    async fn upsert_user_for_identity(
        &self,
        profile: &IdentityProfile,
        now: Timestamp,
    ) -> Result<User, StoreError> {
        // BEGIN IMMEDIATE: take SQLite's write lock before the SELECT. A deferred
        // transaction would let two concurrent first logins both observe "no identity",
        // then collide on the identities INSERT — one caller gets a constraint/busy
        // error and an orphan users row is left behind. Serializing at BEGIN makes the
        // loser wait (within BUSY_TIMEOUT) and then see the winner's committed row.
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;

        let existing = sqlx::query(
            "SELECT user_id FROM identities WHERE provider = ? AND provider_user_id = ?",
        )
        .bind(&profile.provider)
        .bind(&profile.provider_user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let user_id = match existing {
            Some(row) => {
                let user_id: String = row.try_get("user_id")?;
                refresh_existing_user(&mut tx, &user_id, profile).await?;
                user_id
            }
            None => create_user_with_identity(&mut tx, profile, now).await?,
        };

        let row =
            sqlx::query("SELECT id, email, name, avatar_url, created_at FROM users WHERE id = ?")
                .bind(&user_id)
                .fetch_one(&mut *tx)
                .await?;
        tx.commit().await?;
        user_from_row(&row)
    }

    async fn find_user(&self, id: &str) -> Result<User, StoreError> {
        let row =
            sqlx::query("SELECT id, email, name, avatar_url, created_at FROM users WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        require_row(row, user_from_row)
    }
}

#[async_trait]
impl SessionStore for SqliteStore {
    async fn create_session(&self, session: Session) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES (?, ?, ?)")
            .bind(&session.token_hash)
            .bind(&session.user_id)
            .bind(session.expires_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_session(&self, token_hash: TokenHash<'_>) -> Result<Session, StoreError> {
        let row = sqlx::query(
            "SELECT token_hash, user_id, expires_at FROM sessions WHERE token_hash = ?",
        )
        .bind(token_hash.0)
        .fetch_optional(&self.pool)
        .await?;
        require_row(row, |r| {
            Ok(Session {
                token_hash: r.try_get("token_hash")?,
                user_id: r.try_get("user_id")?,
                expires_at: r.try_get("expires_at")?,
            })
        })
    }

    async fn delete_session(&self, token_hash: TokenHash<'_>) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(token_hash.0)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn prune_sessions(&self, now: i64) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl OrgStore for SqliteStore {
    async fn ensure_org(&self, org: Org) -> Result<(), StoreError> {
        sqlx::query("INSERT OR IGNORE INTO orgs (id, name, created_at) VALUES (?, ?, ?)")
            .bind(&org.id)
            .bind(&org.name)
            .bind(org.created_at.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_member(
        &self,
        org_id: OrgId<'_>,
        user_id: UserId<'_>,
        role: Role,
    ) -> Result<(), StoreError> {
        sqlx::query("INSERT OR IGNORE INTO org_members (org_id, user_id, role) VALUES (?, ?, ?)")
            .bind(org_id.0)
            .bind(user_id.0)
            .bind(role.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn org_for_user(&self, user_id: UserId<'_>) -> Result<Org, StoreError> {
        let row = sqlx::query(
            "SELECT o.id, o.name, o.created_at FROM orgs o
             JOIN org_members m ON m.org_id = o.id
             WHERE m.user_id = ? ORDER BY o.created_at LIMIT 1",
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await?;
        require_row(row, |r| {
            Ok(Org {
                id: r.try_get("id")?,
                name: r.try_get("name")?,
                created_at: timestamp_from_row(r, "created_at")?,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh store per test; the temp file is the guard keeping the database alive.
    async fn fresh() -> (tempfile::NamedTempFile, SqliteStore) {
        let file = tempfile::NamedTempFile::new().unwrap();
        let url = format!("sqlite://{}?mode=rwc", file.path().display());
        let store = SqliteStore::connect(&url).await.unwrap();
        (file, store)
    }

    crate::test_support::store_contract_tests!(fresh);

    /// Two overlapping first logins for the same identity must resolve to one user —
    /// neither caller may see a UNIQUE-constraint error, and no orphan `users` row may
    /// be left behind by the losing transaction. Multi-threaded flavor so the two
    /// transactions genuinely interleave (TEST-14).
    #[tokio::test(flavor = "multi_thread")]
    async fn concurrent_first_logins_resolve_to_one_user() {
        let (_guard, store) = fresh().await;
        let store = std::sync::Arc::new(store);
        let profile = oxydraw_core::store::IdentityProfile {
            provider: "github".to_string(),
            provider_user_id: "race-1".to_string(),
            email: Some("race@example.com".to_string()),
            name: Some("Racer".to_string()),
            avatar_url: None,
        };

        let now = Timestamp::parse("2026-01-01T00:00:00Z").unwrap();
        let (a, b) = {
            let (s1, p1, n1) = (store.clone(), profile.clone(), now.clone());
            let (s2, p2, n2) = (store.clone(), profile.clone(), now);
            tokio::join!(
                tokio::spawn(async move { s1.upsert_user_for_identity(&p1, n1).await }),
                tokio::spawn(async move { s2.upsert_user_for_identity(&p2, n2).await }),
            )
        };
        let a = a.unwrap().expect("first concurrent login succeeds");
        let b = b.unwrap().expect("second concurrent login succeeds");
        assert_eq!(a.id, b.id, "both logins resolve to the same user");

        let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(
            users, 1,
            "the losing transaction must not leave an orphan user"
        );
    }

    /// A database created before folders existed has the original 7-column `scenes`
    /// table. `connect` must add the two folder columns (so the new queries work) without
    /// losing existing rows, and a second `connect` must be a no-op (idempotent probe).
    #[tokio::test]
    async fn connect_migrates_a_pre_folders_scenes_table() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let url = format!("sqlite://{}?mode=rwc", file.path().display());

        // Stand up the old schema and a legacy scene row directly, then drop the pool.
        {
            let pool = SqlitePool::connect(&url).await.unwrap();
            sqlx::query(
                "CREATE TABLE scenes (
                    id TEXT PRIMARY KEY, name TEXT NOT NULL, document_id TEXT NOT NULL,
                    key TEXT NOT NULL, owner TEXT NOT NULL DEFAULT 'default',
                    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
                )",
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO scenes (id, name, document_id, key, owner, created_at, updated_at)
                 VALUES ('legacy', 'old scene', 'doc', 'k', 'default',
                         '2026-01-01T00:00:00.000000Z', '2026-01-01T00:00:00.000000Z')",
            )
            .execute(&pool)
            .await
            .unwrap();
            pool.close().await;
        }

        // First connect runs the column migration; the legacy row survives and reads back
        // with the new columns defaulted to NULL.
        let store = SqliteStore::connect(&url).await.unwrap();
        let legacy = store.find_scene("legacy").await.unwrap();
        assert_eq!(legacy.name, "old scene");
        assert_eq!(legacy.folder_id, None);
        assert_eq!(legacy.owner_user_id, None);
        // The new per-folder query works against the migrated table (root = NULL folder).
        let root = store.list_scenes_in_folder("default", None).await.unwrap();
        assert_eq!(
            root.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["legacy"]
        );
        drop(store);

        // A second connect re-probes and is a no-op (no "duplicate column" error).
        let reopened = SqliteStore::connect(&url).await.unwrap();
        assert!(reopened.find_scene("legacy").await.is_ok());
    }

    /// SEC-29: a database freshly created via `mode=rwc` must not be world-readable —
    /// it holds emails, identity mappings, and session-token hashes.
    #[cfg(unix)]
    #[tokio::test]
    async fn fresh_database_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fresh.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let store = SqliteStore::connect(&url).await.unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "database file is owner-only, got {mode:o}");

        // The WAL side file (created on first write) inherits the tightened mode.
        store
            .create_scene(oxydraw_core::model::Scene {
                id: "s1".to_string(),
                name: "n".to_string(),
                document_id: "d".to_string(),
                key: "k".to_string(),
                owner: "o".to_string(),
                folder_id: None,
                owner_user_id: None,
                created_at: Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
                updated_at: Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
            })
            .await
            .unwrap();
        // The WAL file is created when the pool opens the database in WAL mode and held
        // open for the connection's lifetime, so after a committed write it must exist —
        // assert that, rather than letting a missing file silently skip the only check
        // that the session-token-hash data in the WAL is not group/other-readable (TEST-11).
        let wal = path.with_extension("db-wal");
        let meta = std::fs::metadata(&wal).expect("WAL side file created by the first write");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode & 0o077,
            0,
            "WAL side file leaks to group/other: {mode:o}"
        );
    }

    /// SEC-33: a corrupt/cyclic `parent_id` graph — here a folder that is its own parent,
    /// inserted directly to bypass the create/move cycle guards — must not make
    /// `delete_folder`'s subtree CTE recurse forever under the write lock. The depth bound
    /// terminates the walk; the timeout turns a regression (an unbounded loop) into a test
    /// failure rather than a hung suite.
    #[tokio::test]
    async fn delete_folder_terminates_on_cyclic_parent_id() {
        let (_guard, store) = fresh().await;
        let ts = "2026-01-01T00:00:00.000000Z";
        sqlx::query(
            "INSERT INTO folders
                (id, name, parent_id, org_id, owner_user_id, created_at, updated_at)
             VALUES ('cycle', 'c', 'cycle', 'org', NULL, ?, ?)",
        )
        .bind(ts)
        .bind(ts)
        .execute(&store.pool)
        .await
        .expect("seed a self-cyclic folder row directly");

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            store.delete_folder(FolderId("cycle")),
        )
        .await
        .expect("delete_folder must terminate on a cyclic parent_id, not loop forever");
        result.expect("delete of the cyclic folder succeeds");

        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folders WHERE id = 'cycle'")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(remaining, 0, "the cyclic folder row is deleted");
    }
}
