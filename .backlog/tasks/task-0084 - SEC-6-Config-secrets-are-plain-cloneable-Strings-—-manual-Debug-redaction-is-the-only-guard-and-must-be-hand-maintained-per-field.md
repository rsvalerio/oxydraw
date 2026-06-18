---
id: TASK-0084
title: >-
  SEC-6: Config secrets are plain cloneable Strings — manual Debug redaction is
  the only guard and must be hand-maintained per field
status: Done
assignee:
  - TASK-0094
created_date: '2026-06-12 11:31'
updated_date: '2026-06-12 20:54'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 84000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config.rs:14-74` (fields `ext_password:28`, `google_client_secret:43`, `github_client_secret:49`; manual Debug `:81-109`)

**What**: The three secret-bearing fields are plain `Option<String>` on a `#[derive(Clone)]` struct. The only leak protection is the hand-written `Debug` impl calling `redact()` on exactly those three fields. Nothing enforces that invariant: a future secret field (e.g. a fourth OAuth provider's client secret) added to the struct and to the `Debug` impl without `redact()` compiles and leaks silently — the same hand-sync drift class as TASK-0080's `ENV_KEYS` allowlist. The `debug_output_redacts_secrets` test pins only the three currently-known secrets. Secrets are also freely cloneable and never zeroized on drop.

**Why it matters**: SEC-5/SEC-6 — `secrecy::SecretString` moves redaction from convention to the type system: its `Debug` prints `Secret([REDACTED])` unconditionally, accidental `Display`/serialization will not compile, and the buffer is zeroized on drop. With it, the manual `Debug` impl (and its drift hazard) can be deleted and replaced by a derived one, and a new secret field is safe by construction. Callers unwrap explicitly via `.expose_secret()`, making every use site of a raw secret greppable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Secret-bearing Config fields (`ext_password`, `google_client_secret`, `github_client_secret`) use `secrecy::SecretString` (or equivalent type-enforced redaction) instead of plain `String`
- [x] #2 The hand-maintained `Debug` impl no longer carries per-field redaction responsibility — a newly added secret field cannot leak through `{:?}` without a compile error or type-level redaction
- [x] #3 Existing redaction test still passes; use sites access secrets through an explicit expose call
<!-- AC:END -->
