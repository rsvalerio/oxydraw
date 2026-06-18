---
id: TASK-0080
title: >-
  CL-3: ENV_KEYS allowlist must be hand-synced with Config fields — drift makes
  a new env var silently ignored
status: Done
assignee:
  - TASK-0093
created_date: '2026-06-12 10:31'
updated_date: '2026-06-12 20:32'
labels:
  - code-review-rust
  - cognitive-load
dependencies: []
priority: medium
ordinal: 80000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config.rs:161-179` (with `Config` at `config.rs:14-74`)

**What**: `RawEnv` only reads the variables listed in the `ENV_KEYS` const, which is a second, manually maintained copy of the `Config` field list. Nothing — compiler, lint, or test — enforces that the two stay in sync. The existing `reads_env_overrides` test exercises only 5 of the 17 keys.

**Why it matters**: An implicit cross-file invariant (CL-3) on a security-bearing surface — auth, CORS, and quota settings all flow through here. The failure mode is the quiet kind: add a field to `Config` with a serde default, forget the `ENV_KEYS` entry, and the new env var is read by nobody — the field silently keeps its default while the operator believes they configured it (same fail-open shape as the .env issue in TASK-0079). A cheap guard exists entirely in tests: set every key in `ENV_KEYS` to a distinct value under `figment::Jail`, extract, and assert each `Config` field reflects it — that fails whenever a field is added without a key or a key is added without a field.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A test (or compile-time mechanism) fails when a Config field exists without a matching ENV_KEYS entry, and vice versa
- [x] #2 Every key in ENV_KEYS is exercised end-to-end at least once (env var set -> Config field populated)
<!-- AC:END -->
