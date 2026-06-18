// A small modal that browses the folder tree to pick a move destination. Reuses
// `listFolders` to descend level by level; "Move here" picks the folder currently shown
// (the root when at the top). Rendered above the library panel.

import { useEffect, useState } from "react";

import { listFolders, type LibraryFolder } from "./api";
import Breadcrumb from "./Breadcrumb";
import ModalDialog from "./ModalDialog";

export default function MovePicker({
  title,
  excludeId,
  onPick,
  onCancel,
}: {
  /** What is being moved, e.g. `Move "diagram" to…`. */
  title: string;
  /** A folder being moved cannot be its own destination — hidden from the list. */
  excludeId?: string;
  /** Chosen destination; `null` is the root. */
  onPick: (destFolderId: string | null) => void;
  onCancel: () => void;
}) {
  const [folderId, setFolderId] = useState<string | null>(null);
  const [breadcrumb, setBreadcrumb] = useState<LibraryFolder[]>([]);
  const [folders, setFolders] = useState<LibraryFolder[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const controller = new AbortController();
    setLoading(true);
    void listFolders(folderId, controller.signal)
      .then((listing) => {
        if (cancelled || !listing) {
          return;
        }
        setFolders(listing.folders.filter((f) => f.id !== excludeId));
        setBreadcrumb(listing.breadcrumb);
      })
      .catch(() => {
        // Aborted on teardown or a failed listing: leave the prior state, don't crash.
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
      controller.abort();
    };
  }, [folderId, excludeId]);

  return (
    <ModalDialog title={title} className="lib-move" onClose={onCancel}>
      <div className="lib-body">
        <Breadcrumb folders={breadcrumb} onNavigate={setFolderId} />
        {loading ? (
          <p className="lib-message">Loading…</p>
        ) : folders.length === 0 ? (
          <p className="lib-message">No subfolders here.</p>
        ) : (
          <ul className="lib-items">
            {folders.map((folder) => (
              <li key={folder.id} className="lib-item">
                <button
                  type="button"
                  className="lib-item-main"
                  onClick={() => setFolderId(folder.id)}
                >
                  <span className="lib-item-icon" aria-hidden="true">
                    📁
                  </span>
                  <span className="lib-item-name">{folder.name}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
      <footer className="lib-status lib-move-actions">
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className="lib-primary" onClick={() => onPick(folderId)}>
          {breadcrumb.length
            ? `Move to “${breadcrumb[breadcrumb.length - 1].name}”`
            : "Move to Library"}
        </button>
      </footer>
    </ModalDialog>
  );
}
