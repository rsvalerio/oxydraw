// No test files exist yet. Treat an empty test run as a pass (warning) instead of a
// hard failure so `bunx vitest run` is safe to wire into CI/pre-commit before any
// tests are written.
//
// Plain object (not `defineConfig`) so the file needs no local `vitest` install to load —
// vitest is run on demand via `bunx`.
export default {
  test: {
    passWithNoTests: true,
  },
};
