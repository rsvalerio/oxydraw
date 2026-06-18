---
id: TASK-0038
title: 'CONC-5: Blocking std::fs calls inside async SqliteStore::connect'
status: Done
assignee:
  - TASK-0058
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 15:26'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs:58`

**What**: The async fn `SqliteStore::connect` calls blocking `db_path.is_file()` (sqlite.rs:56) and `std::fs::set_permissions` (sqlite.rs:58) directly on the runtime instead of `tokio::fs` equivalents or `spawn_blocking`.

**Why it matters**: Impact is minimal — two metadata syscalls executed once at startup before the server accepts traffic — but it is a blocking-in-async pattern that copies poorly into hot paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The permission tightening uses tokio::fs::set_permissions / tokio::fs::metadata (or is wrapped in spawn_blocking), or carries a comment justifying the startup-only blocking call
<!-- AC:END -->
