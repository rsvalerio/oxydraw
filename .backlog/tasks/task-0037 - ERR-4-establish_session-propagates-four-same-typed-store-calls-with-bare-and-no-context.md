---
id: TASK-0037
title: >-
  ERR-4: establish_session propagates four same-typed store calls with bare ?
  and no context
status: Done
assignee:
  - TASK-0056
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 15:06'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/ext_routes.rs:226`

**What**: `establish_session` (ext_routes.rs:226-235) chains `upsert_user_for_identity?`, `ensure_default_org?`, `add_member?`, and `sessions.mint?` — all returning StoreError whose Display is the generic "storage backend error" — into an anyhow::Result without any `.context()`/`.with_context()`.

**Why it matters**: When a login fails, the logged chain cannot identify which of the four steps failed, slowing diagnosis of auth incidents.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each fallible step in establish_session carries a distinguishing .context(...) (e.g. "upserting user", "enrolling in default org", "minting session")
- [ ] #2 A simulated failure in any one step produces a log line naming that step
<!-- AC:END -->
