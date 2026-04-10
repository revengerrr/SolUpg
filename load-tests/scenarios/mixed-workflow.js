// Mixed-workload scenario modelling realistic SDK traffic.
//
// Distribution (per iteration):
//   70% — GET /v1/payments/{id}   (status polling)
//   25% — POST /v1/payments        (new payment creation)
//    5% — POST /v1/escrows         (escrow create, lower-frequency ops)
//
// Holds 500 VUs for 5 minutes. Target: sustained ≥500 TPS, p95 < 250ms,
// overall error rate < 0.5%. This is the scenario used for pre-launch
// sign-off per docs/phase-6-testing-deployment/README.md.
//
// Usage:
//   SOLUPG_API_KEY=... k6 run load-tests/scenarios/mixed-workflow.js

import http from "k6/http";
import { check, sleep } from "k6";
import { Counter, Rate } from "k6/metrics";

import { authHeaders, BASE_URL } from "../lib/auth.js";
import {
  randomDirectPayment,
  randomEscrowPayment,
  pick,
} from "../lib/fixtures.js";

const opCounter = new Counter("solupg_mixed_ops");
const opErrors = new Rate("solupg_mixed_errors");

export const options = {
  scenarios: {
    mixed: {
      executor: "constant-vus",
      vus: 500,
      duration: __ENV.K6_DURATION || "5m",
    },
  },
  thresholds: {
    "solupg_mixed_errors": ["rate<0.005"],
    "http_req_duration": ["p(95)<250", "p(99)<600"],
    "http_req_failed": ["rate<0.01"],
  },
};

export function setup() {
  // Warm up an ID pool for the read path.
  const ids = [];
  for (let i = 0; i < 50; i += 1) {
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
        // ignore
      }
    }
  }
  if (ids.length === 0) {
    throw new Error("setup(): could not create seed payments");
  }
  return { ids };
}

function doRead(data) {
  const id = pick(data.ids);
  return http.get(`${BASE_URL}/v1/payments/${id}`, {
    headers: authHeaders(),
    tags: { endpoint: "mixed_read" },
  });
}

function doCreate() {
  return http.post(
    `${BASE_URL}/v1/payments`,
    JSON.stringify(randomDirectPayment()),
    { headers: authHeaders(), tags: { endpoint: "mixed_create" } },
  );
}

function doEscrow() {
  return http.post(
    `${BASE_URL}/v1/escrows`,
    JSON.stringify(randomEscrowPayment()),
    { headers: authHeaders(), tags: { endpoint: "mixed_escrow" } },
  );
}

export default function (data) {
  const r = Math.random();
  let res;
  if (r < 0.7) {
    res = doRead(data);
  } else if (r < 0.95) {
    res = doCreate();
  } else {
    res = doEscrow();
  }
  opCounter.add(1);

  const ok = check(res, {
    "status 2xx": (rr) => rr.status >= 200 && rr.status < 300,
  });
  opErrors.add(!ok);

  sleep(Math.random() * 0.1);
}
