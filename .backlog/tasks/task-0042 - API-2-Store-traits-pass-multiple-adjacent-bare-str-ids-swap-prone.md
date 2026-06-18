---
id: TASK-0042
title: 'API-2: Store traits pass multiple adjacent bare &str ids (swap-prone)'
status: Done
assignee:
  - TASK-0059
created_date: '2026-06-10 21:09'
updated_date: '2026-06-11 15:44'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/store.rs:113`

**What**: Domain ids are stringly typed throughout the store traits. The sharpest case is OrgStore::add_member(&self, org_id: &str, user_id: &str, role: &str) — three adjacent &str parameters; user_id/token_hash/org_id/document_id are interchangeable Strings across all traits and model structs.

**Why it matters**: Swapping org_id and user_id (or passing a raw token where a token_hash is expected) compiles silently; a newtype like UserId/OrgId would make such mistakes type errors at zero runtime cost. Low because the crate is internal, call sites are few, and contract tests cover behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce newtypes (or at minimum a typed wrapper for the security-relevant token_hash vs raw token distinction) for the ids used in multi-id signatures such as add_member
- [ ] #2 All store implementations and callers compile with the typed signatures; no positional &str id pair remains in a public trait method
<!-- AC:END -->
