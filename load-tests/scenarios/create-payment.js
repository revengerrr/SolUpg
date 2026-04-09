// Write-heavy scenario: ramp 0 → 1,000 VUs over 5 minutes, targeting
// POST /v1/payments. Phase 6 success criteria: 1k TPS sustained, p95 < 200ms,
// error rate < 0.1%.
//
// Usage:
//   SOLUPG_API_KEY=... k6 run load-tests/scenarios/create-payment.js

import http from "k6/http";
import { check } from "k6";
import { Counter, Rate, Trend } from "k6/metrics";

import { authHeaders, BASE_URL } from "../lib/auth.js";
import { randomDirectPayment } from "../lib/fixtures.js";

const paymentCreated = new Counter("solupg_payments_created");
const paymentFailed = new Rate("solupg_payments_failed");
const createLatency = new Trend("solupg_create_latency_ms", true);

export const options = {
  scenarios: {
    ramp_to_1k: {
      executor: "ramping-vus",
      startVUs: 0,
      stages: [
        { duration: "30s", target: 100 },
        { duration: "1m", target: 500 },
        { duration: "2m", target: 1000 },
        { duration: "1m", target: 1000 },
        { duration: "30s", target: 0 },
      ],
      gracefulRampDown: "10s",
    },
  },
  thresholds: {
    "solupg_payments_failed": ["rate<0.001"],
    "http_req_duration{expected_response:true}": ["p(95)<200", "p(99)<500"],
    "http_req_failed": ["rate<0.01"],
  },
};

export default function () {
  const body = JSON.stringify(randomDirectPayment());
  const started = Date.now();
  const res = http.post(`${BASE_URL}/v1/payments`, body, {
    headers: authHeaders(),
    tags: { endpoint: "create_payment" },
  });
  createLatency.add(Date.now() - started);

  const ok = check(res, {
    "status 2xx": (r) => r.status >= 200 && r.status < 300,
    "has id": (r) => {
      try {
        return typeof r.json("id") === "string";
      } catch {
        return false;
      }
    },
  });

  if (ok) {
    paymentCreated.add(1);
    paymentFailed.add(0);
  } else {
    paymentFailed.add(1);
  }
}
