---
id: TASK-0002
title: 'SEC-33: Scene library has no count or size quota'
status: Done
assignee:
  - TASK-0026
created_date: '2026-06-07 11:09'
updated_date: '2026-06-07 12:59'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/ext_routes.rs:593` (`create_scene`); storage in `crates/storage/src/sqlite.rs:212` / `crates/storage/src/memory.rs:70`

**What**: `create_scene` accepts `name`, `document_id`, and `key` of arbitrary length and inserts a new `scenes` row with no upper bound on row count or total bytes. Every other write path that can grow persistent state is quota-bounded — anonymous documents (`routes.rs:create_document`, `max_documents_bytes`), durable files (`firebase.rs:persist_durable_file`, `max_files_bytes`), and the in-memory emulators (`firebase.rs:BoundedMap`, `oauth::PendingFlows`). The scene library is the one unbounded persistent table.

**Why it matters**: SEC-33 (bound resource consumption on untrusted input). In open mode (`auth_enabled == false`) `create_scene` is reachable by anyone who can reach the server via the pass-through `CurrentUser` in `require_session`; with auth configured it is reachable by any signed-in user. In both cases a caller can grow the `scenes` table without limit (unbounded row count, unbounded per-field length), filling disk/process memory — the same DoS class the firebase emulators and document/file endpoints were explicitly bounded against. Threat model is weaker than the unauthenticated firebase endpoints (auth-gated when configured), hence low severity, but the inconsistency is a real gap.

<!-- scan confidence: candidates to inspect -->

**Fix options**: cap per-org/global scene count and/or reject oversized `name`/`document_id`/`key` fields at the handler; optionally add a `scenes` byte/row quota mirroring `max_documents_bytes`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 create_scene rejects unbounded growth: oversized field lengths are rejected and/or a scene count/size cap is enforced
- [x] #2 Behavior is consistent with the existing document/file quota model and documented
<!-- AC:END -->
