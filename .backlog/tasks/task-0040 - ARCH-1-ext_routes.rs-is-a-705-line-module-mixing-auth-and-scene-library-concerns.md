---
id: TASK-0040
title: >-
  ARCH-1: ext_routes.rs is a 705-line module mixing auth and scene-library
  concerns
status: Done
assignee:
  - TASK-0057
created_date: '2026-06-10 21:09'
updated_date: '2026-06-11 15:19'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/ext_routes.rs:1`

**What**: The module bundles four distinct concerns, marked by its own section dividers: password login + throttle (LoginThrottle, login/logout), the OAuth sign-in flow (auth_start/auth_callback/validated_callback plus cookie helpers), the session middleware/current-user resolution, and the scene-library REST resource (create_scene/list_scenes/SceneView) plus /me. At 705 lines (~670 production) it exceeds the >500-line red flag with genuinely mixed responsibilities.

**Why it matters**: Auth-flow changes and scene-library changes now churn the same file; the security-sensitive OAuth callback logic is harder to review when interleaved with CRUD handlers, and the file will keep growing as either concern evolves.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Split into focused modules, e.g. ext_routes/auth.rs (login, throttle, OAuth handlers, cookies, require_session) and ext_routes/scenes.rs (scene CRUD, /me), with the router assembly remaining thin
- [ ] #2 No public API change: the crate-internal router() and the existing route paths stay identical, and all existing tests pass unchanged
<!-- AC:END -->
