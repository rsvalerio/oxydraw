---
id: TASK-0031
title: 'SEC-10: Session tokens generated with rand::rng() instead of OsRng'
status: Done
assignee:
  - TASK-0062
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 16:06'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/session.rs:32`

**What**: Sessions::mint fills the 32-byte session token with rand::rng() (ThreadRng). SEC-10 prefers OsRng for security tokens; ThreadRng is CSPRNG-backed (ChaCha, reseeded from OsRng) so this is not exploitable, but it is userspace state that outlives fork/snapshot events and is the wrong default for credential material. OWASP A02 Cryptographic Failures.

**Why it matters**: Risk is minimal in practice, but using OsRng directly removes the userspace-PRNG state-compromise/VM-clone class entirely at zero cost on this infrequent code path (one call per login).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Token bytes are generated via rand::rngs::OsRng (or getrandom) in Sessions::mint
- [ ] #2 Existing session tests still pass (64-hex-char token, uniqueness, hashed-at-rest)
<!-- AC:END -->
