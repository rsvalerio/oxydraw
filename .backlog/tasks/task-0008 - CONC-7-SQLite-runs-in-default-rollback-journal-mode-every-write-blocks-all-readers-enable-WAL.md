---
id: TASK-0008
title: >-
  CONC-7: SQLite runs in default rollback-journal mode; every write blocks all
  readers (enable WAL)
status: Done
assignee:
  - TASK-0027
created_date: '2026-06-07 11:29'
updated_date: '2026-06-07 13:04'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: low
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs:35-48`

**What**: `SqliteStore::connect` configures `busy_timeout` and `acquire_timeout` deliberately (documented constants) but leaves the journal mode at SQLite's default (rollback/DELETE). With `max_connections(5)` serving concurrent async requests, every write transaction takes an exclusive file lock that blocks all readers (and vice versa) for the duration of the write+fsync; contention surfaces as 5s `busy_timeout` stalls and then errors under write load.

**Why it matters**: For a collaborative drawing server, document writes are frequent. WAL mode (`SqliteConnectOptions::journal_mode(SqliteJournalMode::Wal)`, typically paired with `synchronous(Normal)`) lets readers proceed concurrently with a writer, removing the main contention mode this code's own comments worry about. Rule mapping is approximate (no exact rule covers DB journal config); CONC-7's "shared collection behind one lock in a hot path" is the closest fit — the rollback journal is one exclusive lock around the entire database.

**Severity note**: Baseline Medium for a write-contended service; downgraded to Low because the connection/lock timeouts are deliberate and documented, and current deployment scale is embedded/single-node.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 SqliteStore::connect enables WAL journal mode (and an explicit synchronous level) via SqliteConnectOptions
- [x] #2 A doc comment explains the journal-mode choice alongside the existing BUSY_TIMEOUT/ACQUIRE_TIMEOUT rationale
- [x] #3 Store contract tests still pass against the SQLite backend
<!-- AC:END -->
