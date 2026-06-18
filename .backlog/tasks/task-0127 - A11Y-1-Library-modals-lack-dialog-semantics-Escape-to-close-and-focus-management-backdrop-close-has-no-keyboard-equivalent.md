---
id: TASK-0127
title: >-
  A11Y-1: Library modals lack dialog semantics, Escape-to-close, and focus
  management; backdrop close has no keyboard equivalent
status: Done
assignee:
  - TASK-0143
created_date: '2026-06-16 18:09'
updated_date: '2026-06-16 20:37'
labels:
  - code-review-web
  - a11y
dependencies: []
priority: medium
ordinal: 127000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/library/LibraryPanel.tsx:251-252`, `frontend/src/library/MovePicker.tsx:50-57`

**What**: Both modals render `<div className="lib-backdrop" onClick={...close}>` wrapping `<div className="lib-panel" onClick={stopPropagation}>`. The backdrop is a click-to-dismiss target with no `role`/keyboard handler (A11Y-1), and neither panel declares `role="dialog"`/`aria-modal="true"`/`aria-label`, traps focus, or closes on `Escape`. (The per-row icon buttons and close button do have `aria-label` — that part is fine.)

**Why it matters**: A keyboard or screen-reader user cannot dismiss the dialog the way a mouse user can (backdrop click is mouse-only), gets no announcement that a modal opened, and can Tab out of the dialog into the obscured canvas behind it. These are core dialog accessibility requirements.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both modal panels expose role="dialog" (or a <dialog> element), aria-modal, and an accessible name
- [x] #2 Escape closes each modal and focus is moved into the dialog on open and restored on close
- [x] #3 Backdrop dismissal has a keyboard-accessible equivalent (the close button already exists; ensure it is reachable and focus is trapped within the dialog)
<!-- AC:END -->
