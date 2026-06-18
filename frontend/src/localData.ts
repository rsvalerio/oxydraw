// Debounced local persistence to localStorage, so an accidental reload doesn't lose work.
// Intentionally minimal compared to upstream's IndexedDB `LocalData`; scenes that need to
// outlive the browser go through share links (or, later, collaboration rooms).
//
// We reuse the package's `serializeAsJSON` for the write so transient appState (selection,
// collaborators, cursors, …) is stripped exactly as the editor expects.

import { serializeAsJSON } from "@excalidraw/excalidraw";
import type { ExcalidrawElement } from "@excalidraw/excalidraw/element/types";
import type { AppState, BinaryFiles } from "@excalidraw/excalidraw/types";

import type { RestoredScene } from "./share";

const SCENE_KEY = "oxydraw:scene";
const SAVE_DEBOUNCE_MS = 300;

let saveTimer: ReturnType<typeof setTimeout> | undefined;

/** Persist the scene, coalescing rapid `onChange` bursts into one write. */
export function saveToLocalStorage(
  elements: readonly ExcalidrawElement[],
  appState: Partial<AppState>,
  files: BinaryFiles,
): void {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    try {
      localStorage.setItem(
        SCENE_KEY,
        serializeAsJSON(elements, appState, files, "local"),
      );
    } catch {
      // Quota or private-mode failures are non-fatal: the scene stays in memory.
    }
  }, SAVE_DEBOUNCE_MS);
}

/** Load a previously persisted scene, or `null` when nothing valid is stored. */
export function loadFromLocalStorage(): RestoredScene | null {
  try {
    const raw = localStorage.getItem(SCENE_KEY);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw) as {
      elements?: readonly ExcalidrawElement[];
      appState?: Partial<AppState>;
      files?: BinaryFiles;
    };
    return {
      elements: parsed.elements ?? [],
      appState: parsed.appState ?? {},
      files: parsed.files ?? ({} as BinaryFiles),
    };
  } catch {
    return null;
  }
}
