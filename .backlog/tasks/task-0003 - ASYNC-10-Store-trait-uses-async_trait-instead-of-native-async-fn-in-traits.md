---
id: TASK-0003
title: 'ASYNC-10: Store trait uses async_trait instead of native async fn in traits'
status: Done
assignee:
  - TASK-0028
created_date: '2026-06-07 11:24'
updated_date: '2026-06-07 13:09'
labels:
  - code-review-rust
  - async
dependencies: []
priority: low
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/store.rs:3`

**What**: The `Store` trait uses the `#[async_trait]` macro. With workspace MSRV at 1.85, native async fn in traits (stable since 1.75) is available and avoids the `Box<dyn Future>` heap allocation that async_trait adds to every method call.

**Why it matters**: Every async Store method call (find_id, list_scenes, documents_total_bytes, …) heap-allocates its future. Native syntax (or the `trait-variant` crate where `Send` bounds on `dyn Store` are required) removes that overhead and a dependency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Store trait no longer uses the async-trait macro, or a comment documents why dyn-compatibility forces it
- [x] #2 All Store implementations and call sites compile and pass the existing contract tests
<!-- AC:END -->
