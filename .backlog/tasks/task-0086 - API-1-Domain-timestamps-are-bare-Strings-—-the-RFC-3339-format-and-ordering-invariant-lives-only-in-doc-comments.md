---
id: TASK-0086
title: >-
  API-1: Domain timestamps are bare Strings — the RFC 3339 format and ordering
  invariant lives only in doc comments
status: Done
assignee:
  - TASK-0095
created_date: '2026-06-12 11:33'
updated_date: '2026-06-12 22:16'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/model.rs:32-33` (`Scene.created_at`/`updated_at`), `model.rs:45` (`User.created_at`), `model.rs:77` (`Org.created_at`), `crates/core/src/store.rs:107-111` (`upsert_user_for_identity(..., now: &str)`)

**What**: Every wall-clock timestamp in the domain model is a plain `String`, with "RFC 3339, set by the caller" stated only in doc comments. Nothing rejects a caller passing `"12/06/2026"`, an empty string, or a `+02:00`-offset value. The invariant is load-bearing: `SceneStore::list_scenes` promises "newest `updated_at` first", which the SQLite backend implements as a lexicographic `ORDER BY` over the TEXT column — correct only while every writer uses one uniform UTC format. Meanwhile `Session.expires_at` is `i64` unix seconds, so two timestamp representations coexist in one small model (the i64 choice is documented, but the asymmetry doubles what a caller must remember).

**Why it matters**: API-1 / READ-5 — a `Timestamp` newtype (constructible only from the server's clock helper, or a thin RFC-3339-validated wrapper) would turn the format contract and the sort-order guarantee into a compile-time property at zero runtime cost, matching the crate's own pattern of `UserId`/`OrgId`/`TokenHash` (store.rs:38-48). Misformatted timestamps would not corrupt data, but they silently break scene ordering and produce unparseable `created_at` values that only surface in the UI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Domain timestamps use a newtype (or validated wrapper) whose construction guarantees a uniform, lexicographically sortable RFC 3339 UTC format, replacing bare `String` fields in `Scene`, `User`, `Org` and the `now: &str` parameter of `upsert_user_for_identity`
- [x] #2 Doc comments stating the format invariant are reduced to pointing at the type, and store/server callers compile against the new type
<!-- AC:END -->
