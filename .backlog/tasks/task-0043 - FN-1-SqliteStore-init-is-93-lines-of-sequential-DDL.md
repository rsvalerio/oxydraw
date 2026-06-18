---
id: TASK-0043
title: 'FN-1: SqliteStore::init is 93 lines of sequential DDL'
status: Done
assignee:
  - TASK-0059
created_date: '2026-06-10 21:09'
updated_date: '2026-06-11 15:44'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**File**: `crates/storage/src/sqlite.rs:68`

**What**: init() spans lines 68-160 (93 lines), well past the 50-line guideline. It is, however, a flat homogeneous sequence of CREATE TABLE / CREATE INDEX executions at a single abstraction level — the shape the rule's exception clause (state machines, exhaustive arms, DSL builders) is meant to cover.

**Why it matters**: Length alone makes the schema harder to scan and each new table adds ~10 more lines to one function; a data-driven loop over a const list of DDL statements would keep it bounded. Impact is minimal since complexity is linear and cyclomatic complexity is 1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either accept as a documented FN-1 exception, or refactor to iterate over a const &[&str] of DDL statements (one execute loop), keeping per-statement comments adjacent to their SQL
- [ ] #2 Schema produced is byte-identical; existing store contract tests and the SEC-29 permission test pass
<!-- AC:END -->
