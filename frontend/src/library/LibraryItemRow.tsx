// A single folder or scene row in the library listing. Shows an icon + name (and, for
// scenes, a timestamp) with per-row Rename / Move / Delete actions; while being renamed it
// swaps to an inline editor. Pure presentation — every action is delegated to the browser.

import type { ItemRef } from "./useLibraryActions";

export default function LibraryItemRow({
  item,
  icon,
  subtitle,
  onOpen,
  editing,
  editName,
  onEditNameChange,
  onCommitRename,
  onCancelRename,
  onStartRename,
  onMove,
  onDelete,
}: {
  item: ItemRef;
  /** Leading glyph (📁 for folders, 🖼️ for scenes). */
  icon: string;
  /** Optional trailing text, e.g. a scene's last-updated time. */
  subtitle?: string;
  /** Open the folder/scene (the row's primary action). */
  onOpen: () => void;
  /** Whether this row is currently in inline-rename mode. */
  editing: boolean;
  editName: string;
  onEditNameChange: (value: string) => void;
  onCommitRename: () => void;
  onCancelRename: () => void;
  onStartRename: (item: ItemRef) => void;
  onMove: (item: ItemRef) => void;
  onDelete: (item: ItemRef) => void;
}) {
  if (editing) {
    return (
      <li className="lib-item">
        <span className="lib-rename">
          <input
            type="text"
            autoFocus
            value={editName}
            onChange={(e) => onEditNameChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                onCommitRename();
              } else if (e.key === "Escape") {
                onCancelRename();
              }
            }}
          />
          <button type="button" onClick={onCommitRename}>
            Save
          </button>
          <button type="button" onClick={onCancelRename}>
            Cancel
          </button>
        </span>
      </li>
    );
  }

  return (
    <li className="lib-item">
      <button type="button" className="lib-item-main" onClick={onOpen}>
        <span className="lib-item-icon" aria-hidden="true">
          {icon}
        </span>
        <span className="lib-item-name">{item.name}</span>
        {subtitle && <span className="lib-item-date">{subtitle}</span>}
      </button>
      <span className="lib-item-actions">
        <button
          type="button"
          title="Rename"
          aria-label={`Rename ${item.name}`}
          onClick={() => onStartRename(item)}
        >
          ✏️
        </button>
        <button
          type="button"
          title="Move"
          aria-label={`Move ${item.name}`}
          onClick={() => onMove(item)}
        >
          ↪️
        </button>
        <button
          type="button"
          title="Delete"
          aria-label={`Delete ${item.name}`}
          onClick={() => onDelete(item)}
        >
          🗑️
        </button>
      </span>
    </li>
  );
}
