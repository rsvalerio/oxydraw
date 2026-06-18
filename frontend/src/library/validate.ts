// Runtime shape checks for backend JSON at the trust boundary.
//
// The REST client casts `response.json()` to typed interfaces, but `as` is a compile-time
// fiction: at runtime the parsed value is whatever the backend sent. These validators turn
// contract drift (a renamed field, a null where a string was expected) into one clear error
// at the fetch boundary instead of a downstream crash or "Invalid Date" far from the cause.

import type { FolderListing, LibraryFolder, LibraryScene, Me, Providers } from "./api";

/** Raised when a backend response does not match its expected shape. */
export class ResponseShapeError extends Error {
  constructor(endpoint: string, detail: string) {
    super(`${endpoint}: malformed response (${detail})`);
    this.name = "ResponseShapeError";
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isStringOrNull(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

export function asProviders(endpoint: string, value: unknown): Providers {
  if (!isRecord(value) || !Array.isArray(value.providers) || typeof value.password !== "boolean") {
    throw new ResponseShapeError(endpoint, "expected { providers: string[], password: boolean }");
  }
  if (!value.providers.every(isString)) {
    throw new ResponseShapeError(endpoint, "providers must be strings");
  }
  return { providers: value.providers, password: value.password };
}

export function asMe(endpoint: string, value: unknown): Me {
  if (!isRecord(value) || !isRecord(value.user) || !isRecord(value.org)) {
    throw new ResponseShapeError(endpoint, "expected { user, org }");
  }
  const { user, org } = value;
  if (!isStringOrNull(user.name) || !isStringOrNull(user.email) || !isStringOrNull(user.avatar_url)) {
    throw new ResponseShapeError(endpoint, "user fields must be string|null");
  }
  if (!isString(org.id) || !isString(org.name)) {
    throw new ResponseShapeError(endpoint, "org fields must be strings");
  }
  return {
    user: { name: user.name, email: user.email, avatar_url: user.avatar_url },
    org: { id: org.id, name: org.name },
  };
}

function asLibraryScene(endpoint: string, value: unknown): LibraryScene {
  if (
    !isRecord(value) ||
    !isString(value.id) ||
    !isString(value.name) ||
    !isString(value.document_id) ||
    !isString(value.key) ||
    !isString(value.updated_at)
  ) {
    throw new ResponseShapeError(endpoint, "scene missing string field(s)");
  }
  return {
    id: value.id,
    name: value.name,
    document_id: value.document_id,
    key: value.key,
    updated_at: value.updated_at,
  };
}

export function asLibraryScenes(endpoint: string, value: unknown): LibraryScene[] {
  if (!Array.isArray(value)) {
    throw new ResponseShapeError(endpoint, "expected an array of scenes");
  }
  return value.map((scene) => asLibraryScene(endpoint, scene));
}

function asLibraryFolder(endpoint: string, value: unknown): LibraryFolder {
  if (
    !isRecord(value) ||
    !isString(value.id) ||
    !isString(value.name) ||
    !isStringOrNull(value.parent_id) ||
    !isString(value.updated_at)
  ) {
    throw new ResponseShapeError(endpoint, "folder missing field(s)");
  }
  return {
    id: value.id,
    name: value.name,
    parent_id: value.parent_id,
    updated_at: value.updated_at,
  };
}

export function asFolderListing(endpoint: string, value: unknown): FolderListing {
  if (!isRecord(value) || !Array.isArray(value.folders) || !Array.isArray(value.breadcrumb)) {
    throw new ResponseShapeError(endpoint, "expected { folders: [], breadcrumb: [] }");
  }
  return {
    folders: value.folders.map((folder) => asLibraryFolder(endpoint, folder)),
    breadcrumb: value.breadcrumb.map((folder) => asLibraryFolder(endpoint, folder)),
  };
}
