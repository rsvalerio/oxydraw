---
id: TASK-0079
title: >-
  SEC-30: Config::from_env silently swallows .env parse errors, dropping all
  configuration in the file
status: Done
assignee:
  - TASK-0093
created_date: '2026-06-12 10:31'
updated_date: '2026-06-12 20:31'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 79000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config.rs:208`

**What**: `let _ = dotenvy::dotenv();` discards *every* error, not just file-not-found. `dotenvy::dotenv()` returns `Err` both when no `.env` exists (fine to ignore — the documented "best-effort" case) and when the file exists but has a malformed line, in which case parsing stops and the variables are not loaded. The two cases are distinguishable via `dotenvy::Error::not_found()` / the `Io(NotFound)` variant, but the code treats them identically.

**Why it matters**: A stray edit to `.env` (unquoted value with a space, BOM, Windows line ending mid-file) makes the server silently start with *defaults* instead of the operator's configuration: `EXT_PASSWORD` unset (scene library open), `EXT_ALLOWED_EMAILS` unset (OAuth admits everyone), `LISTEN` back to `0.0.0.0:3002`. That's a security-relevant fail-open on a config error — SEC-30 says validate configuration at startup and fail fast. The recent non-loopback-unauthenticated warning (e435852) mitigates one symptom but not the class.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A missing .env is still ignored silently (current behavior preserved)
- [x] #2 A malformed .env causes from_env to return an error (preferred, fail-fast) — or at minimum the parse error is surfaced to the caller for logging, with the choice documented
- [x] #3 A test covers the malformed-.env path (e.g. via figment::Jail with a bad .env file)
<!-- AC:END -->
