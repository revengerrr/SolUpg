// Shared auth helpers for SolUPG k6 load tests.
//
// Reads SOLUPG_API_KEY from env and exposes a helper that produces the
// standard headers every authenticated gateway request needs.

const API_KEY = __ENV.SOLUPG_API_KEY || "";

if (!API_KEY) {
  // k6 will still run, but every request will hit 401. Surface this
  // loudly so the operator notices before they waste a run.
  // eslint-disable-next-line no-console
  console.warn(
    "[solupg-load] SOLUPG_API_KEY not set — requests will be unauthenticated",
  );
}

/**
 * Standard auth headers for JSON requests to the API gateway.
 * @returns {Record<string, string>}
 */
export function authHeaders() {
  return {
    "Content-Type": "application/json",
    "X-API-Key": API_KEY,
  };
}

/**
 * Base URL for the gateway under test. Override via SOLUPG_BASE_URL.
 */
export const BASE_URL =
  (__ENV.SOLUPG_BASE_URL || "http://localhost:3002").replace(/\/+$/, "");
