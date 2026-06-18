---
id: TASK-0076
title: >-
  TEST-6: store-outage-as-500 contract is untested on the auth-path handlers
  despite the BrokenStore harness existing
status: Done
assignee:
  - TASK-0091
created_date: '2026-06-12 08:58'
updated_date: '2026-06-12 19:56'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 76000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/tests/store_errors.rs:124-152` (existing coverage); uncovered branches in `crates/server/src/ext_routes/auth.rs:94-99` (`require_session`), `crates/server/src/ext_routes/auth.rs:181-190` (`login` → `establish_session`), `crates/server/src/ext_routes/scenes.rs:50-53` (`me` → `find_user`), `crates/server/src/ext_routes/scenes.rs:136-139` (`create_scene` quota count)

**What**: `store_errors.rs` builds a full `BrokenStore` (every trait method errors) but pins the outage→500 contract for only two handlers: the anonymous document read and the durable storage download. The auth-side handlers carry the same contract — most notably `require_session`, whose `Err(e) => 500` arm is the fix for TASK-0036 (a backend outage must not surface as a spurious 401) and is documented with a `see ERR-6` comment — yet none of them is exercised against `BrokenStore`. A test is cheap: spawn `BrokenStore` with `ext_password` set and request `/api/ext/scenes` with any session cookie; today nothing fails if that arm regresses to 401. The `login`-establish-session, `/me` `find_user`, and `create_scene` quota-count 500 arms are uncovered for the same reason (no password-enabled BrokenStore fixture in the file).

**Why it matters**: The 401-vs-500 distinction on session validation was a deliberate, comment-annotated fix (ERR-6 / TASK-0036); without a pinning test, a refactor that collapses the `Err` arm back into "unauthorized" passes the full suite while silently masking storage outages as mass logouts. The harness for the test already exists in the same file.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 store_errors.rs spawns BrokenStore with ext_password set and pins that a guarded route with a session cookie returns 500, not 401, during an outage
- [x] #2 The login (establish_session) outage path is pinned to 500
- [x] #3 At least the /me or create_scene outage path is pinned to 500 (or both)
<!-- AC:END -->
