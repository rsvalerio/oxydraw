//! In-memory store — volatile backend for development and tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use oxydraw_core::model::{Document, Folder, Org, Scene, Session, StoredFile, Timestamp, User};
use oxydraw_core::store::{
    DocumentStore, FileStore, FolderGrant, FolderId, FolderMoveError, FolderStore, IdentityProfile,
    OrgId, OrgStore, Permission, PrincipalKind, Role, SceneStore, SessionStore, StoreError,
    TokenHash, UserId, UserStore, MAX_FOLDER_DEPTH,
};
use oxydraw_core::sync::lock_unpoisoned;

/// Volatile store backed by an in-process map. Not durable across restarts.
#[derive(Default)]
pub struct MemoryStore {
    documents: Mutex<HashMap<String, Vec<u8>>>,
    scenes: Mutex<HashMap<String, Scene>>,
    files: Mutex<HashMap<String, StoredFile>>,
    users: Mutex<HashMap<String, User>>,
    /// `(provider, provider_user_id)` → user id.
    identities: Mutex<HashMap<(String, String), String>>,
    sessions: Mutex<HashMap<String, Session>>,
    orgs: Mutex<HashMap<String, Org>>,
    /// `(org_id, user_id)` → role.
    org_members: Mutex<HashMap<(String, String), Role>>,
    folders: Mutex<HashMap<String, Folder>>,
    /// `(folder_id, principal_kind, principal_id)` → grant.
    folder_permissions: Mutex<HashMap<(String, String, String), FolderGrant>>,
    /// `(group_id, user_id)` → role. Written by the future group-management feature; the
    /// part-1 `effective_permission` group-grant path reads it (empty for now).
    group_members: Mutex<HashMap<(String, String), Role>>,
}

/// Depth of `id` from its root (root = 0), or `None` if absent. A `visited` set bounds the
/// walk so a corrupt `parent_id` cycle cannot loop forever (creation/move prevent cycles).
fn folder_depth(map: &HashMap<String, Folder>, id: &str) -> Option<usize> {
    let mut current = map.get(id)?;
    let mut depth = 0;
    let mut visited = std::collections::HashSet::new();
    while let Some(parent_id) = &current.parent_id {
        if !visited.insert(current.id.clone()) {
            break;
        }
        match map.get(parent_id) {
            Some(parent) => {
                current = parent;
                depth += 1;
            }
            None => break,
        }
    }
    Some(depth)
}

/// Whether `target` is `start` itself or one of its ancestors — the cycle test for
/// reparenting `target` under `start`.
fn is_ancestor_or_self(map: &HashMap<String, Folder>, start: &str, target: &str) -> bool {
    let mut current = map.get(start);
    let mut visited = std::collections::HashSet::new();
    while let Some(folder) = current {
        if folder.id == target {
            return true;
        }
        if !visited.insert(folder.id.clone()) {
            break;
        }
        current = folder.parent_id.as_deref().and_then(|p| map.get(p));
    }
    false
}

/// `parent_id` → child folder ids, built in a single pass over `map`. Subtree walks use
/// this so they don't rescan the whole map once per node (PERF-1).
fn children_index(map: &HashMap<String, Folder>) -> HashMap<&str, Vec<&str>> {
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for folder in map.values() {
        if let Some(parent) = folder.parent_id.as_deref() {
            children.entry(parent).or_default().push(folder.id.as_str());
        }
    }
    children
}

/// All folder ids in the subtree rooted at `root` (inclusive). Empty if `root` is absent.
fn subtree_ids(map: &HashMap<String, Folder>, root: &str) -> Vec<String> {
    if !map.contains_key(root) {
        return Vec::new();
    }
    let children = children_index(map);
    let mut out = Vec::new();
    let mut stack = vec![root];
    let mut visited = std::collections::HashSet::new();
    while let Some(id) = stack.pop() {
        if !visited.insert(id) {
            continue;
        }
        if let Some(kids) = children.get(id) {
            stack.extend(kids.iter().copied());
        }
        out.push(id.to_string());
    }
    out
}

/// `id` and all of its ancestors up to the root. Empty if `id` is absent. Bounded by a
/// `visited` set against corrupt cycles.
fn ancestor_ids(map: &HashMap<String, Folder>, id: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = map.get(id);
    let mut visited = std::collections::HashSet::new();
    while let Some(folder) = current {
        if !visited.insert(folder.id.clone()) {
            break;
        }
        out.push(folder.id.clone());
        current = folder.parent_id.as_deref().and_then(|p| map.get(p));
    }
    out
}

