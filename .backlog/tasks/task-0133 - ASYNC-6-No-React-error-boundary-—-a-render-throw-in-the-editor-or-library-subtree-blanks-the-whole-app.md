---
id: TASK-0133
title: >-
  ASYNC-6: No React error boundary — a render throw in the editor or library
  subtree blanks the whole app
status: Done
assignee:
  - TASK-0140
created_date: '2026-06-16 19:01'
updated_date: '2026-06-16 19:29'
labels:
  - code-review-web
  - async
dependencies: []
priority: medium
ordinal: 133000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/main.tsx:14`, `frontend/src/App.tsx:149`

**What**: The app tree (`<StrictMode><AuthProvider><App/></AuthProvider></StrictMode>`) has no React error boundary anywhere. A render-time throw in `<Excalidraw>`, `LibraryPanel`, `MovePicker`, or any child unwinds to the root and React 19 unmounts the entire tree, leaving a blank white page with no recovery path and no user-visible message.

**Why it matters**: This is a third-party-heavy SPA (`@excalidraw/excalidraw`) that also feeds it data parsed from untrusted/remote sources (share fragments, collab snapshots, REST responses cast without validation). Any of those can produce a shape the editor rejects at render time. Without a boundary, one bad element array takes down the whole canvas with no toast, retry, or "reload" affordance — the user loses access to their in-memory scene.

<!-- scan confidence: high — grep for ErrorBoundary/componentDidCatch/getDerivedStateFromError returns nothing -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An error boundary wraps the editor/library subtree (at minimum around <App/> in main.tsx)
- [x] #2 A render throw shows a recoverable fallback (e.g. message + reload) instead of a blank page
- [x] #3 The boundary logs the captured error for diagnosis
<!-- AC:END -->
