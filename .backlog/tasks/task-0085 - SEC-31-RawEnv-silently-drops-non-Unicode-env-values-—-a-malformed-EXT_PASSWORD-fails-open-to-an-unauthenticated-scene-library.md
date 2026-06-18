---
id: TASK-0085
title: >-
  SEC-31: RawEnv silently drops non-Unicode env values — a malformed
  EXT_PASSWORD fails open to an unauthenticated scene library
status: Done
assignee:
  - TASK-0093
created_date: '2026-06-12 11:32'
updated_date: '2026-06-12 20:33'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config.rs:196-198` (`RawEnv::data`)

**What**: `std::env::var(key).ok()` discards both `NotPresent` and `NotUnicode` indistinguishably. An env value containing invalid UTF-8 (a binary secret, a latin-1 paste, a corrupted unit file) is silently treated as unset. For most fields that is benign, but for `EXT_PASSWORD` and `EXT_ALLOWED_EMAILS` "unset" means *less* security: the overlay library runs open and the OAuth allowlist is disabled. The server boots normally with a silently weakened posture instead of failing closed.

**Why it matters**: SEC-31 — no security bypass on error; fail closed. The startup warning for unauthenticated non-loopback binds (commit e435852) softens the password case, but the allowlist case has no equivalent warning, and an operator who set the variable has every reason to believe it took effect. Same silent-config-drop family as TASK-0079 (`.env` parse errors) — worth fixing together: distinguish `VarError::NotUnicode` (use `std::env::var_os` or match the error) and fail or at minimum `warn!` when a configured key is dropped.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A non-Unicode value for any key in ENV_KEYS is no longer silently equivalent to unset: config loading either fails with an error naming the variable or emits a warning
- [x] #2 A test covers the non-Unicode env value path (settable via `std::os::unix::ffi::OsStrExt` bytes in a jail/test process)
<!-- AC:END -->
