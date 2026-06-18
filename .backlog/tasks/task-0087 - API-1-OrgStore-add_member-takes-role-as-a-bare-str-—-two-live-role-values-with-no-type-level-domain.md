---
id: TASK-0087
title: >-
  API-1: OrgStore::add_member takes role as a bare &str — two live role values
  with no type-level domain
status: Done
assignee:
  - TASK-0095
created_date: '2026-06-12 11:33'
updated_date: '2026-06-12 22:19'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/store.rs:132-137` (`add_member(..., role: &str)`)

**What**: `role` is a free-form string sitting next to the crate's own `OrgId`/`UserId` newtypes in the same signature. Two distinct values are already live in the codebase ("member" at `crates/server/src/ext_routes/auth.rs:225`, "admin" in the storage contract tests at `crates/storage/src/lib.rs:332`), plus a third spelling of the default in SQL (`DEFAULT 'member'`, `crates/storage/src/sqlite.rs:130`). A typo'd role ("memmber", "Member") inserts a new role value silently — no compile error, no runtime rejection.

**Why it matters**: API-1 — roles are a closed set that will eventually drive authorization decisions; today the column is write-only, so the cost of a corrupted role is deferred until the first code path reads it, at which point bad rows already exist. A `Role` enum (`Member`, `Admin`) with an `as_str()`/`FromStr` pair closes the domain at compile time, exactly the design philosophy the surrounding newtypes already follow ("every wrong attempt rejected at compile time").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A `Role` enum in oxydraw-core replaces the `role: &str` parameter of `OrgStore::add_member`; string conversion happens only at the storage serialization boundary
- [x] #2 The SQL default and all call sites use the enum-derived string, so no third hand-written spelling of a role value remains
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC2 satisfied by removing the dead `DEFAULT 'member'` clause instead of deriving it: every org_members insert goes through add_member, which always binds a Role-derived string, so the column needs no SQL default and the only spellings of role values now live on the Role enum (as_str/FromStr, round-trip tested). MemoryStore stores Role directly — no string outside the SQLite serialization boundary.
<!-- SECTION:NOTES:END -->
