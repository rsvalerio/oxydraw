---
id: TASK-0131
title: >-
  ASYNC-5: network fetches have no timeout/AbortController — a hung request can
  strand UI state (e.g. Share button stuck)
status: Done
assignee:
  - TASK-0140
created_date: '2026-06-16 18:09'
updated_date: '2026-06-16 19:28'
labels:
  - code-review-web
  - async
dependencies: []
priority: low
ordinal: 131000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/share.ts:40,82`, `frontend/src/files.ts:15,29`, `frontend/src/collab/Collab.ts:331,345`, `frontend/src/library/api.ts` (all `fetch` calls)

**What**: No `fetch` in the app passes an `AbortSignal` or any timeout. Most calls hit a same-origin backend so this is low-likelihood, but there is no upper bound on how long any request may hang.

**Why it matters**: Concretely, `App.handleShare` sets `sharing=true`, awaits `exportToShareLink` (a POST), and only clears `sharing` in `finally`. If that POST never settles (dead/slow backend, captive portal), the "Share link" action is disabled indefinitely with no error toast and no recovery short of reload. The same unbounded-wait applies to share import, file transfer, and room snapshot save/load. Add an `AbortController` with a sensible timeout (and wire abort into effect/teardown where applicable) so requests fail fast and surface an error state.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 User-initiated network actions (at minimum share upload/download) use an AbortController with a timeout and surface a failure state instead of hanging
- [x] #2 Abort is propagated from effect/component teardown where the fetch is tied to a mounted component
<!-- AC:END -->
