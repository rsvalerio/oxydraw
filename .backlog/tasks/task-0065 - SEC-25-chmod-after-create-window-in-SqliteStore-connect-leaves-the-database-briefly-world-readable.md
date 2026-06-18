---
id: TASK-0065
title: >-
  SEC-25: chmod-after-create window in SqliteStore::connect leaves the database
  briefly world-readable
status: Done
assignee:
  - TASK-0088
created_date: '2026-06-11 22:04'
updated_date: '2026-06-12 15:14'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 65000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs:44-65`

**What**: `connect` lets the sqlx pool create the database file (`mode=rwc`) with default umask permissions, and only afterwards tightens it to `0o600`. Between pool creation and the `set_permissions` call there is a window in which the file is world-readable. On Unix, a local process that `open(2)`s the file (or its `-wal`/`-shm` side files) during that window keeps a readable fd that survives the later chmod, and can read everything written subsequently — emails, identity mappings, session-token hashes. The existing comment ("chmodding the database before any write covers them too") covers the on-disk modes but not a held-open fd.

**Why it matters**: The SEC-29 fix (TASK-0010) is itself a small TOCTOU race. Exploitation needs a local attacker polling the data directory at server startup, so practical risk is low — but the fix is cheap and removes the window entirely instead of shrinking it.

**Fix**: before connecting, pre-create the database file with owner-only permissions so it is never world-readable: `OpenOptions::new().write(true).create(true).mode(0o600).open(&db_path)` (via `std::os::unix::fs::OpenOptionsExt`), then connect. The side files inherit the main file's mode as today. The existing `fresh_database_is_owner_only` test keeps covering the resulting mode.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The database file is created with 0o600 from the start; there is no interval in which it is group/other-readable
- [x] #2 The -wal/-shm side files are never group/other-readable at any point after the main file exists
- [x] #3 fresh_database_is_owner_only (and the WAL-mode assertion) still pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
connect now pre-creates the database file via OpenOptions with mode(0o600) (truncate(false)) before the sqlx pool opens it, so a fresh database is owner-only from its first instant — no chmod-after-create window. Pre-existing databases (older versions, looser umask) are still tightened with set_permissions before any new data is written. Empty and :memory: filenames are skipped. -wal/-shm side files inherit the main file's mode as before; fresh_database_is_owner_only including its WAL-mode assertion passes.
<!-- SECTION:NOTES:END -->
