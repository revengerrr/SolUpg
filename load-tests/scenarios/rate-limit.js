// Rate-limit validation scenario.
//
// Deliberately bursts above the configured per-API-key rate limit to
// verify that:
//   1. the gateway returns HTTP 429 (not 5xx) when the limit is hit
//   2. 429 responses carry Retry-After
//   3. the gateway recovers within one window — subsequent requests 2xx
//
// This protects the reliability contract: rate limiting must fail loudly
// and recover gracefully, never by dropping connections or crashing.
//
// Usage:
//   SOLUPG_API_KEY=... k6 run load-tests/scenarios/rate-limit.js

import http from "k6/http";
import { check, sleep } from "k6";
import { Counter, Rate } from "k6/metrics";

import { authHeaders, BASE_URL } from "../lib/auth.js";
import { randomDirectPayment } from "../lib/fixtures.js";

const tooManyRequests = new Counter("solupg_429_count");
const serverErrors = new Counter("solupg_5xx_count");
const recoveryOk = new Rate("solupg_recovery_ok");

export const options = {
  scenarios: {
    burst: {
      executor: "constant-arrival-rate",
      // Well above any reasonable default per-key limit.
      rate: 200,
      timeUnit: "1s",
      duration: "30s",
      preAllocatedVUs: 100,
      maxVUs: 200,
    },
  },
  thresholds: {
    // 5xx is a bug: rate limiting should never manifest as a crash.
    "solupg_5xx_count": ["count==0"],
    // We MUST see some 429s, otherwise the rate limiter isn't engaged
    // and this scenario isn't actually testing anything.
    "solupg_429_count": ["count>0"],
    // Post-burst recovery probes must succeed.
    "solupg_recovery_ok": ["rate>0.95"],
  },
};

export default function () {
  const res = http.post(
    `${BASE_URL}/v1/payments`,
    JSON.stringify(randomDirectPayment()),
    { headers: authHeaders(), tags: { endpoint: "burst_create" } },
  );

  if (res.status === 429) {
    tooManyRequests.add(1);
    check(res, {
      "429 carries Retry-After": (r) =>
        r.headers["Retry-After"] !== undefined ||
        r.headers["retry-after"] !== undefined,
    });
  } else if (res.status >= 500) {
    serverErrors.add(1);
  }
}

/**
 * After the burst scenario ends, probe the gateway a few times to confirm
 * it's serving normal traffic again within one window.
 */
export function teardown() {
  sleep(2); // let the rate-limit window expire
  let ok = 0;
  for (let i = 0; i < 20; i += 1) {
    const res = http.post(
      `${BASE_URL}/v1/payments`,
      JSON.stringify(randomDirectPayment()),
      { headers: authHeaders(), tags: { endpoint: "recovery_probe" } },
    );
    if (res.status >= 200 && res.status < 300) ok += 1;
    sleep(0.1);
  }
  recoveryOk.add(ok / 20);
}
