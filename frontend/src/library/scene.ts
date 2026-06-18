// Save the live scene to the library and open a saved one back onto the canvas.
//
// Both ride the existing share flow: a library scene *is* a share blob (images embedded,
// end-to-end-encrypted) plus a name recorded against it. No separate file upload — unlike the
// old overlay, which had to mirror upstream's by-reference file storage.

import { CaptureUpdateAction } from "@excalidraw/excalidraw";
import type { OrderedExcalidrawElement } from "@excalidraw/excalidraw/element/types";
import type { BinaryFileData, ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";

import { importFromShareLink, uploadScene } from "../share";
import { createScene } from "./api";

export type SaveResult = "saved" | "empty" | "unauthorized";

/** Upload the current scene and record it in the library under `name`, in `folderId`
 * (the root when `null`). */
export async function saveSceneToLibrary(
  api: ExcalidrawImperativeAPI,
  name: string,
  folderId: string | null,
): Promise<SaveResult> {
  const elements = api.getSceneElements();
  if (!elements.length) {
    return "empty";
  }
  const { id, key } = await uploadScene(elements, api.getAppState(), api.getFiles());
  const ok = await createScene(name, id, key, folderId);
  return ok ? "saved" : "unauthorized";
}

/** Fetch, decrypt, and load a saved scene onto the canvas. */
export async function openLibraryScene(
  api: ExcalidrawImperativeAPI,
  documentId: string,
  key: string,
): Promise<void> {
  const scene = await importFromShareLink(documentId, key);
  api.updateScene({
    elements: scene.elements as readonly OrderedExcalidrawElement[],
    captureUpdate: CaptureUpdateAction.IMMEDIATELY,
  });
  const files = Object.values(scene.files) as BinaryFileData[];
  if (files.length) {
    api.addFiles(files);
  }
  api.scrollToContent(scene.elements as readonly OrderedExcalidrawElement[], {
    fitToContent: true,
  });
}
