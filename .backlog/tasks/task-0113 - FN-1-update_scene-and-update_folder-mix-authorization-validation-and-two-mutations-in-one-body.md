---
id: TASK-0113
title: >-
  FN-1: update_scene and update_folder mix authorization, validation, and two
  mutations in one body
status: Done
assignee:
  - TASK-0118
created_date: '2026-06-14 19:06'
updated_date: '2026-06-14 20:23'
labels:
  - code-review-rust
  - complexity
dependencies: []
priority: low
ordinal: 113000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/ext_routes/scenes.rs:215-254` (`update_scene`), `backend/crates/server/src/ext_routes/folders.rs:201-253` (`update_folder`)

**What**: Each handler interleaves an access check, optional name validation + rename + error map, optional destination access check + move + error map, then a reload + error map — four distinct concerns in ~40 lines with multiple early-return branches, pushing cyclomatic complexity at/over the FN threshold. (Distinct from the already-filed FN-1 on `auth_callback`, task-0104.)

**Why it matters**: This is the partial-write surface from the rename-before-authz finding (TASK-0109); collapsing rename/move into named, fully-pre-authorized helpers makes the ordering invariant explicit and testable and lowers the chance of an authorization-ordering regression.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rename and move are each extracted to a named helper; the handler reads as validate -> authorize all -> mutate -> reload
- [ ] #2 Each handler body is <= ~25 lines with a single clear abstraction level
<!-- AC:END -->
