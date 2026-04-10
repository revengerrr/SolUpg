// Read-heavy scenario: hold 500 VUs against GET /v1/payments/{id}.
// This is the hottest read path (status polling from SDKs), so it must
// stay fast even when the write path is also under load.
//
// Prerequisites: a pre-seeded set of payment IDs. The scenario first
// creates N payments in a short setup stage, then enters a long read
// phase pulling IDs out of the `setup()` return value.
//
// Usage:
//   SOLUPG_API_KEY=... k6 run load-tests/scenarios/get-status.js

import http from "k6/http";
import { check, sleep } from "k6";
import { Rate, Trend } from "k6/metrics";

import { authHeaders, BASE_URL } from "../lib/auth.js";
import { randomDirectPayment, pick } from "../lib/fixtures.js";

const readLatency = new Trend("solupg_read_latency_ms", true);
const readErrors = new Rate("solupg_read_errors");

export const options = {
  scenarios: {
    steady_reads: {
      executor: "constant-vus",
      vus: 500,
      duration: __ENV.K6_DURATION || "3m",
      startTime: "5s",
    },
  },
  thresholds: {
    "solupg_read_errors": ["rate<0.001"],
    "http_req_duration{endpoint:get_payment}": ["p(95)<100", "p(99)<300"],
  },
};

/**
 * Setup: create 100 payments up front so that the steady-read stage has
 * a pool of real IDs to query. Returns { ids: string[] } which k6 passes
 * to the default function.
 */
export function setup() {
  const ids = [];
  for (let i = 0; i < 100; i += 1) {
    const res = http.post(
      `${BASE_URL}/v1/payments`,
      JSON.stringify(randomDirectPayment()),
      { headers: authHeaders(), tags: { endpoint: "setup_create" } },
    );
    if (res.status >= 200 && res.status < 300) {
      try {
        const id = res.json("id");
        if (typeof id === "string") ids.push(id);
      } catch {
        // ignore parse errors during setup
      }
    }
  }
  if (ids.length === 0) {
    throw new Error(
      "setup(): could not create any payments — check SOLUPG_API_KEY / gateway",
    );
  }
  return { ids };
}

export default function (data) {
  const id = pick(data.ids);
  const res = http.get(`${BASE_URL}/v1/payments/${id}`, {
    headers: authHeaders(),
    tags: { endpoint: "get_payment" },
  });
  readLatency.add(res.timings.duration);

  const ok = check(res, {
    "status 200": (r) => r.status === 200,
    "has status field": (r) => {
      try {
        return typeof r.json("status") === "string";
      } catch {
        return false;
      }
    },
  });
  readErrors.add(!ok);

  // Small pacing jitter to avoid all VUs hitting the same tick.
  sleep(Math.random() * 0.05);
}
