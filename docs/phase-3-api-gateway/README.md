# Phase 3: API Gateway & Merchant SDK

> **Status**: ✅ Complete
> **Dependencies**: Phase 2 (Routing Engine & Directory Service)

---

## Objective

Build the public-facing API Gateway and merchant SDKs that allow external developers, merchants, and dApps to integrate with SolUPG.

---

## Component 1: API Gateway (Rust/Axum — Port 3002)

### Architecture
The API Gateway is the single entry point for all external requests. It authenticates, rate-limits, and proxies to internal services (Routing Engine on :3000, Directory Service on :3001).

### Endpoints

#### Payments (`/v1/payments`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/payments` | Create a new payment intent |
| GET | `/v1/payments/:id` | Get payment status |
| POST | `/v1/payments/:id/cancel` | Cancel a pending payment |
| GET | `/v1/payments` | List payments (with filters) |

#### Escrow (`/v1/escrows`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/escrows` | Create escrow payment |
| GET | `/v1/escrows/:id` | Get escrow status |
| POST | `/v1/escrows/:id/release` | Release escrow funds |
| POST | `/v1/escrows/:id/cancel` | Cancel escrow |
| POST | `/v1/escrows/:id/dispute` | Dispute escrow |

#### Directory (`/v1`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/aliases` | Register alias |
| GET | `/v1/resolve/:alias` | Resolve alias to wallet |

#### Merchants (`/v1/merchants`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/merchants/register` | Register as merchant (returns API key) |
| POST | `/v1/merchants/login` | Login (returns JWT) |
| GET | `/v1/merchants/dashboard` | Merchant analytics |
| POST | `/v1/merchants/api-keys` | Create additional API key |
| GET | `/v1/merchants/api-keys` | List API keys |

#### Webhooks (`/v1/webhooks`)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/webhooks` | Configure webhook URL |
| GET | `/v1/webhooks` | List webhooks |
| GET | `/v1/webhooks/:id` | Get webhook details |
| PUT | `/v1/webhooks/:id` | Update webhook |
| DELETE | `/v1/webhooks/:id` | Deactivate webhook |

### Authentication
- **API Key**: Required for all `/v1/*` routes via `X-API-Key` header
- **JWT (HMAC-SHA256)**: For merchant dashboard sessions

### Rate Limiting
Redis-backed sliding window counter per API key:
- Free tier: 100 requests/minute
- Standard: 1,000 requests/minute
- Enterprise: 10,000 requests/minute

Rate limit headers returned: `X-RateLimit-Limit`, `X-RateLimit-Remaining`

### Webhook Delivery
- HMAC-SHA256 signed payloads (`X-SolUPG-Signature` header)
- Automatic retry (3 attempts with exponential backoff)
- Delivery tracking with status in `webhook_deliveries` table

---

## Component 2: TypeScript SDK (`@solupg/sdk`)

```typescript
import { SolUPG } from '@solupg/sdk';

const solupg = new SolUPG({
  apiKey: 'solupg_live_...',
  baseUrl: 'http://localhost:3002',
});

// Create a payment
const payment = await solupg.payments.create({
  payer: 'wallet_address',
  recipient: { type: 'Email', value: 'merchant@example.com' },
  amount: 10_000_000,
});

// Check status
const status = await solupg.payments.get(payment.id);

// Escrow
const escrow = await solupg.escrows.create({
  payer: 'wallet_address',
  recipient: { type: 'Merchant', value: 'coffee_shop' },
  amount: 5_000_000,
  condition: 'AuthorityApproval',
});

// Directory
const alias = await solupg.directory.resolve('user@example.com');

// Merchant dashboard
const dashboard = await solupg.merchants.dashboard();

// Webhooks
await solupg.webhooks.create({
  merchant_id: 'uuid',
  url: 'https://example.com/webhook',
  events: ['payment.completed', 'payment.failed'],
});
```

### SDK Modules
- `solupg.payments` — Create, get, list, cancel payments
- `solupg.escrows` — Create, get, release, cancel, dispute escrows
- `solupg.directory` — Create and resolve aliases
- `solupg.merchants` — Register, login, dashboard, API key management
- `solupg.webhooks` — CRUD for webhook endpoints

---

## Database Migrations

### `api_keys` table
Stores hashed API keys with merchant association and tier (free/standard/enterprise).

### `webhooks` table
Webhook endpoint configuration per merchant with event filtering.

### `webhook_deliveries` table
Tracks delivery attempts, status, and response codes.

---

## Deliverables Checklist

- [x] API Gateway with all endpoints
- [x] Authentication middleware (API key + HMAC-SHA256 JWT)
- [x] Rate limiting with Redis (sliding window)
- [x] TypeScript SDK with full API coverage
- [x] Webhook delivery system with retry and HMAC signing
- [x] Database migrations for api_keys, webhooks, webhook_deliveries
- [x] 8 unit tests (API key gen/hash, JWT create/verify/tamper)
- [x] Phase 3 documentation

---

## Running

```bash
# Start infrastructure
cd services && docker compose up -d

# Run API Gateway (port 3002)
cargo run -p api-gateway

# Build TypeScript SDK
cd sdk/typescript && npm install && npm run build
```

---

## Next Phase

With the API Gateway live, **Phase 4** focuses on building the Clearing & Reconciliation engine for merchant reporting and analytics.
