// Shared parser for capability-URL fragments of the form `#<prefix>=<a>,<b>`.
//
// Both the share-link flow (`#json=<id>,<key>`) and the collaboration flow (`#room=<roomId>,<key>`)
// carry a document/room id plus its AES key in the URL fragment, comma-separated. Keeping the
// parsing rule in one place means a hardening change (length bounds, trimming, rejecting extra
// commas) applies to both forms instead of drifting between two copies.

/**
 * Parse a `#prefix=<a>,<b>` fragment. `prefix` includes the leading `#` (e.g. `"#json="`).
 * Returns `[a, b]` only when the fragment matches the prefix and both parts are non-empty;
 * otherwise `null`.
 */
export function parseFragment(prefix: string, hash: string): [string, string] | null {
  if (!hash.startsWith(prefix)) {
    return null;
  }
  const [a, b] = hash.slice(prefix.length).split(",");
  if (!a || !b) {
    return null;
  }
  return [a, b];
}

/** Fragment prefix carrying a stored share document id + its AES key (`#json=<id>,<key>`). */
export const SHARE_FRAGMENT_PREFIX = "#json=";

/**
 * Parse `#json=<id>,<key>` from a location hash, or `null` if absent/malformed. Lives here, free of
 * the editor bundle, so this security-critical parser of untrusted URL input stays independently
 * unit-testable (the share-link flow in `share.ts` pulls in `@excalidraw/excalidraw`, which touches
 * `window` at load time). `share.ts` re-exports it as the share module's public API.
 */
export function parseShareFragment(
  hash: string = window.location.hash,
): { id: string; key: string } | null {
  const parts = parseFragment(SHARE_FRAGMENT_PREFIX, hash);
  return parts ? { id: parts[0], key: parts[1] } : null;
}
