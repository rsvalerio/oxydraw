---
id: TASK-0052
title: 'DUP-1: Capture log-writer test helper copy-pasted twice within collab tests'
status: Done
assignee:
  - TASK-0061
created_date: '2026-06-10 21:10'
updated_date: '2026-06-11 16:03'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
ordinal: 52000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/src/lib.rs:340`

**What**: The Capture struct, its std::io::Write impl, its MakeWriter impl, and the tracing_subscriber::fmt().with_writer(...).with_ansi(false) setup are duplicated verbatim inside the same #[cfg(test)] module: once in failed_emit_is_logged_with_context (crates/collab/src/lib.rs:340-365) and again in room_id_with_newline_cannot_forge_a_log_line (crates/collab/src/lib.rs:396-422). ~25 identical lines per copy.

**Why it matters**: Identical 25-line infrastructure duplicated in one file; the next log-assertion test will create a third copy, and any fix to the capture mechanics (e.g. lock handling, ansi setting) must be applied in lockstep. This is helper infrastructure, not test-clarity duplication, so DUP applies despite test-code tolerance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Capture (struct + Write + MakeWriter impls) is defined once at the tests-module level, or replaced by a capture_logs(closure) -> String helper wrapping the subscriber setup
- [ ] #2 Both log-assertion tests use the shared helper and keep their existing assertions unchanged
<!-- AC:END -->
