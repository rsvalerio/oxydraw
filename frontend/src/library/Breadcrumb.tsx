// Root-first breadcrumb trail shared by the library browser and the move picker: a
// "Library" root crumb followed by one button per ancestor folder. The parent decides what
// navigating to a crumb does (browse vs. pick a destination).

import type { LibraryFolder } from "./api";

export default function Breadcrumb({
  folders,
  onNavigate,
}: {
  /** Ancestor chain to the current folder, root-first; empty at the root. */
  folders: LibraryFolder[];
  /** Navigate to a folder, or the root when `null`. */
  onNavigate: (folderId: string | null) => void;
}) {
  return (
    <nav className="lib-breadcrumb">
      <button type="button" onClick={() => onNavigate(null)}>
        Library
      </button>
      {folders.map((folder) => (
        <span key={folder.id}>
          <span className="lib-crumb-sep">/</span>
          <button type="button" onClick={() => onNavigate(folder.id)}>
            {folder.name}
          </button>
        </span>
      ))}
    </nav>
  );
}
