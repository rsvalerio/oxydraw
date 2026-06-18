---
id: TASK-0137
title: >-
  SEC-11: GitHub OAuth accepts unverified profile email for the sign-in
  allowlist gate
status: Done
assignee:
  - TASK-0139
created_date: '2026-06-16 19:06'
updated_date: '2026-06-16 19:15'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 137000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/oauth/github.rs:120-123`

**What**: `GithubProvider` resolves the login email with `match user.email { Some(email) => Some(email), None => self.primary_email(...) }`. The `Some(email)` arm uses the email from `GET /user` (the account's *publicly visible* profile email) directly, with no verification check. Only the `None` fallback path, `primary_email` (lines 153-168), filters on `e.primary && e.verified`.

**Why it matters**: GitHub's `/user.email` field is the user's publicly visible address and is **not** guaranteed to be verified. That value feeds straight into the `email_allowed` allowlist gate (`ext_routes/auth.rs`). An attacker can set their public profile email to an allowlisted address they do not control and pass the SEC-18 allowlist (TASK-0029) — the same bypass that SEC-11/TASK-0096 closed for the Google/OIDC `email_verified` claim, but that finding is scoped only to the OIDC path and does not cover this plain-OAuth2 provider. The asymmetry is visible in-file: the fallback path requires `verified`, the primary path does not. `google.rs` already has an `email_requires_verified_claim`-style guard; GitHub has none.

<!-- scan confidence: verified at source line -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The email used for the allowlist gate from GitHub is only ever a verified address (e.g. always resolve through /user/emails filtering on primary && verified, or otherwise confirm verification before the /user.email value is used)
- [x] #2 A test asserts that a GitHub identity whose /user.email is set but unverified does not yield that address as the gating email (mirroring google.rs email-verified coverage)
<!-- AC:END -->
