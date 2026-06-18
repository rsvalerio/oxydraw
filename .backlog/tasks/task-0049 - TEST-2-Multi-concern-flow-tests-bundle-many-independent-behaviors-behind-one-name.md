---
id: TASK-0049
title: >-
  TEST-2: Multi-concern flow tests bundle many independent behaviors behind one
  name
status: Done
assignee:
  - TASK-0060
created_date: '2026-06-10 21:09'
updated_date: '2026-06-11 15:53'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/tests/ext_auth.rs:42`

**What**: `scene_library_requires_login_when_password_set` verifies six distinct behaviors (unauthenticated 401, wrong-password 401 without cookie, successful login cookie attributes, cookie replay grants access, forged token rejected, logout revocation); `callback_rejects_unknown_state_and_replays` (ext_oauth_github.rs:209) similarly bundles forged state, state replay, consent-denied, and unknown-provider 404.

**Why it matters**: A failure at an early step masks all later assertions (e.g. a login regression hides whether logout revocation still works), and the name only describes the first concern, so CI output misidentifies what actually broke.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Independent concerns that don't need the shared session (forged token rejection, unknown provider 404, consent-denied) are split into separately named tests
- [ ] #2 Remaining flow tests cover only genuinely sequential steps (login -> use -> logout) and are named for the flow
<!-- AC:END -->
