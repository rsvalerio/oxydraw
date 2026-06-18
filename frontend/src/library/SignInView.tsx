// The library panel's signed-out view: one button per configured OAuth provider plus an
// optional shared-password form. Owns its own password/busy/status state so the rest of the
// panel doesn't carry sign-in concerns.

import { useCallback, useState } from "react";

import { oauthStartUrl } from "./api";
import { PROVIDER_LABELS, type AuthState } from "./auth";

export default function SignInView({ auth }: { auth: AuthState }) {
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");

  const handleLogin = useCallback(
    async (event: React.FormEvent) => {
      event.preventDefault();
      setBusy(true);
      setStatus("");
      try {
        if (await auth.loginWithPassword(password)) {
          setPassword("");
        } else {
          setStatus("Wrong password.");
        }
      } finally {
        setBusy(false);
      }
    },
    [auth, password],
  );

  return (
    <>
      <div className="lib-signin">
        {auth.providers.providers.map((provider) => (
          <button
            key={provider}
            type="button"
            className="lib-provider"
            onClick={() => {
              window.location.href = oauthStartUrl(provider);
            }}
          >
            Sign in with {PROVIDER_LABELS[provider] ?? provider}
          </button>
        ))}
        {auth.providers.password && (
          <form className="lib-login" onSubmit={handleLogin}>
            <input
              type="password"
              placeholder="Password"
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
            <button type="submit" disabled={busy}>
              Log in
            </button>
          </form>
        )}
        {!auth.authEnabled && (
          <p className="lib-message">Sign-in is not configured on this server.</p>
        )}
      </div>
      {status && <footer className="lib-status">{status}</footer>}
    </>
  );
}
