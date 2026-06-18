---
id: TASK-0001
title: 'SEC-7: Password comparison leaks length via short-circuiting ct_eq'
status: Done
assignee:
  - TASK-0026
created_date: '2026-06-07 11:09'
updated_date: '2026-06-07 12:56'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/ext_routes.rs:180`

**What**: `expected.as_bytes().ct_eq(req.password.as_bytes())` uses `subtle::ConstantTimeEq` for `[u8]`, which **short-circuits on a length mismatch**: it returns `Choice(0)` before comparing any bytes when the two slices differ in length. The constant-time guarantee therefore only holds for equal-length inputs — an attacker submitting passwords of varying length can observe a timing difference that reveals when their guess matches the configured `EXT_PASSWORD` length.

**Why it matters**: The intent of using `subtle` here is to deny a timing oracle on the shared password. Leaking the secret's length narrows brute-force/guessing effort. Impact is bounded by the global `LoginThrottle` (10 failures / 60s) and the fact that only length (not content) leaks, so severity is low — but the mitigation is cheap and removes the oracle entirely.

**Fix**: Compare fixed-width digests instead of raw bytes, e.g. `Sha256::digest(expected) ct_eq Sha256::digest(password)` (both 32 bytes, so length is constant and content comparison is genuinely constant-time). The `sha2` crate is already a dependency (`session::hash_token`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Password verification compares constant-width values so no timing/length oracle remains for any input length
- [x] #2 Existing login throttle and constant-time-on-content behavior are preserved
<!-- AC:END -->
