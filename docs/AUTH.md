# Authentication for the scene library

The scene library (`/api/ext/*`, surfaced as a panel in the app) supports three access modes:

| Mode | Configuration | Behavior |
| --- | --- | --- |
| **Open** (default) | nothing set | The library is open to anyone who can reach the server. Fine on a trusted LAN. |
| **Password** | `EXT_PASSWORD` | One shared password; everyone acts as the same built-in `local` user. |
| **OAuth sign-in** | Google and/or GitHub credentials (below) | Real accounts via "Sign in with Google / GitHub". |

Password and OAuth can be enabled together — the sign-in panel shows the provider buttons
plus the password form.

Failed password logins are throttled globally (not per-IP): after 10 failures within
60 seconds, all login attempts get a 429 until old failures age out. Global on purpose —
`EXT_PASSWORD` is one shared secret, so the total guess budget is what matters, and
client addresses are unreliable behind proxies. Worst case during an active attack,
a legitimate user waits out one 60-second window; existing sessions are unaffected.

Everything else the server does (anonymous share links, collaboration relay) is
unauthenticated by design; authentication only gates the scene library.

Signed-in users all join a single built-in **organization** (`default`) and share its
scene library. The schema already supports multiple organizations; today every login
joins the default org.

## Restricting who can sign in (OAuth allowlist)

By default OAuth sign-in admits **any** account the provider will authenticate — with
`GOOGLE_CLIENT_ID`/`GITHUB_CLIENT_ID` set on an internet-facing deployment, that is every
Google or GitHub account in the world, and they all share the single default org's scene
library. Set `EXT_ALLOWED_EMAILS` to a comma-separated allowlist to restrict access:

```bash
EXT_ALLOWED_EMAILS=me@example.com,teammate@example.com
```

Only those addresses may complete an OAuth login; any other identity is bounced to the
login error with no user or membership record created. Matching is case-insensitive and
ignores surrounding whitespace. The allowlist gates OAuth only — the `EXT_PASSWORD`
fallback is already gated by the password and is unaffected. Leaving `EXT_ALLOWED_EMAILS`
unset keeps the historical open behavior (fine on a trusted LAN) and logs a startup
warning whenever OAuth is enabled.

## How it works

- Standard OAuth 2.0 authorization-code flow with PKCE; Google additionally uses OIDC
  (the identity comes from a verified ID token).
- On first sign-in a user record is created and linked to the provider identity
  (`(provider, provider_user_id)`); later sign-ins resolve to the same user. One user
  can hold both a Google and a GitHub identity.
- A successful login sets an `ext_session` cookie (HttpOnly, SameSite=Strict, Secure on
  HTTPS, 7-day lifetime). Sessions are persisted in SQLite — restarts don't log anyone
  out — and only a hash of the token is stored.

## Redirect URI

Both providers redirect the browser back to:

```
https://<your-host>/api/ext/auth/callback/<provider>
```

i.e. `…/callback/google` and `…/callback/github`. Set `PUBLIC_URL` to your public base
URL (e.g. `https://draw.example.com`) so the server builds exactly this value. Without
it the server derives the URL from the request's `Host` and `x-forwarded-proto`
headers, **but only for loopback hosts** (`localhost`, `127.0.0.1`, `[::1]`) — local
development keeps working with zero config, while a forged `Host` header on a real
deployment cannot poison the redirect URI (the OAuth analog of password-reset
poisoning). Any non-loopback deployment must set `PUBLIC_URL` for sign-in to work.

`PUBLIC_URL` also drives the login cookies' `Secure` attribute: when it starts with
`https://`, `Secure` is always set, regardless of request headers. Without `PUBLIC_URL`
the server falls back to the `x-forwarded-proto` header — only trust that setup behind
a reverse proxy that *overwrites* the header on every request (as the nginx example in
[DEPLOYMENT.md](DEPLOYMENT.md) does); a client talking to the server directly can spoof
it. On an HTTPS deployment, set `PUBLIC_URL`.

For local development, `http://localhost:3002/api/ext/auth/callback/<provider>` is
accepted by both Google and GitHub.

## Google setup (Google Cloud Console)

1. Open <https://console.cloud.google.com/> and create (or pick) a project.
2. **APIs & Services → OAuth consent screen**: configure the consent screen
   (User type *External* is fine). The default non-sensitive scopes are all that's
   needed (`openid`, `email`, `profile`).
3. **APIs & Services → Credentials → Create credentials → OAuth client ID**:
   - Application type: **Web application**
   - Authorized redirect URI: `https://<your-host>/api/ext/auth/callback/google`
     (add the `http://localhost:3002/…` variant for local dev)
4. Copy the client ID and secret into the environment:

```bash
GOOGLE_CLIENT_ID=1234567890-abc.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=GOCSPX-…
PUBLIC_URL=https://draw.example.com
```

While the consent screen is in *Testing* status only listed test users can sign in;
publish it to lift that restriction.

## GitHub setup (OAuth App)

1. Open <https://github.com/settings/developers> → **OAuth Apps → New OAuth App**
   (or under an organization: *Org settings → Developer settings*).
2. Fill in:
   - Homepage URL: `https://<your-host>`
   - Authorization callback URL: `https://<your-host>/api/ext/auth/callback/github`
3. Register, then **Generate a new client secret** and set:

```bash
GITHUB_CLIENT_ID=Iv1.…
GITHUB_CLIENT_SECRET=…
PUBLIC_URL=https://draw.example.com
```

The app requests the `read:user` and `user:email` scopes; private email addresses are
resolved via the `/user/emails` API. For GitHub Enterprise, point `GITHUB_BASE_URL`
and `GITHUB_API_URL` at your instance.

## Environment variable reference

| Variable | Default | Meaning |
| --- | --- | --- |
| `EXT_PASSWORD` | unset | Shared-password login (fallback / alternative to OAuth). |
| `EXT_ALLOWED_EMAILS` | unset | Comma-separated allowlist of emails permitted to sign in via OAuth. Unset = open to every account the provider authenticates. |
| `PUBLIC_URL` | derived from request | Public base URL used to build OAuth redirect URIs. An `https://` value also forces the `Secure` attribute on login cookies, independent of request headers. |
| `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` | unset | Enable "Sign in with Google" (both required). |
| `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET` | unset | Enable "Sign in with GitHub" (both required). |
| `GOOGLE_ISSUER_URL` | `https://accounts.google.com` | OIDC issuer override (tests / mock providers). |
| `GITHUB_BASE_URL` / `GITHUB_API_URL` | github.com | GitHub Enterprise endpoints. |

## Adding more providers later

The provider registry (`backend/crates/server/src/oauth/`) has two buckets:

- **OIDC providers** (Microsoft Entra, GitLab, …): reuse `OidcProvider` with a
  different issuer URL and client credentials — configuration, not code.
- **Plain OAuth2 providers** (Facebook, Discord, …): a small module shaped like
  `oauth/github.rs` — authorization/token endpoints plus a userinfo-to-profile mapping.

Either way the routes (`/api/ext/auth/{provider}`, `…/callback/{provider}`), session
handling, and user/identity storage are already generic.
