---
id: TASK-0034
title: >-
  ERR-4: Startup storage error flattened via anyhow!("{e}"), severing the source
  chain
status: Done
assignee:
  - TASK-0056
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 15:06'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: medium
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/lib.rs:139`

**What**: `run()` maps the `select_store` failure with `.map_err(|e| anyhow::anyhow!("selecting storage backend: {e}"))`. `StoreError::Backend`'s Display is just "storage backend error" with the real cause (e.g. the sqlx "unable to open database file" detail) in `source()`, so formatting `{e}` discards the chain that anyhow would otherwise print at exit.

**Why it matters**: The most common startup failure (bad DATA_SOURCE_NAME path/permissions) surfaces as the unactionable message "selecting storage backend: storage backend error" instead of the underlying driver error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The map_err is replaced with .context("selecting storage backend") (StoreError is Error + Send + Sync + 'static, so this compiles)
- [ ] #2 A failed sqlite open prints the underlying sqlx cause in the startup error output
<!-- AC:END -->
