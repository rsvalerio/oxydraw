---
id: TASK-0126
title: >-
  ASYNC-3: useFolderBrowser load effect has no cancellation/staleness guard —
  rapid navigation can apply a stale folder listing
status: Done
assignee:
  - TASK-0140
created_date: '2026-06-16 18:09'
updated_date: '2026-06-16 19:25'
labels:
  - code-review-web
  - async
dependencies: []
priority: medium
ordinal: 126000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/library/useFolderBrowser.ts:37-78`

**What**: `load(folderId)` awaits `Promise.all([listFolders, listScenes])` then calls `setFolders`/`setBreadcrumb`/`setScenes`. The driving `useEffect` (line 69) invokes `void load(currentFolderId)` with no cancellation token — unlike the sibling effects in `auth.tsx` (`cancelled` flag) and `MovePicker.tsx` (`cancelled` flag), which both guard correctly.

**Why it matters**: If the user navigates folders quickly (each `navigate()` changes `currentFolderId`, re-running the effect), two `load` calls are in flight. If the earlier request resolves *after* the later one, its `setFolders`/`setScenes` overwrite the correct listing with stale contents — the panel shows folder A's children while the breadcrumb says folder B. This is a classic stale-response race (ASYNC-3), distinct from the `react-hooks/set-state-in-effect` warning ESLint already reports on this file (that warning is about cascading renders, not the race). The fix is a per-run `cancelled`/sequence guard so only the latest request commits state.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The load effect ignores results from a superseded request (cancelled flag or request-sequence/AbortController guard), matching the pattern already used in auth.tsx and MovePicker.tsx
- [x] #2 Rapid successive navigate() calls always leave the panel showing the listing for the last-selected folder
<!-- AC:END -->