/// Height of the subtree rooted at `id`: 0 for a leaf. Walks the subtree level by level so
/// each node's relative depth is carried from its parent instead of recomputing
/// `folder_depth` (a full ancestor walk) for every descendant (PERF-1).
fn subtree_height(map: &HashMap<String, Folder>, id: &str) -> usize {
    if !map.contains_key(id) {
        return 0;
    }
    let children = children_index(map);
    let mut max_depth = 0;
    let mut depth = 0;
    let mut visited = std::collections::HashSet::new();
    let mut frontier = vec![id];
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for node in frontier {
            if !visited.insert(node) {
                continue;
            }
            max_depth = depth;
            if let Some(kids) = children.get(node) {
                next.extend(kids.iter().copied());
            }
        }
        depth += 1;
        frontier = next;
    }
    max_depth
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DocumentStore for MemoryStore {
    async fn find_id(&self, id: &str) -> Result<Document, StoreError> {
        let guard = lock_unpoisoned(&self.documents);
        guard
            .get(id)
            .map(|data| Document { data: data.clone() })
            .ok_or(StoreError::NotFound)
    }

    async fn create(&self, document: Document) -> Result<String, StoreError> {
        let id = uuid::Uuid::new_v4().to_string();
        lock_unpoisoned(&self.documents).insert(id.clone(), document.data);
        Ok(id)
    }

    async fn documents_total_bytes(&self) -> Result<u64, StoreError> {
        let guard = lock_unpoisoned(&self.documents);
        Ok(guard.values().map(|data| data.len() as u64).sum())
    }
}

#[async_trait]
impl SceneStore for MemoryStore {
    async fn list_scenes(&self, owner: &str) -> Result<Vec<Scene>, StoreError> {
        // SEC-33: bounded by the per-owner scene cap enforced at creation — see the
        // `SceneStore` trait docs.
        let guard = lock_unpoisoned(&self.scenes);
        let mut scenes: Vec<Scene> = guard
            .values()
            .filter(|s| s.owner == owner)
            .cloned()
            .collect();
        scenes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(scenes)
    }

    async fn list_scenes_in_folder(
        &self,
        owner: &str,
        folder: Option<FolderId<'_>>,
    ) -> Result<Vec<Scene>, StoreError> {
        let folder = folder.map(|f| f.0);
        let guard = lock_unpoisoned(&self.scenes);
        let mut scenes: Vec<Scene> = guard
            .values()
            .filter(|s| s.owner == owner && s.folder_id.as_deref() == folder)
            .cloned()
            .collect();
        scenes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(scenes)
    }

    async fn count_scenes(&self, owner: &str) -> Result<u64, StoreError> {
        let guard = lock_unpoisoned(&self.scenes);
        Ok(guard.values().filter(|s| s.owner == owner).count() as u64)
    }

    async fn create_scene(&self, scene: Scene) -> Result<(), StoreError> {
        lock_unpoisoned(&self.scenes).insert(scene.id.clone(), scene);
        Ok(())
    }

    async fn find_scene(&self, id: &str) -> Result<Scene, StoreError> {
        lock_unpoisoned(&self.scenes)
            .get(id)
            .cloned()
            .ok_or(StoreError::NotFound)
    }

    async fn move_scene(
        &self,
        id: &str,
        owner: OrgId<'_>,
        folder: Option<FolderId<'_>>,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        // Org isolation at the store (SEC-20): a destination folder that exists must belong
        // to `owner`. A non-existent destination stays allowed — scenes carry no FK.
        if let Some(folder) = folder {
            let folders = lock_unpoisoned(&self.folders);
            if let Some(dest) = folders.get(folder.0) {
                if dest.org_id != owner.0 {
                    return Err(StoreError::NotFound);
                }
            }
        }
        let mut guard = lock_unpoisoned(&self.scenes);
        let scene = guard.get_mut(id).ok_or(StoreError::NotFound)?;
        // ... and the scene itself must be owned by `owner`.
        if scene.owner != owner.0 {
            return Err(StoreError::NotFound);
        }
        scene.folder_id = folder.map(|f| f.0.to_string());
        scene.updated_at = now;
        Ok(())
    }

    async fn rename_scene(&self, id: &str, name: &str, now: Timestamp) -> Result<(), StoreError> {
        let mut guard = lock_unpoisoned(&self.scenes);
        let scene = guard.get_mut(id).ok_or(StoreError::NotFound)?;
        scene.name = name.to_string();
        scene.updated_at = now;
        Ok(())
    }

