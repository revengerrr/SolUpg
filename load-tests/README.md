# SolUPG Load Tests (k6)

Load and stress scenarios for the SolUPG API Gateway stack, authored for
[Grafana k6](https://k6.io/).

## Prerequisites

- `k6` ≥ 0.50 installed (`choco install k6` on Windows, `brew install k6` on macOS,
  or see https://k6.io/docs/get-started/installation/)
- The full SolUPG stack running locally:
  ```bash
  cd services
  docker compose up -d
  ```
- A valid API key exported as `SOLUPG_API_KEY` (the default fixtures assume a
  test merchant has been registered via `scripts/seed-db.sh`)

## Running

All scenarios read configuration from environment variables so they can be
pointed at dev, staging, or local stacks without editing files.

```bash
# Run a single scenario
k6 run load-tests/scenarios/create-payment.js

# Override target URL and API key
SOLUPG_BASE_URL=http://api.staging.solupg.internal \
SOLUPG_API_KEY=solupg_live_xxxxxxxx \
  k6 run load-tests/scenarios/mixed-workflow.js

# Override ramp/duration profile
K6_VUS=200 K6_DURATION=2m k6 run load-tests/scenarios/get-status.js
```

## Scenarios

| Script | Purpose | Target |
|---|---|---|
| `scenarios/create-payment.js` | Write-heavy ramp (0 → 1k VUs over 5 min) on `POST /v1/payments` | Peak 1k TPS writes, p95 < 200ms |
| `scenarios/get-status.js` | Read-heavy, constant 500 VUs on `GET /v1/payments/{id}` | p95 < 100ms, error rate < 0.1% |
| `scenarios/rate-limit.js` | Burst past the configured rate limit, assert 429s | 429s must appear; no 5xx |
| `scenarios/mixed-workflow.js` | Realistic mix: 70% reads, 25% creates, 5% escrow ops | Sustained 500 TPS, p95 < 250ms |

## Thresholds (Phase 6 success criteria)

Per `docs/phase-6-testing-deployment/README.md`:

- **Throughput**: ≥ 1,000 TPS sustained on `create-payment`
- **Latency**: p95 < 200ms on the gateway (end-to-end, including routing engine)
- **Error rate**: < 0.1% across the full mix
- **Recovery**: no cascade failure when one service is killed for 30s

Each scenario file declares its own `thresholds` block; a failed threshold
exits k6 non-zero, which is what CI / pre-launch sign-off checks for.

## Shared helpers

- `lib/auth.js` — builds authenticated request headers from env.
- `lib/fixtures.js` — randomized payloads (payer/recipient/amount) drawn from
  a small static pool to avoid overwhelming any one DB row.

## CI integration

These tests are **not** part of the default CI run — they need a live stack
and a budget of minutes/infra. Two recommended workflows:

1. **Manual trigger**: add a `workflow_dispatch` job that spins up compose in
   a larger runner, then runs k6 against it.
2. **Scheduled soak**: nightly cron against the staging environment.

Both flows are out of scope for this session; see
`docs/phase-6-testing-deployment/IMPLEMENTATION.md` for the external-action
checklist.

## Caveats

- k6 results for `POST /v1/payments` reflect the gateway + routing engine path
  but **do not** exercise real on-chain settlement at load; the local validator
  is single-threaded and will be the bottleneck long before the gateway is.
  For true end-to-end TPS you need devnet/mainnet-fork infrastructure.
- The `rate-limit.js` scenario depends on a rate-limit layer being present.
  Currently implemented in `services/api-gateway/src/middleware/rate_limit.rs`;
  adjust thresholds there if the limits change.
