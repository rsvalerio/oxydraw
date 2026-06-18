// Navigation + listing state for the scene-library folder tree. Owns the current folder,
// its breadcrumb, and the folders/scenes inside it; the panel layers create/rename/move/
// delete on top (calling the api.ts mutators, then `reload`). State is kept here rather
// than in a global store because it is local to the library panel.

import { useCallback, useEffect, useRef, useState } from "react";

import { listFolders, listScenes, type LibraryFolder, type LibraryScene } from "./api";

export interface FolderBrowser {
  /** The folder being viewed; `null` is the root. */
  currentFolderId: string | null;
  /** Root-first chain to the current folder; empty at the root. */
  breadcrumb: LibraryFolder[];
  folders: LibraryFolder[];
  scenes: LibraryScene[];
  loading: boolean;
  error: string | null;
  /** Navigate to a folder (or `null` for the root); the listing reloads. */
  navigate: (folderId: string | null) => void;
  /** Re-fetch the current folder's contents (after a mutation). */
  reload: () => Promise<void>;
}

/**
 * `enabled` gates fetching on the signed-in state; `onUnauthorized` fires when a listing
 * comes back 401 (the session lapsed) so the panel can fall back to its sign-in view.
 */
export function useFolderBrowser(enabled: boolean, onUnauthorized: () => void): FolderBrowser {
  const [currentFolderId, setCurrentFolderId] = useState<string | null>(null);
  const [breadcrumb, setBreadcrumb] = useState<LibraryFolder[]>([]);
  const [folders, setFolders] = useState<LibraryFolder[]>([]);
  const [scenes, setScenes] = useState<LibraryScene[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Monotonic request id: rapid navigate() calls put several `load`s in flight, and they can
  // resolve out of order. Only the most recent request is allowed to commit state, so a slow
  // earlier listing can't overwrite the folder the user actually navigated to (ASYNC-3).
  const requestSeq = useRef(0);

  const load = useCallback(
    async (folderId: string | null, signal?: AbortSignal) => {
      const seq = ++requestSeq.current;
      setLoading(true);
      setError(null);
      try {
        const [listing, sceneList] = await Promise.all([
          listFolders(folderId, signal),
          listScenes(folderId, signal),
        ]);
        if (seq !== requestSeq.current) {
          return; // superseded by a later navigation
        }
        if (listing === null || sceneList === null) {
          onUnauthorized();
          return;
        }
        setFolders(listing.folders);
        setBreadcrumb(listing.breadcrumb);
        setScenes(sceneList);
      } catch {
        if (signal?.aborted || seq !== requestSeq.current) {
          return; // unmounted/torn down, or superseded by a later navigation
        }
        // A folder removed out from under us (or any fetch failure): fall back to the root
        // rather than stranding the panel on a dead folder.
        if (folderId !== null) {
          setCurrentFolderId(null);
        } else {
          setError("Could not reach the server.");
        }
      } finally {
        if (!signal?.aborted && seq === requestSeq.current) {
          setLoading(false);
        }
      }
    },
    [onUnauthorized],
  );

  // (Re)load when signed in or the current folder changes; clear when signed out. The
  // AbortController ties the in-flight listing to this effect run, so navigating away or
  // unmounting aborts the fetch instead of letting a stale result settle later.
  useEffect(() => {
    if (!enabled) {
      setFolders([]);
      setScenes([]);
      setBreadcrumb([]);
      setCurrentFolderId(null);
      return;
    }
    const controller = new AbortController();
    void load(currentFolderId, controller.signal);
    return () => controller.abort();
  }, [enabled, currentFolderId, load]);

  const navigate = useCallback((folderId: string | null) => setCurrentFolderId(folderId), []);
  const reload = useCallback(() => load(currentFolderId), [load, currentFolderId]);

  return { currentFolderId, breadcrumb, folders, scenes, loading, error, navigate, reload };
}