    async fn delete_scene(&self, id: &str) -> Result<(), StoreError> {
        lock_unpoisoned(&self.scenes)
            .remove(id)
            .map(|_| ())
            .ok_or(StoreError::NotFound)
    }
}

#[async_trait]
impl FolderStore for MemoryStore {
    async fn ensure_root_folder(
        &self,
        org_id: OrgId<'_>,
        now: Timestamp,
    ) -> Result<String, StoreError> {
        let id = format!("root:{}", org_id.0);
        lock_unpoisoned(&self.folders)
            .entry(id.clone())
            .or_insert_with(|| Folder {
                id: id.clone(),
                name: "Root".to_string(),
                parent_id: None,
                org_id: org_id.0.to_string(),
                owner_user_id: None,
                created_at: now.clone(),
                updated_at: now,
            });
        Ok(id)
    }

    async fn find_folder(&self, id: FolderId<'_>) -> Result<Folder, StoreError> {
        lock_unpoisoned(&self.folders)
            .get(id.0)
            .cloned()
            .ok_or(StoreError::NotFound)
    }

    async fn list_folders(
        &self,
        org_id: OrgId<'_>,
        parent: Option<FolderId<'_>>,
    ) -> Result<Vec<Folder>, StoreError> {
        let parent = parent.map(|p| p.0);
        // SEC-33: bounded by the per-org folder cap enforced at creation — see the
        // `FolderStore` trait docs.
        let guard = lock_unpoisoned(&self.folders);
        let mut folders: Vec<Folder> = guard
            .values()
            .filter(|f| f.org_id == org_id.0 && f.parent_id.as_deref() == parent)
            .cloned()
            .collect();
        folders.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(folders)
    }

    async fn count_folders(&self, org_id: OrgId<'_>) -> Result<u64, StoreError> {
        let guard = lock_unpoisoned(&self.folders);
        Ok(guard.values().filter(|f| f.org_id == org_id.0).count() as u64)
    }

    async fn create_folder(&self, folder: Folder) -> Result<(), FolderMoveError> {
        let mut guard = lock_unpoisoned(&self.folders);
        if let Some(parent) = &folder.parent_id {
            match folder_depth(&guard, parent) {
                None => return Err(FolderMoveError::NotFound),
                Some(parent_depth) if parent_depth + 1 > MAX_FOLDER_DEPTH => {
                    return Err(FolderMoveError::TooDeep)
                }
                Some(_) => {}
            }
        }
        guard.insert(folder.id.clone(), folder);
        Ok(())
    }

    async fn rename_folder(
        &self,
        id: FolderId<'_>,
        name: &str,
        now: Timestamp,
    ) -> Result<(), StoreError> {
        let mut guard = lock_unpoisoned(&self.folders);
        let folder = guard.get_mut(id.0).ok_or(StoreError::NotFound)?;
        folder.name = name.to_string();
        folder.updated_at = now;
        Ok(())
    }

    async fn move_folder(
        &self,
        id: FolderId<'_>,
        org: OrgId<'_>,
        new_parent: Option<FolderId<'_>>,
        now: Timestamp,
    ) -> Result<(), FolderMoveError> {
        let mut guard = lock_unpoisoned(&self.folders);
        // Org isolation at the store (SEC-20): the moved folder must belong to `org`. The
        // single lock makes the whole check-then-write atomic, matching the sqlite tx.
        match guard.get(id.0) {
            None => return Err(FolderMoveError::NotFound),
            Some(folder) if folder.org_id != org.0 => return Err(FolderMoveError::NotFound),
            Some(_) => {}
        }
        if let Some(parent) = new_parent {
            // ... and so must the destination — never reparent across tenants.
            match guard.get(parent.0) {
                None => return Err(FolderMoveError::NotFound),
                Some(p) if p.org_id != org.0 => return Err(FolderMoveError::NotFound),
                Some(_) => {}
            }
            let parent_depth = match folder_depth(&guard, parent.0) {
                None => return Err(FolderMoveError::NotFound),
                Some(d) => d,
            };
            if is_ancestor_or_self(&guard, parent.0, id.0) {
                return Err(FolderMoveError::Cycle);
            }
            if parent_depth + 1 + subtree_height(&guard, id.0) > MAX_FOLDER_DEPTH {
                return Err(FolderMoveError::TooDeep);
            }
        }
        let folder = guard.get_mut(id.0).expect("existence checked above");
        folder.parent_id = new_parent.map(|p| p.0.to_string());
        folder.updated_at = now;
        Ok(())
    }

