---
id: TASK-0010
title: 'SEC-29: SQLite database file created with default (world-readable) permissions'
status: Done
assignee:
  - TASK-0027
created_date: '2026-06-07 11:30'
updated_date: '2026-06-07 13:06'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/lib.rs:31-44` (URL with `mode=rwc`), `crates/storage/src/sqlite.rs:35-48` (`SqliteStore::connect`)

**What**: The database file is created on first start (`mode=rwc`) with SQLite's default permissions (0644 under a typical umask). The database stores user emails, OAuth identity mappings, and session token hashes; on a shared host any local user can read it.

**Why it matters**: SEC-29 (secure defaults / configuration file permissions). Session token hashes plus user PII in a world-readable file weakens the local trust boundary. Fix options: set restrictive permissions after creation (0600), or document that deployments must place the DB in a restricted directory (the Docker image may already do this — verify).

**Severity note**: Low — values are hashed tokens (not raw secrets) and typical deployment is a single-user container, but the default should still be tightened or documented.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Database file (and SQLite -wal/-shm side files if WAL is enabled) is not world-readable after first creation, or deployment docs explicitly require a restricted data directory
- [x] #2 Behavior verified on a fresh database created via mode=rwc
<!-- AC:END -->
