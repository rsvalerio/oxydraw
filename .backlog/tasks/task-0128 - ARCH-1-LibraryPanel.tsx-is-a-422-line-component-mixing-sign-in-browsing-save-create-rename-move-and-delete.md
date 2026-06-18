---
id: TASK-0128
title: >-
  ARCH-1: LibraryPanel.tsx is a 422-line component mixing sign-in, browsing,
  save, create, rename, move, and delete
status: Done
assignee:
  - TASK-0143
created_date: '2026-06-16 18:09'
updated_date: '2026-06-16 20:37'
labels:
  - code-review-web
  - structure
dependencies: []
priority: medium
ordinal: 128000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/library/LibraryPanel.tsx` (422 lines; single default-export component)

**What**: One component owns the loading/sign-in/library view switch, the password login form, the folder browser toolbar/breadcrumb, scene-save, create-folder, inline rename, move, and delete — ~12 pieces of local state and 8 handlers, plus inline `rowActions`/`renameInput` render helpers. Exceeds the ARCH-1 component-size budget (>250 lines).

**Why it matters**: The breadth of concerns in one unit makes the file hard to scan, raises the surface for state-coupling bugs (e.g. `busy`/`status` shared across every mutation), and blocks targeted unit testing. Splitting into smaller components (e.g. `SignInView`, `LibraryItemRow`, `SaveBar`, `CreateFolderBar`) along the already-clear seams would isolate state and improve readability.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 LibraryPanel is decomposed so no single component file exceeds ~250 lines and each sub-component owns a cohesive concern
- [x] #2 Behavior is unchanged and eslint + tsc still pass
<!-- AC:END -->
