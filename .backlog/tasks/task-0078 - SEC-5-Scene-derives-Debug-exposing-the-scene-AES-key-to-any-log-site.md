---
id: TASK-0078
title: 'SEC-5: Scene derives Debug, exposing the scene AES key to any {:?} log site'
status: Done
assignee:
  - TASK-0094
created_date: '2026-06-12 10:31'
updated_date: '2026-06-12 20:49'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 78000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/model.rs:21-34`

**What**: `Scene` carries the client-generated AES key (`key`, JWK `.k` — doc: "Scene AES key … Exposed only behind the overlay's auth") but derives `Debug`, so any `{:?}` of a `Scene` (tracing field, error context, panic message, debug log in server/storage code) prints the decryption key in cleartext. The codebase already treats comparable secrets this way: `Config` implements `Debug` manually precisely so `ext_password` and OAuth client secrets "cannot leak into logs through a casual `{:?}`" (`crates/core/src/config.rs:81-109`), and `Session` stores only a token *hash* for the same reason. `Scene` is the one secret-bearing type left with a derived `Debug`. `IdentityProfile` (`store.rs:89-98`) similarly derives `Debug` over PII (email, name) — lower stakes, worth fixing in the same pass.

**Why it matters**: The scene key is what makes the stored `documents` blob decryptable — together with the (server-held) blob it yields plaintext scene content. A single future `tracing::debug!(?scene, ...)` or `.with_context(|| format!("{scene:?}"))` in a handler or store impl silently defeats the encryption design. Defense-in-depth says the type, not reviewer vigilance at every log site, should enforce redaction (same rationale the project already applied to `Config`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Scene implements Debug manually (or via a redaction wrapper) so the key field renders as "***" (presence-preserving, matching Config::redact style) while other fields stay visible
- [x] #2 A test asserts format!("{scene:?}") does not contain the key value (mirroring config.rs debug_output_redacts_secrets)
- [x] #3 IdentityProfile's derived Debug is reviewed in the same pass and either redacts email/name or is documented as acceptable
<!-- AC:END -->
