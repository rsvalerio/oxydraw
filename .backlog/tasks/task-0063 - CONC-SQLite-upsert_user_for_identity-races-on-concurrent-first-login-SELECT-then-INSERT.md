---
id: TASK-0063
title: >-
  CONC: SQLite upsert_user_for_identity races on concurrent first login
  (SELECT-then-INSERT)
status: Done
assignee:
  - TASK-0088
created_date: '2026-06-11 21:27'
updated_date: '2026-06-12 15:12'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: low
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs:374-405` (and helper `create_user_with_identity` at 342-370)

**What**: `upsert_user_for_identity` opens a transaction with `self.pool.begin()` (SQLite `BEGIN DEFERRED`), then `SELECT user_id FROM identities WHERE provider = ? AND provider_user_id = ?`. On a miss it calls `create_user_with_identity`, which `INSERT`s into `users` and then `identities`. Because a deferred transaction takes no write lock until its first write, two concurrent first logins for the *same* identity both observe `existing == None`, both mint a fresh user row, and both attempt `INSERT INTO identities` with the same `(provider, provider_user_id)` primary key. The second commit hits a UNIQUE constraint violation, which propagates as a `StoreError` (a 500 at the HTTP layer) instead of resolving to the already-created user. It also leaves an orphan `users` row with no identity.

**Why it matters**: A double-submitted OAuth callback (user double-click, provider retry, or two tabs) turns a first login into a 500 for one of the requests, plus a dangling user row. Rare and self-healing on retry, but it is a real read-then-write race that the transaction does not actually serialize, and the orphan user row is permanent.

**Fix options**: make the identity insert conflict-tolerant and re-resolve — e.g. `INSERT INTO identities (...) VALUES (...) ON CONFLICT(provider, provider_user_id) DO NOTHING`, then re-`SELECT user_id` and, if another tx won, delete/skip the freshly minted user; or begin an IMMEDIATE transaction so the write lock is taken before the SELECT; or insert the identity first and key user creation off its success.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Concurrent first logins for the same identity resolve to a single user without surfacing a UNIQUE-constraint error to either caller
- [x] #2 No orphan users row is left behind when the identity insert loses the race
- [x] #3 A regression test exercises two overlapping upsert_user_for_identity calls for the same (provider, provider_user_id)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed by opening the upsert transaction with BEGIN IMMEDIATE (sqlx Pool::begin_with), taking SQLite's write lock before the identity SELECT so concurrent first logins serialize; the loser waits within BUSY_TIMEOUT and then sees the winner's committed identity row (refresh path). Regression test concurrent_first_logins_resolve_to_one_user (multi_thread flavor) reproduced the failure pre-fix (SQLITE_BUSY 'database is locked' + orphan risk) and now passes 10/10 runs, asserting same user id for both callers and exactly one users row.
<!-- SECTION:NOTES:END -->
