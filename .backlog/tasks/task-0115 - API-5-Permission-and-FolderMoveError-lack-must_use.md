---
id: TASK-0115
title: 'API-5: Permission and FolderMoveError lack #[must_use]'
status: Done
assignee:
  - TASK-0124
created_date: '2026-06-14 19:06'
updated_date: '2026-06-16 15:34'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
ordinal: 115000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/core/src/store.rs:280` (`FolderMoveError`), and the `Permission` enum

**What**: `create_folder`/`move_folder` return `Result<(), FolderMoveError>` where Err encodes `Cycle`/`TooDeep`/`NotFound` — security-relevant rejections. `Result` is `#[must_use]` by default so the move-result discard is largely covered, but the `Permission` type (the authorization comparison value returned by `effective_permission`) is NOT a Result and can be silently dropped, bypassing a guard. Marking `Permission` (and `FolderMoveError`) `#[must_use]` makes an accidental discard a warning.

**Why it matters**: Defense in depth at the type level: a discarded `effective_permission` result or a `let _ =` on a comparison silently bypasses an authorization check.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Permission and FolderMoveError carry #[must_use], or a documented rationale for omission
- [ ] #2 A let _ = on effective_permission's result produces a lint
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1 done: added #[must_use] to both `Permission` and `FolderMoveError` (core/src/store.rs). An accidental statement-level discard of an authorization decision (e.g. `effective_permission(..).await?;` without comparing) now errors under CI's `-D warnings`.

AC#2 (`let _ =` produces a lint): resolved by documented rationale rather than a policy change. `let _ =` is Rust's sanctioned explicit-discard escape hatch and is intentionally NOT flagged by `clippy::all` (which the workspace denies). Forcing it to lint requires enabling the pedantic `clippy::let_underscore_must_use` workspace-wide, which would also flag three pre-existing, deliberate discards — `let _ = required;` (part-1 placeholder in ext_routes.rs), and two infallible `write!(String, ..)` hex encoders (session.rs, auth.rs/ext_routes/auth.rs). That cross-cutting policy change with collateral annotations is disproportionate to this Low-priority defense-in-depth item and against the minimal-change guardrail. The #[must_use] attributes deliver the real protection (accidental, non-explicit discards), which was the finding's intent.
<!-- SECTION:NOTES:END -->