    async fn delete_folder(&self, id: FolderId<'_>) -> Result<(), StoreError> {
        let mut folders = lock_unpoisoned(&self.folders);
        let subtree = subtree_ids(&folders, id.0);
        if subtree.is_empty() {
            return Err(StoreError::NotFound);
        }
        let subtree: std::collections::HashSet<&str> = subtree.iter().map(String::as_str).collect();
        lock_unpoisoned(&self.scenes)
            .retain(|_, s| s.folder_id.as_deref().is_none_or(|f| !subtree.contains(f)));
        lock_unpoisoned(&self.folder_permissions)
            .retain(|(fid, _, _), _| !subtree.contains(fid.as_str()));
        folders.retain(|fid, _| !subtree.contains(fid.as_str()));
        Ok(())
    }

    async fn effective_permission(
        &self,
        folder: FolderId<'_>,
        user: UserId<'_>,
    ) -> Result<Option<Permission>, StoreError> {
        let folders = lock_unpoisoned(&self.folders);
        let target = folders.get(folder.0).ok_or(StoreError::NotFound)?;
        let mut best: Option<Permission> = None;
        let mut consider = |p: Permission| best = Some(best.map_or(p, |b| b.max(p)));

        if target.owner_user_id.as_deref() == Some(user.0) {
            consider(Permission::Admin);
        }
        let is_member = lock_unpoisoned(&self.org_members)
            .keys()
            .any(|(org_id, uid)| org_id == &target.org_id && uid == user.0);
        if is_member {
            consider(Permission::Editor);
        }
        // The user's groups, for the group-grant arm of the ACL.
        let user_groups: std::collections::HashSet<String> = lock_unpoisoned(&self.group_members)
            .keys()
            .filter(|(_, uid)| uid == user.0)
            .map(|(gid, _)| gid.clone())
            .collect();
        // Ancestor chain of the folder (inclusive).
        let ancestors: std::collections::HashSet<String> =
            ancestor_ids(&folders, folder.0).into_iter().collect();
        for ((fid, kind, pid), grant) in lock_unpoisoned(&self.folder_permissions).iter() {
            if !ancestors.contains(fid) {
                continue;
            }
            let applies = (kind == PrincipalKind::User.as_str() && pid == user.0)
                || (kind == PrincipalKind::Group.as_str() && user_groups.contains(pid));
            if applies {
                consider(grant.permission);
            }
        }
        Ok(best)
    }

    async fn set_permission(
        &self,
        folder: FolderId<'_>,
        grant: FolderGrant,
        _now: Timestamp,
    ) -> Result<(), StoreError> {
        if !lock_unpoisoned(&self.folders).contains_key(folder.0) {
            return Err(StoreError::NotFound);
        }
        lock_unpoisoned(&self.folder_permissions).insert(
            (
                folder.0.to_string(),
                grant.principal_kind.as_str().to_string(),
                grant.principal_id.clone(),
            ),
            grant,
        );
        Ok(())
    }

    async fn list_permissions(&self, folder: FolderId<'_>) -> Result<Vec<FolderGrant>, StoreError> {
        // SEC-33: grants only enter via `set_permission`, which has no HTTP route yet —
        // see the `FolderStore` trait docs for the cap the future sharing endpoint enforces.
        let guard = lock_unpoisoned(&self.folder_permissions);
        let mut grants: Vec<FolderGrant> = guard
            .iter()
            .filter(|((fid, _, _), _)| fid == folder.0)
            .map(|(_, g)| g.clone())
            .collect();
        grants.sort_by(|a, b| {
            a.principal_kind
                .as_str()
                .cmp(b.principal_kind.as_str())
                .then(a.principal_id.cmp(&b.principal_id))
        });
        Ok(grants)
    }

    async fn remove_permission(
        &self,
        folder: FolderId<'_>,
        kind: PrincipalKind,
        principal_id: &str,
    ) -> Result<(), StoreError> {
        lock_unpoisoned(&self.folder_permissions).remove(&(
            folder.0.to_string(),
            kind.as_str().to_string(),
            principal_id.to_string(),
        ));
        Ok(())
    }
}

#[async_trait]
impl FileStore for MemoryStore {
    async fn put_file(&self, path: &str, file: StoredFile) -> Result<(), StoreError> {
        lock_unpoisoned(&self.files).insert(path.to_string(), file);
        Ok(())
    }

