// Shared sign-in state for the scene library. Auth (configured providers + current
// principal) is surfaced in two places — the main menu's Account section and the library
// panel — so it lives in one context rather than being fetched independently by each.
// Signing out from the menu must immediately reflect in the panel, and vice versa.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import {
  fetchMe,
  fetchProviders,
  login as loginRequest,
  logout as logoutRequest,
  type Me,
  type Providers,
} from "./api";

export interface AuthState {
  /** Configured sign-in capabilities (OAuth providers + password fallback). */
  providers: Providers;
  /** Current principal, or `null` when signed out / sign-in required. */
  me: Me | null;
  /** True until the initial provider + principal fetch settles. */
  loading: boolean;
  /** Whether any login mechanism is configured at all (open mode when false). */
  authEnabled: boolean;
  /** Whether a principal is currently resolved. */
  signedIn: boolean;
  /** Re-resolve the current principal (after a login/logout elsewhere). */
  refresh: () => Promise<void>;
  /** Attempt the shared-password login; returns whether it succeeded. */
  loginWithPassword: (password: string) => Promise<boolean>;
  /** Clear the session and re-resolve. */
  signOut: () => Promise<void>;
}

/** Human-friendly names for the OAuth provider keys the backend reports. */
export const PROVIDER_LABELS: Record<string, string> = { google: "Google", github: "GitHub" };

const AuthContext = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [providers, setProviders] = useState<Providers>({ providers: [], password: false });
  const [me, setMe] = useState<Me | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setMe(await fetchMe());
  }, []);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [provs, principal] = await Promise.all([fetchProviders(), fetchMe()]);
        if (cancelled) {
          return;
        }
        setProviders(provs);
        setMe(principal);
      } catch {
        // Leave defaults (open mode / signed out); the panel surfaces reachability errors.
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const loginWithPassword = useCallback(
    async (password: string) => {
      const ok = await loginRequest(password);
      if (ok) {
        await refresh();
      }
      return ok;
    },
    [refresh],
  );

  const signOut = useCallback(async () => {
    await logoutRequest();
    await refresh();
  }, [refresh]);

  const value = useMemo<AuthState>(
    () => ({
      providers,
      me,
      loading,
      authEnabled: providers.providers.length > 0 || providers.password,
      signedIn: me !== null,
      refresh,
      loginWithPassword,
      signOut,
    }),
    [providers, me, loading, refresh, loginWithPassword, signOut],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthState {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return ctx;
}
