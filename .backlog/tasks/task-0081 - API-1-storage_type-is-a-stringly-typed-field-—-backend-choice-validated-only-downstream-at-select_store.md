---
id: TASK-0081
title: >-
  API-1: storage_type is a stringly-typed field — backend choice validated only
  downstream at select_store
status: Done
assignee:
  - TASK-0095
created_date: '2026-06-12 10:32'
updated_date: '2026-06-12 22:11'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
ordinal: 81000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config.rs:19-21`

**What**: `storage_type: String` admits any value; the legal set (`sqlite` | `memory`) lives only in a doc comment and in `storage::select_store`'s match (`crates/storage/src/lib.rs:22-28`). A `StorageType` enum with `#[serde(rename_all = "lowercase")]` (plus a `Default` of `Sqlite` and tolerance for `""`) would reject `STORAGE_TYPE=sqllite` at config-extraction time with a serde error naming the legal variants, and `select_store`'s `other =>` arm would become unrepresentable.

**Why it matters**: Type-states over string flags (design philosophy: "use types to represent states"). Today an invalid value does fail fast at startup via select_store's error arm, so impact is limited to error quality and to every consumer needing its own validation — hence Low. Related: TASK-0067 (Triage) covers testing select_store's dispatch; an enum here would shrink what that test has to cover.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Config::storage_type is an enum (sqlite default, memory) deserialized case-insensitively or lowercase-renamed, with empty-string tolerance preserved or consciously dropped
- [x] #2 select_store matches on the enum; the unknown-string error arm is removed
- [x] #3 Existing config tests pass; an invalid STORAGE_TYPE produces a config-extraction error naming the valid options
<!-- AC:END -->
