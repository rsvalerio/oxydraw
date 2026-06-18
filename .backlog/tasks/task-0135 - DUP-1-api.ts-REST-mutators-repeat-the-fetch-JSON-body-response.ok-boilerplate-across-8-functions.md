---
id: TASK-0135
title: >-
  DUP-1: api.ts REST mutators repeat the fetch + JSON-body + response.ok
  boilerplate across 8 functions
status: Done
assignee:
  - TASK-0141
created_date: '2026-06-16 19:01'
updated_date: '2026-06-16 20:00'
labels:
  - code-review-web
  - duplication
dependencies: []
priority: low
ordinal: 135000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/library/api.ts:59-185`

**What**: Eight mutator functions (`login`, `createScene`, `renameScene`, `moveScene`, `deleteScene`, `createFolder`, `renameFolder`, `moveFolder`, `deleteFolder`) each hand-roll the same shape: `const response = await fetch(url, { method, headers: { "content-type": "application/json" }, body: JSON.stringify(payload) }); return response.ok`. `renameScene`/`moveScene` and `renameFolder`/`moveFolder` differ only in the JSON body key; the DELETE pairs differ only in the URL segment.

**Why it matters**: The `content-type` header, the `encodeURIComponent(id)` on the path, and the `return response.ok` convention are duplicated 8×, so a cross-cutting change (add a CSRF header, a timeout/AbortController per ASYNC-5/TASK-0131, consistent error logging) must be applied in every copy and is easy to miss in one. A single `request(method, path, body?)` helper that returns `response.ok` would collapse these to one-line wrappers and make the AbortController work land in one place.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A shared request helper encapsulates method + JSON content-type header + body serialization + response.ok return
- [x] #2 The 8 mutators are rewritten as thin wrappers over the helper with no behavior change
<!-- AC:END -->
