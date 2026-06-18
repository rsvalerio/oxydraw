// `fetch` with an upper bound on how long a request may hang. Every network call in the app
// hits a same-origin backend, so a hang is low-likelihood — but without a timeout a dead or
// slow server can strand UI state indefinitely (e.g. the Share action disabled forever while
// it awaits a POST that never settles). A timed-out request rejects, so callers surface their
// normal failure path instead of waiting forever (ASYNC-5).

/** Default upper bound for a single request. */
export const DEFAULT_TIMEOUT_MS = 15000;

/**
 * Like `fetch`, but aborts after `timeoutMs`. A caller-supplied `init.signal` (e.g. a
 * component's teardown signal) is chained in, so the request aborts when *either* the timeout
 * fires or the caller's signal does. The timer is always cleared once the response settles.
 */
export async function fetchWithTimeout(
  input: RequestInfo | URL,
  init: RequestInit = {},
  timeoutMs: number = DEFAULT_TIMEOUT_MS,
): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(
    () => controller.abort(new DOMException("Request timed out", "TimeoutError")),
    timeoutMs,
  );
  const external = init.signal;
  if (external) {
    if (external.aborted) {
      controller.abort(external.reason);
    } else {
      external.addEventListener("abort", () => controller.abort(external.reason), { once: true });
    }
  }
  try {
    return await fetch(input, { ...init, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}
