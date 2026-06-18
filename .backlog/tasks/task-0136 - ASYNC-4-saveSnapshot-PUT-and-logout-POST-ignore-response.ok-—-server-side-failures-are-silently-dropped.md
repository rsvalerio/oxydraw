---
id: TASK-0136
title: >-
  ASYNC-4: saveSnapshot PUT and logout POST ignore response.ok — server-side
  failures are silently dropped
status: Done
assignee:
  - TASK-0140
created_date: '2026-06-16 19:02'
updated_date: '2026-06-16 19:26'
labels:
  - code-review-web
  - async
dependencies: []
priority: low
ordinal: 136000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/collab/Collab.ts:330-338` (saveSnapshot), `frontend/src/library/api.ts:68-70` (logout)

**What**: `saveSnapshot` awaits the room-snapshot `PUT` inside a `try` but never checks `response.ok` — a 4xx/5xx (quota exceeded, auth lapsed, server error) resolves the promise normally and is treated as success; only a network-level rejection is caught. `logout` likewise `await fetch(...)` with no `.ok` check, so a failed logout silently leaves the session apparently cleared client-side.

**Why it matters**: Other fetches in this codebase (`uploadScene`, `importFromShareLink`, `uploadFile`, `fetchFile`, `loadSnapshot`) consistently gate on `response.ok`; these two break that convention. For `saveSnapshot` the cost is real: a rejected persist means the collab room's durable snapshot silently stops updating while the UI shows no error, so a peer that later does `loadSnapshot` (first-in-room) restores stale state and the user believes their work was saved.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 saveSnapshot checks response.ok and logs/surfaces a failed persist instead of treating any HTTP status as success
- [x] #2 logout checks response.ok (or documents why a failed logout is acceptable to ignore)
<!-- AC:END -->