    async fn get_file(&self, path: &str) -> Result<StoredFile, StoreError> {
        lock_unpoisoned(&self.files)
            .get(path)
            .cloned()
            .ok_or(StoreError::NotFound)
    }

    async fn files_total_bytes(&self) -> Result<u64, StoreError> {
        let guard = lock_unpoisoned(&self.files);
        Ok(guard.values().map(|f| f.data.len() as u64).sum())
    }

    async fn count_files(&self) -> Result<u64, StoreError> {
        Ok(lock_unpoisoned(&self.files).len() as u64)
    }
}

#[async_trait]
impl UserStore for MemoryStore {
    async fn upsert_user_for_identity(
        &self,
        profile: &IdentityProfile,
        now: Timestamp,
    ) -> Result<User, StoreError> {
        let key = (profile.provider.clone(), profile.provider_user_id.clone());
        let mut identities = lock_unpoisoned(&self.identities);
        let mut users = lock_unpoisoned(&self.users);

        let user_id = identities.get(&key).cloned().unwrap_or_else(|| {
            let id = uuid::Uuid::new_v4().to_string();
            identities.insert(key, id.clone());
            users.insert(
                id.clone(),
                User {
                    id: id.clone(),
                    created_at: now,
                    ..User::default()
                },
            );
            id
        });

        // The critical section above guarantees an identity entry always has a matching
        // user row (both maps are written under the locks held here) — but the invariant
        // spans two maps, so degrade to a backend error instead of panicking if a future
        // refactor breaks it.
        let user = users.get_mut(&user_id).ok_or_else(|| {
            StoreError::Backend(format!("identity maps to missing user {user_id}").into())
        })?;
        user.email = profile.email.clone();
        user.name = profile.name.clone();
        user.avatar_url = profile.avatar_url.clone();
        Ok(user.clone())
    }

    async fn find_user(&self, id: &str) -> Result<User, StoreError> {
        lock_unpoisoned(&self.users)
            .get(id)
            .cloned()
            .ok_or(StoreError::NotFound)
    }
}

#[async_trait]
impl SessionStore for MemoryStore {
    async fn create_session(&self, session: Session) -> Result<(), StoreError> {
        lock_unpoisoned(&self.sessions).insert(session.token_hash.clone(), session);
        Ok(())
    }

    async fn find_session(&self, token_hash: TokenHash<'_>) -> Result<Session, StoreError> {
        lock_unpoisoned(&self.sessions)
            .get(token_hash.0)
            .cloned()
            .ok_or(StoreError::NotFound)
    }

    async fn delete_session(&self, token_hash: TokenHash<'_>) -> Result<(), StoreError> {
        lock_unpoisoned(&self.sessions).remove(token_hash.0);
        Ok(())
    }

    async fn prune_sessions(&self, now: i64) -> Result<(), StoreError> {
        lock_unpoisoned(&self.sessions).retain(|_, s| s.expires_at > now);
        Ok(())
    }
}

#[async_trait]
impl OrgStore for MemoryStore {
    async fn ensure_org(&self, org: Org) -> Result<(), StoreError> {
        lock_unpoisoned(&self.orgs)
            .entry(org.id.clone())
            .or_insert(org);
        Ok(())
    }

    async fn add_member(
        &self,
        org_id: OrgId<'_>,
        user_id: UserId<'_>,
        role: Role,
    ) -> Result<(), StoreError> {
        lock_unpoisoned(&self.org_members)
            .entry((org_id.0.to_string(), user_id.0.to_string()))
            .or_insert(role);
        Ok(())
    }

    async fn org_for_user(&self, user_id: UserId<'_>) -> Result<Org, StoreError> {
        let members = lock_unpoisoned(&self.org_members);
        let orgs = lock_unpoisoned(&self.orgs);
        let mut found: Vec<&Org> = members
            .keys()
            .filter(|(_, uid)| uid == user_id.0)
            .filter_map(|(org_id, _)| orgs.get(org_id))
            .collect();
        // Oldest org first, matching the SQLite backend's ORDER BY created_at.
        found.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        found
            .first()
            .map(|o| (*o).clone())
            .ok_or(StoreError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh store per test; no backend resources, so the guard is `()`.
    async fn fresh() -> ((), MemoryStore) {
        ((), MemoryStore::new())
    }

    crate::test_support::store_contract_tests!(fresh);
}
