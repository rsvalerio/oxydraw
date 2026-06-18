---
id: TASK-0068
title: >-
  READ-6: SQLite schema declares foreign keys inconsistently — sessions and
  scenes omit REFERENCES that sibling tables have
status: Done
assignee:
  - TASK-0088
created_date: '2026-06-11 22:04'
updated_date: '2026-06-12 15:19'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/storage/src/sqlite.rs:87-140` (`SCHEMA`)

**What**: `identities.user_id` and `org_members.{org_id,user_id}` declare `REFERENCES users(id)` / `REFERENCES orgs(id)`, and sqlx's SQLite options enable the `foreign_keys` pragma by default, so those constraints are enforced. The remaining relationships are not declared: `sessions.user_id` (→ users.id), `scenes.document_id` (→ documents.id), and `scenes.owner` (→ orgs.id, though `DEFAULT 'default'` may predate any org row, so this one may be intentionally loose).

**Why it matters**: Consistency and dormant integrity. The schema currently reads as if referential integrity were a deliberate per-table choice, but nothing documents why sessions/scenes are exempt. Today nothing deletes users or documents, so the gap is dormant — but the moment a delete-user or delete-document path lands, sessions and scenes can silently orphan while identities/org_members would correctly refuse, an asymmetry nobody will think to test.

**Fix**: either add the missing `REFERENCES` clauses (sessions.user_id at minimum — note SQLite cannot ALTER ADD a foreign key, so existing deployments need a table-rebuild migration or the constraint applies to fresh databases only), or document at the `SCHEMA` const which relationships are intentionally unenforced and why.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every inter-table relationship in SCHEMA is either declared with REFERENCES or has a comment explaining why it is intentionally unenforced
- [x] #2 Store contract tests still pass (insert order already satisfies the added constraints)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
sessions.user_id now declares REFERENCES users(id), matching the sibling identities/org_members tables (every production session is minted after upsert_user_for_identity, and the contract test seeds a user first, so insert order satisfies it). scenes.document_id and scenes.owner are documented at the SCHEMA const as intentionally unenforced: document_id is an opaque key per the store contract (MemoryStore enforces nothing, so a SQLite-only FK would diverge from the contract its tests exercise with synthetic ids), and owner's DEFAULT 'default' predates any org row. Also documented that IF NOT EXISTS means added constraints apply to fresh databases only. All store contract tests pass.
<!-- SECTION:NOTES:END -->
