---
id: TASK-0134
title: 'SEC-12: REST API responses are cast to typed shapes without runtime validation'
status: Done
assignee:
  - TASK-0141
created_date: '2026-06-16 19:01'
updated_date: '2026-06-16 19:59'
labels:
  - code-review-web
  - security
dependencies: []
priority: low
ordinal: 134000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/library/api.ts:43,55,85,145` (and `share.ts:44,89`, `localData.ts:45`)

**What**: Every backend response is fed through `(await response.json()) as T` with no runtime shape check — `fetchProviders` → `Providers`, `fetchMe` → `Me`, `listScenes` → `LibraryScene[]`, `listFolders` → `FolderListing`, plus the share/localStorage parse paths. The `as` cast is a compile-time fiction; at runtime the parsed value is whatever JSON came back.

**Why it matters**: This is a separate trust surface from the collab broadcast path (RT-1 / TASK-0125). When the backend contract drifts (a field renamed, a null where a string was assumed), the type lie propagates silently: `scene.updated_at` → `new Date(undefined)` renders "Invalid Date", a missing `folders` array throws deep in `.map`, etc. — all far from the fetch, with no actionable error. A minimal validator (or at least defensive guards on the fields actually read) turns contract drift into one clear failure at the boundary.

<!-- scan confidence: candidates to inspect — same-origin trusted backend, so security impact is low; primary value is robustness against contract drift -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 REST responses whose fields are consumed (Me, Providers, LibraryScene[], FolderListing) are validated at the boundary, or the consuming code guards the fields it reads
- [x] #2 A malformed/partial response yields one clear boundary-level error rather than a downstream crash or 'Invalid Date'
<!-- AC:END -->
