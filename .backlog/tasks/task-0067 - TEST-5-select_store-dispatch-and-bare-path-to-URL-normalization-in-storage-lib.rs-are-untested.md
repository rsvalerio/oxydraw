---
id: TASK-0067
title: >-
  TEST-5: select_store dispatch and bare-path-to-URL normalization in storage
  lib.rs are untested
status: Done
assignee:
  - TASK-0088
created_date: '2026-06-11 22:04'
updated_date: '2026-06-12 15:17'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/lib.rs:21-44`

**What**: `select_store` — the crate's public entry point — has no test for any of its branches: `"sqlite"`/`""` dispatch, `"memory"` dispatch, or the unknown-type error. Neither does `connect_sqlite`'s normalization logic, which accepts either a bare filesystem path or a full `sqlite:` URL and appends `?mode=rwc` only to bare paths. The store-contract suites construct `SqliteStore`/`MemoryStore` directly, bypassing this layer entirely.

**Why it matters**: This is the code every production startup actually runs (the contract tests build their own URLs). A regression in the normalization — dropping `?mode=rwc` (fresh deploys fail with "unable to open database file"), mishandling a `sqlite:`-prefixed DSN, or breaking the empty-string default — would only surface at server startup in a real deployment, not in CI.

**Fix**: add unit tests in `lib.rs` driving `select_store` through a `Config`: storage_type `"memory"` returns a working store; `""` and `"sqlite"` with a tempdir `data_source_name` create the file and return a working store; a `sqlite:`-prefixed DSN is accepted verbatim; an unknown storage type returns an error naming the type.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each select_store branch (memory, sqlite, empty-string default, unknown type) has a test
- [x] #2 Both connect_sqlite input forms — bare path and sqlite:-prefixed URL — are covered, including that a bare path creates a missing database file
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added a tests module in lib.rs driving select_store through Config: memory returns a working store; sqlite and empty-string storage types with a bare tempdir path create the missing database file and return a working store; a sqlite://...?mode=rwc DSN is accepted verbatim; an unknown type errors with the type named in the error source (Backend Display is generic, so the test asserts on Error::source). Each store is exercised with a document round-trip, not just constructed. Suite passes with default features and with --no-default-features (sqlite-gated tests compile out).
<!-- SECTION:NOTES:END -->
