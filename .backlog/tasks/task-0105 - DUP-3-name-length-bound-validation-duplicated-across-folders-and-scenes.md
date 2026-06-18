---
id: TASK-0105
title: 'DUP-3: name/length-bound validation duplicated across folders and scenes'
status: Done
assignee:
  - TASK-0120
created_date: '2026-06-14 15:50'
updated_date: '2026-06-14 21:53'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 105000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/ext_routes/folders.rs:283-296` and `backend/crates/server/src/ext_routes/scenes.rs:120-132,235-236`

**What**: Folder name validation (`validated_name`) and scene name/field bounds (`create_scene`, `update_scene`) duplicate the same trim/empty/`> MAX_*_BYTES` -> `BAD_REQUEST` shape across three sites with slightly diverging rules. `update_scene` (scenes.rs:235) only length-checks and does not trim/reject empty names like folders do.

**Why it matters**: Divergent copies of input-validation logic risk one path being tightened while another is not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Factor a shared validated_name(raw, max, label) helper and reuse it for scene name validation in create_scene/update_scene
- [ ] #2 Ensure update_scene trims and rejects empty names consistently with folders
<!-- AC:END -->
