---
id: TASK-0050
title: >-
  TEST-11: WAL file permission assertion silently skipped when the WAL file is
  absent
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
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs:564`

**What**: `fresh_database_is_owner_only` guards the WAL-side-file permission check behind `if let Ok(meta) = std::fs::metadata(&wal)`. If the WAL file is never created (journal mode changed, write buffered differently, or the `with_extension("db-wal")` name drifting from SQLite's `<path>-wal` convention), the security assertion silently never runs and the test still passes.

**Why it matters**: This is the only check that session-token-hash data in the WAL side file is not group/other-readable (SEC-29); a silent skip converts a security regression into a green test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After the write, the test asserts the WAL file exists (e.g. let meta = std::fs::metadata(&wal).expect("WAL file created by first write")) before checking its mode
- [ ] #2 If WAL creation is legitimately environment-dependent, the skip is made explicit and logged/documented rather than an unconditional pass
<!-- AC:END -->
