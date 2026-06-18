// The signed-in library view: folder breadcrumb + "+ Folder", a save bar, and the listing
// of folders/scenes with per-row rename/move/delete. Presentation only — the save/create/
// rename/move/delete state and handlers live in `useLibraryActions`.

import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";

import { type LibraryFolder, type LibraryScene } from "./api";
import type { AuthState } from "./auth";
import Breadcrumb from "./Breadcrumb";
import LibraryItemRow from "./LibraryItemRow";
import MovePicker from "./MovePicker";
import { useLibraryActions, type ItemRef } from "./useLibraryActions";
import type { FolderBrowser } from "./useFolderBrowser";

export default function LibraryBrowser({
  api,
  browser,
  auth,
  onClose,
}: {
  api: ExcalidrawImperativeAPI;
  browser: FolderBrowser;
  auth: AuthState;
  onClose: () => void;
}) {
  const actions = useLibraryActions(api, browser, auth, onClose);
  const isEmpty =
    !browser.loading && browser.folders.length === 0 && browser.scenes.length === 0;

  return (
    <>
      <div className="lib-body">
        <div className="lib-toolbar">
          <Breadcrumb folders={browser.breadcrumb} onNavigate={browser.navigate} />
          <button type="button" onClick={() => actions.setNewFolderOpen((open) => !open)}>
            + Folder
          </button>
        </div>

        {actions.newFolderOpen && (
          <div className="lib-save">
            <input
              type="text"
              placeholder="Folder name"
              autoFocus
              value={actions.newFolderName}
              onChange={(e) => actions.setNewFolderName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  void actions.handleCreateFolder();
                } else if (e.key === "Escape") {
                  actions.setNewFolderOpen(false);
                }
              }}
            />
            <button
              type="button"
              onClick={() => void actions.handleCreateFolder()}
              disabled={actions.busy}
            >
              Create
            </button>
          </div>
        )}

        <div className="lib-save">
          <input
            type="text"
            placeholder="Scene name"
            value={actions.name}
            onChange={(e) => actions.setName(e.target.value)}
          />
          <button type="button" onClick={actions.handleSave} disabled={actions.busy}>
            Save here
          </button>
        </div>

        {browser.loading ? (
          <p className="lib-message">Loading…</p>
        ) : isEmpty ? (
          <p className="lib-message">This folder is empty.</p>
        ) : (
          <ul className="lib-items">
            {browser.folders.map((folder: LibraryFolder) => {
              const item: ItemRef = { kind: "folder", id: folder.id, name: folder.name };
              return (
                <LibraryItemRow
                  key={`f-${folder.id}`}
                  item={item}
                  icon="📁"
                  onOpen={() => browser.navigate(folder.id)}
                  editing={actions.isEditing("folder", folder.id)}
                  editName={actions.editName}
                  onEditNameChange={actions.setEditName}
                  onCommitRename={() => void actions.commitRename()}
                  onCancelRename={() => actions.setEditing(null)}
                  onStartRename={actions.startRename}
                  onMove={actions.setMoving}
                  onDelete={(target) => void actions.handleDelete(target)}
                />
              );
            })}
            {browser.scenes.map((scene: LibraryScene) => {
              const item: ItemRef = { kind: "scene", id: scene.id, name: scene.name };
              return (
                <LibraryItemRow
                  key={`s-${scene.id}`}
                  item={item}
                  icon="🖼️"
                  subtitle={new Date(scene.updated_at).toLocaleString()}
                  onOpen={() => void actions.handleOpenScene(scene)}
                  editing={actions.isEditing("scene", scene.id)}
                  editName={actions.editName}
                  onEditNameChange={actions.setEditName}
                  onCommitRename={() => void actions.commitRename()}
                  onCancelRename={() => actions.setEditing(null)}
                  onStartRename={actions.startRename}
                  onMove={actions.setMoving}
                  onDelete={(target) => void actions.handleDelete(target)}
                />
              );
            })}
          </ul>
        )}
      </div>

      {(actions.status || browser.error) && (
        <footer className="lib-status">{actions.status || browser.error}</footer>
      )}

      {actions.moving && (
        <MovePicker
          title={`Move “${actions.moving.name}” to…`}
          excludeId={actions.moving.kind === "folder" ? actions.moving.id : undefined}
          onPick={(dest) => void actions.handleMovePick(dest)}
          onCancel={() => actions.setMoving(null)}
        />
      )}
    </>
  );
}
