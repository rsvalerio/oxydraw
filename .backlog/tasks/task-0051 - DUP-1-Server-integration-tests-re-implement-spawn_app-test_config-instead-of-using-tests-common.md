---
id: TASK-0051
title: >-
  DUP-1: Server integration tests re-implement spawn_app/test_config instead of
  using tests/common
status: Done
assignee:
  - TASK-0061
created_date: '2026-06-10 21:10'
updated_date: '2026-06-11 16:03'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
ordinal: 51000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/tests/share.rs:26`

**What**: The bind-ephemeral-port + axum::serve spawn block and the memory-store test Config are copy-pasted across almost every server test binary even though crates/server/tests/common/mod.rs:12-21 already provides spawn_app(config). Byte-identical copies of spawn_app's body: share.rs:26-35 (spawn_with_config), security_headers.rs:17-26 (spawn), firestore.rs:22-31 (spawn), ext_auth.rs:20-29 (spawn). Near-identical variants that only add a store/state parameter: ext.rs:22-32, storage.rs:30-39 (spawn_with), store_errors.rs:117-134 (spawn_broken), ext_auth.rs:164-171 (inline), ext_auth.rs:226-236 (spawn_sqlite closure). The 5-line listener-bind/serve tail also recurs in the fake-provider servers at ext_oauth_github.rs:52-57 and ext_oauth_google.rs:88-94. The test_config() builder is duplicated at ext.rs:11-20, ext_auth.rs:9-18, firestore.rs:11-20, share.rs:11-20, security_headers.rs:9-15, storage.rs:11-20, and inline at store_errors.rs:118-125. Only the two OAuth binaries use common::spawn_app. (Also a TEST-4 violation; note TASK-0022 fixed the same pattern for the collab crate — this is the server-crate counterpart.)

**Why it matters**: A shared helper exists and is used by exactly two of nine binaries; the other seven each carry a diverged private copy, so any change to app bootstrapping (e.g. adding the collab layer or a new AppState field) must be repeated in ~10 places and copies will silently drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 common/mod.rs gains a spawn_app_with_store(config, store) (and optionally a serve(router) -> SocketAddr helper and a test_config() builder); spawn_app delegates to it
- [ ] #2 share.rs, security_headers.rs, firestore.rs, ext_auth.rs, ext.rs, storage.rs, and store_errors.rs use the common helpers instead of private spawn/test_config copies
- [ ] #3 No test binary retains a private copy of the bind/local_addr/tokio::spawn(axum::serve) block except where a fake third-party server genuinely needs a custom Router (which should go through the shared serve(router) helper)
<!-- AC:END -->
