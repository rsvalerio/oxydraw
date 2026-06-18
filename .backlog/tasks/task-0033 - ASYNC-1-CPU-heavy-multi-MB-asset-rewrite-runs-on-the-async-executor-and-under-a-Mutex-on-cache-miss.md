---
id: TASK-0033
title: >-
  ASYNC-1: CPU-heavy multi-MB asset rewrite runs on the async executor (and
  under a Mutex on cache miss)
status: Done
assignee:
  - TASK-0058
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 15:26'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: medium
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/frontend.rs:58`

**What**: In the async handler `serve`, `render()` does `String::from_utf8_lossy` plus a chain of 5-6 `.replace()` passes over multi-MB embedded JS/HTML bundles (frontend.rs:159-175), each pass allocating a fresh full-size String. On the configured-host path this runs inside `or_insert_with` while holding the `RewriteCache` std Mutex (frontend.rs:58-63), serializing all concurrent text-asset requests behind the render; on the no-configured-host LAN/dev path it runs on every single request (frontend.rs:65). There is no `spawn_blocking` or yield point.

**Why it matters**: Tens of milliseconds of compute per call on a tokio worker thread starves other tasks (collab relay traffic shares the runtime); the cache-miss case additionally blocks every other frontend request on the mutex. Per-request rendering on the LAN path repeats the cost on every page load.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Text-asset rendering of multi-MB bundles is moved under tokio::task::spawn_blocking (or rendered once eagerly at startup when EXCALIDRAW_BACKEND_HOST is set)
- [ ] #2 The RewriteCache mutex is never held while render() executes (e.g. render outside the lock, insert after)
- [ ] #3 The per-request LAN-host render path either caches per trusted host or is also off the executor
<!-- AC:END -->
