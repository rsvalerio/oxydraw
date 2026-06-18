// End-to-end encryption primitives for share links (and, later, collaboration frames).
//
// The server only ever stores opaque ciphertext: the AES key lives in the URL fragment and
// is never sent to the backend, matching Excalidraw's capability-URL share semantics. We use
// AES-GCM via the Web Crypto API (no dependencies) and a fresh random IV per payload.

const IV_BYTES = 12; // AES-GCM standard nonce length.
const KEY_BITS = 128;

/** Generate a fresh extractable AES-GCM key for a new share/room. */
export async function generateKey(): Promise<CryptoKey> {
  return crypto.subtle.generateKey({ name: "AES-GCM", length: KEY_BITS }, true, [
    "encrypt",
    "decrypt",
  ]);
}

/** Export a key as a URL-safe base64 string for embedding in the URL fragment. */
export async function exportKey(key: CryptoKey): Promise<string> {
  const raw = await crypto.subtle.exportKey("raw", key);
  return bytesToBase64Url(new Uint8Array(raw));
}

/** Re-import a key previously produced by {@link exportKey}. */
export async function importKey(encoded: string): Promise<CryptoKey> {
  const raw = base64UrlToBytes(encoded);
  return crypto.subtle.importKey("raw", raw, { name: "AES-GCM" }, true, [
    "encrypt",
    "decrypt",
  ]);
}

/**
 * Encrypt `plaintext` and return a single buffer of `iv || ciphertext`, ready to POST as the
 * opaque share body. Keeping the IV prefixed means the stored blob is self-describing — the
 * reader needs only the key from the fragment.
 */
/**
 * Encrypt and return the IV and ciphertext separately. The collaboration relay forwards a
 * broadcast frame's `(data, iv)` as two distinct binary args, so the wire protocol needs them
 * unbundled; share links bundle them via {@link encrypt}.
 */
export async function encryptParts(
  key: CryptoKey,
  plaintext: BufferSource,
): Promise<{ iv: Uint8Array<ArrayBuffer>; ciphertext: Uint8Array<ArrayBuffer> }> {
  const iv = crypto.getRandomValues(new Uint8Array(IV_BYTES));
  const ciphertext = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, plaintext);
  return { iv, ciphertext: new Uint8Array(ciphertext) };
}

/** Inverse of {@link encryptParts}. */
export async function decryptParts(
  key: CryptoKey,
  iv: BufferSource,
  ciphertext: BufferSource,
): Promise<Uint8Array<ArrayBuffer>> {
  const plaintext = await crypto.subtle.decrypt({ name: "AES-GCM", iv }, key, ciphertext);
  return new Uint8Array(plaintext);
}

/**
 * Encrypt `plaintext` into a single self-describing `iv || ciphertext` buffer, ready to POST as
 * an opaque share body — the reader needs only the key from the URL fragment.
 */
export async function encrypt(
  key: CryptoKey,
  plaintext: BufferSource,
): Promise<Uint8Array<ArrayBuffer>> {
  const { iv, ciphertext } = await encryptParts(key, plaintext);
  const out = new Uint8Array(iv.length + ciphertext.length);
  out.set(iv, 0);
  out.set(ciphertext, iv.length);
  return out;
}

/** Inverse of {@link encrypt}: split the IV prefix, decrypt the remainder. */
export async function decrypt(
  key: CryptoKey,
  blob: Uint8Array<ArrayBuffer>,
): Promise<Uint8Array<ArrayBuffer>> {
  return decryptParts(key, blob.subarray(0, IV_BYTES), blob.subarray(IV_BYTES));
}

function bytesToBase64Url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function base64UrlToBytes(encoded: string): Uint8Array<ArrayBuffer> {
  const base64 = encoded.replace(/-/g, "+").replace(/_/g, "/");
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}
