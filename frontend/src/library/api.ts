// Client for the scene-library backend (`/api/ext/*`): auth + saved-scene metadata. The Rust
// routes are unchanged from the old overlay; only the UI moved into the React app.

import { fetchWithTimeout } from "../http";
import { asFolderListing, asLibraryScenes, asMe, asProviders } from "./validate";

export interface Providers {
  providers: string[];
  password: boolean;
}

export interface Me {
  user: { name: string | null; email: string | null; avatar_url: string | null };
  org: { id: string; name: string };
}

export interface LibraryScene {
  id: string;
  name: string;
  document_id: string;
  key: string;
  updated_at: string;
}

export interface LibraryFolder {
  id: string;
  name: string;
  /** `null` for a top-level folder. */
  parent_id: string | null;
  updated_at: string;
}

/** A folder level's child folders plus the breadcrumb from the root down to it. */
export interface FolderListing {
  folders: LibraryFolder[];
  /** Root-first chain to the folder being viewed; empty at the root. */
  breadcrumb: LibraryFolder[];
}

/**
 * Send a mutating request (POST/PATCH/DELETE) with an optional JSON body and return whether
 * it succeeded (`response.ok`). Centralizes the `content-type` header and body serialization
 * so cross-cutting concerns (timeouts, future CSRF headers, error logging) land in one place
 * rather than being duplicated across every mutator. A `DELETE` with no body sends no header.
 */
async function mutate(method: string, path: string, body?: unknown): Promise<boolean> {
  const init: RequestInit = { method };
  if (body !== undefined) {
    init.headers = { "content-type": "application/json" };
    init.body = JSON.stringify(body);
  }
  const response = await fetchWithTimeout(path, init);
  return response.ok;
}

/** Configured sign-in capabilities. Empty providers + no password = open mode. */
export async function fetchProviders(): Promise<Providers> {
  const response = await fetchWithTimeout("/api/ext/auth/providers");
  if (!response.ok) {
    return { providers: [], password: false };
  }
  return asProviders("/api/ext/auth/providers", await response.json());
}

/** Current principal, or `null` when sign-in is required (401). */
export async function fetchMe(): Promise<Me | null> {
  const response = await fetchWithTimeout("/api/ext/me");
  if (response.status === 401) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`/api/ext/me failed: ${response.status}`);
  }
  return asMe("/api/ext/me", await response.json());
}

/** Attempt a shared-password login. Returns whether it succeeded. */
export async function login(password: string): Promise<boolean> {
  return mutate("POST", "/api/ext/login", { password });
}

export async function logout(): Promise<void> {
  const response = await fetchWithTimeout("/api/ext/logout", { method: "POST" });
  if (!response.ok) {
    // A failed logout leaves the server-side session intact while the UI clears local state;
    // log it so the discrepancy is diagnosable rather than silently dropped.
    console.error(`logout failed: ${response.status}`);
  }
}

/**
 * Saved scenes directly inside `folderId` (or the root when `null`), newest first, or
 * `null` when sign-in is required (401).
 */
export async function listScenes(
  folderId: string | null,
  signal?: AbortSignal,
): Promise<LibraryScene[] | null> {
  const query = folderId ? `?folder=${encodeURIComponent(folderId)}` : "";
  const response = await fetchWithTimeout(`/api/ext/scenes${query}`, { signal });
  if (response.status === 401) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`/api/ext/scenes failed: ${response.status}`);
  }
  return asLibraryScenes("/api/ext/scenes", await response.json());
}

/**
 * Record a saved scene's `{name, document_id, key}` in `folderId` (root when `null`).
 * Returns false when sign-in is required.
 */
export async function createScene(
  name: string,
  documentId: string,
  key: string,
  folderId: string | null,
): Promise<boolean> {
  return mutate("POST", "/api/ext/scenes", {
    name,
    document_id: documentId,
    key,
    folder_id: folderId,
  });
}

/** Rename a saved scene. Returns whether it succeeded. */
export async function renameScene(id: string, name: string): Promise<boolean> {
  return mutate("PATCH", `/api/ext/scenes/${encodeURIComponent(id)}`, { name });
}

/** Move a scene into `folderId` (root when `null`). Returns whether it succeeded. */
export async function moveScene(id: string, folderId: string | null): Promise<boolean> {
  return mutate("PATCH", `/api/ext/scenes/${encodeURIComponent(id)}`, { folder_id: folderId });
}

/** Delete a saved scene's library entry. Returns whether it succeeded. */
export async function deleteScene(id: string): Promise<boolean> {
  return mutate("DELETE", `/api/ext/scenes/${encodeURIComponent(id)}`);
}

/**
 * Child folders of `parentId` (or the top level when `null`) plus the breadcrumb, or
 * `null` when sign-in is required (401).
 */
export async function listFolders(
  parentId: string | null,
  signal?: AbortSignal,
): Promise<FolderListing | null> {
  const query = parentId ? `?parent=${encodeURIComponent(parentId)}` : "";
  const response = await fetchWithTimeout(`/api/ext/folders${query}`, { signal });
  if (response.status === 401) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`/api/ext/folders failed: ${response.status}`);
  }
  return asFolderListing("/api/ext/folders", await response.json());
}

/** Create a folder under `parentId` (top level when `null`). Returns whether it succeeded. */
export async function createFolder(name: string, parentId: string | null): Promise<boolean> {
  return mutate("POST", "/api/ext/folders", { name, parent_id: parentId });
}

/** Rename a folder. Returns whether it succeeded. */
export async function renameFolder(id: string, name: string): Promise<boolean> {
  return mutate("PATCH", `/api/ext/folders/${encodeURIComponent(id)}`, { name });
}

/**
 * Move a folder under `parentId` (root when `null`). Returns whether it succeeded — the
 * backend rejects a move that would create a cycle (409) or nest too deep (422).
 */
export async function moveFolder(id: string, parentId: string | null): Promise<boolean> {
  return mutate("PATCH", `/api/ext/folders/${encodeURIComponent(id)}`, { parent_id: parentId });
}

/** Delete a folder and its whole subtree (descendant folders and their scenes). */
export async function deleteFolder(id: string): Promise<boolean> {
  return mutate("DELETE", `/api/ext/folders/${encodeURIComponent(id)}`);
}

/** Top-level navigation target that starts an OAuth sign-in flow. */
export function oauthStartUrl(provider: string): string {
  return `/api/ext/auth/${provider}`;
}
