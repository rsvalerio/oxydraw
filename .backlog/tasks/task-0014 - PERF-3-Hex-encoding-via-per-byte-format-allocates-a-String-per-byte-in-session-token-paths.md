---
id: TASK-0014
title: >-
  PERF-3: Hex encoding via per-byte format! allocates a String per byte in
  session token paths
status: Done
assignee:
  - TASK-0026
created_date: '2026-06-07 11:55'
updated_date: '2026-06-07 13:02'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/session.rs:33` (also `:61`)

**What**: Both `mint` and `hash_token` build hex strings with `bytes.iter().map(|b| format!("{b:02x}")).collect()`, allocating and dropping a small `String` for every one of the 32 bytes on each session mint/validate.

**Why it matters**: 32 transient allocations per call on the login/session-validate path. A single-pass `write!` into a `String::with_capacity(64)` (or a hex helper) avoids them. Not a tight inner loop, so impact is modest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Hex conversion no longer allocates one String per byte (uses fmt::Write into a pre-sized buffer or a hex crate)
- [x] #2 session.rs tests (minted_tokens_validate_until_revoked, tokens_are_stored_hashed) still pass and token length stays 64
<!-- AC:END -->
