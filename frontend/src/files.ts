// Scene-image file transfer against the backend's clean `/api/files/{id}` endpoints.
//
// Excalidraw addresses each binary file by a content hash (`BinaryFileData.id`), so uploads are
// idempotent and peers converge on the same id with no coordination. The bytes are opaque to
// the server (the editor encrypts image data client-side in a collaboration room).

import { FILE_URL } from "./config";
import { fetchWithTimeout } from "./http";

/** Upload (or overwrite) a file blob under its content-addressed id. */
export async function uploadFile(
  id: string,
  contentType: string,
  data: BufferSource,
): Promise<void> {
  const response = await fetchWithTimeout(`${FILE_URL}${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: { "Content-Type": contentType },
    body: data,
  });
  if (!response.ok) {
    throw new Error(`file upload failed: ${response.status}`);
  }
}

/** Fetch a file blob by id. Throws on 404 / network error. */
export async function fetchFile(
  id: string,
): Promise<{ data: Uint8Array<ArrayBuffer>; contentType: string }> {
  const response = await fetchWithTimeout(`${FILE_URL}${encodeURIComponent(id)}`);
  if (!response.ok) {
    throw new Error(`file download failed: ${response.status}`);
  }
  return {
    data: new Uint8Array(await response.arrayBuffer()),
    contentType: response.headers.get("content-type") ?? "application/octet-stream",
  };
}
