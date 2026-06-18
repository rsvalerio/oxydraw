// Mutation state + handlers for the signed-in library view: scene save, create-folder,
// inline rename, move, and delete. Kept in a hook (rather than inline in LibraryBrowser) so
// the view stays presentational and the shared busy/status plumbing lives in one place.

import { useCallback, useState } from "react";
import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";

import {
  createFolder,
  deleteFolder,
  deleteScene,
  moveFolder,
  moveScene,
  renameFolder,
  renameScene,
  type LibraryScene,
} from "./api";
import type { AuthState } from "./auth";
import { openLibraryScene, saveSceneToLibrary } from "./scene";
import type { FolderBrowser } from "./useFolderBrowser";

/** What an inline rename or the move picker is currently acting on. */
export type ItemRef = { kind: "folder" | "scene"; id: string; name: string };

export function useLibraryActions(
  api: ExcalidrawImperativeAPI,
  browser: FolderBrowser,
  auth: AuthState,
  onClose: () => void,
) {
  const [name, setName] = useState("");
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const [newFolderName, setNewFolderName] = useState("");
  const [newFolderOpen, setNewFolderOpen] = useState(false);
  const [editing, setEditing] = useState<ItemRef | null>(null);
  const [editName, setEditName] = useState("");
  const [moving, setMoving] = useState<ItemRef | null>(null);

  const handleSave = useCallback(async () => {
    setBusy(true);
    setStatus("Saving…");
    try {
      const result = await saveSceneToLibrary(
        api,
        name.trim() || `Scene ${new Date().toLocaleString()}`,
        browser.currentFolderId,
      );
      if (result === "empty") {
        setStatus("Nothing to save: the canvas is empty.");
      } else if (result === "unauthorized") {
        await auth.refresh();
      } else {
        setName("");
        setStatus("Saved.");
        await browser.reload();
      }
    } catch {
      setStatus("Save failed.");
    } finally {
      setBusy(false);
    }
  }, [api, name, auth, browser]);

  const handleCreateFolder = useCallback(async () => {
    const folderName = newFolderName.trim();
    if (!folderName) {
      return;
    }
    setBusy(true);
    try {
      if (await createFolder(folderName, browser.currentFolderId)) {
        setNewFolderName("");
        setNewFolderOpen(false);
        await browser.reload();
      } else {
        setStatus("Could not create the folder.");
      }
    } finally {
      setBusy(false);
    }
  }, [newFolderName, browser]);

  const handleOpenScene = useCallback(
    async (scene: LibraryScene) => {
      try {
        await openLibraryScene(api, scene.document_id, scene.key);
        onClose();
      } catch {
        setStatus("Could not open that scene.");
      }
    },
    [api, onClose],
  );

  const commitRename = useCallback(async () => {
    if (!editing) {
      return;
    }
    const trimmed = editName.trim();
    if (!trimmed || trimmed === editing.name) {
      setEditing(null);
      return;
    }
    setBusy(true);
    try {
      const ok =
        editing.kind === "folder"
          ? await renameFolder(editing.id, trimmed)
          : await renameScene(editing.id, trimmed);
      if (ok) {
        await browser.reload();
      } else {
        setStatus("Rename failed.");
      }
    } finally {
      setBusy(false);
      setEditing(null);
    }
  }, [editing, editName, browser]);

  const handleDelete = useCallback(
    async (item: ItemRef) => {
      const message =
        item.kind === "folder"
          ? `Delete the folder “${item.name}” and everything inside it (including its scenes)? This cannot be undone.`
          : `Delete the scene “${item.name}”? This cannot be undone.`;
      if (!window.confirm(message)) {
        return;
      }
      setBusy(true);
      try {
        const ok =
          item.kind === "folder" ? await deleteFolder(item.id) : await deleteScene(item.id);
        if (ok) {
          await browser.reload();
        } else {
          setStatus("Delete failed.");
        }
      } finally {
        setBusy(false);
      }
    },
    [browser],
  );

  const handleMovePick = useCallback(
    async (dest: string | null) => {
      if (!moving) {
        return;
      }
      const item = moving;
      setMoving(null);
      setBusy(true);
      try {
        const ok =
          item.kind === "folder" ? await moveFolder(item.id, dest) : await moveScene(item.id, dest);
        if (ok) {
          await browser.reload();
        } else {
          setStatus(
            item.kind === "folder"
              ? "Could not move there (a folder can't go inside itself)."
              : "Move failed.",
          );
        }
      } finally {
        setBusy(false);
      }
    },
    [moving, browser],
  );

  const startRename = useCallback((item: ItemRef) => {
    setEditing(item);
    setEditName(item.name);
  }, []);

  const isEditing = (kind: "folder" | "scene", id: string) =>
    editing?.kind === kind && editing.id === id;

  return {
    name,
    setName,
    status,
    busy,
    newFolderName,
    setNewFolderName,
    newFolderOpen,
    setNewFolderOpen,
    editName,
    setEditName,
    moving,
    setMoving,
    setEditing,
    isEditing,
    handleSave,
    handleCreateFolder,
    handleOpenScene,
    commitRename,
    handleDelete,
    handleMovePick,
    startRename,
  };
}
