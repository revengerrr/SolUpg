# Phase 3: API Gateway & Merchant SDK

> **Status**: 🔲 Not Started
> **Estimated Duration**: 2-3 weeks
> **Dependencies**: Phase 2 (Routing Engine & Directory Service)

---

## Objective

Build the public-facing API Gateway and merchant SDKs that allow external developers, merchants, and dApps to integrate with SolUPG.

---

## Component 1: API Gateway

### Purpose
Single entry point for all external requests. Handles authentication, rate limiting, request validation, and proxies to internal services.

### Endpoints

#### Payments
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/payments` | Create a new payment intent |
| GET | `/v1/payments/:id` | Get payment status |
| POST | `/v1/payments/:id/cancel` | Cancel a pending payment |
| GET | `/v1/payments` | List payments (with filters) |

#### Escrow
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/escrows` | Create escrow payment |
| POST | `/v1/escrows/:id/release` | Release escrow funds |
| POST | `/v1/escrows/:id/cancel` | Cancel escrow |
| POST | `/v1/escrows/:id/dispute` | Dispute escrow |

#### Directory
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/aliases` | Register alias |
| GET | `/v1/resolve/:alias` | Resolve alias to wallet |

#### Merchants
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/merchants/register` | Register as merchant |
| GET | `/v1/merchants/dashboard` | Merchant analytics |
| POST | `/v1/webhooks` | Configure webhook URL |

### Authentication
- **API Key**: For server-to-server calls
- **JWT**: For merchant dashboard sessions
- **Wallet Signature**: For wallet-based authentication (Sign-In with Solana)

### Rate Limiting
- Free tier: 100 requests/minute
- Standard: 1,000 requests/minute
- Enterprise: Custom limits

### Technology
- **Language**: Rust (Axum)
- **Auth**: JWT + API key middleware
- **Rate Limiter**: Redis-backed sliding window
- **Docs**: Auto-generated OpenAPI/Swagger

---

## Component 2: Merchant SDK

### TypeScript SDK

```typescript
import { SolUPG } from '@solupg/sdk';

const solupg = new SolUPG({
  apiKey: 'solupg_live_...',
  network: 'mainnet-beta',
});

// Create a payment
const payment = await solupg.payments.create({
  amount: 10_000_000, // 10 USDC (6 decimals)
  token: 'USDC',
  recipient: '@merchant_coffee',
  metadata: { orderId: 'ORD-001' },
});

// Check payment status
const status = await solupg.payments.get(payment.id);

// Listen for payment events
solupg.on('payment.completed', (event) => {
  console.log('Payment received:', event.paymentId);
});
```

### Python SDK

```python
from solupg import SolUPG

solupg = SolUPG(api_key="solupg_live_...")

payment = solupg.payments.create(
    amount=10_000_000,
    token="USDC",
    recipient="@merchant_coffee",
    metadata={"order_id": "ORD-001"},
)

status = solupg.payments.get(payment.id)
```

---

## Deliverables Checklist

- [ ] API Gateway with all endpoints
- [ ] Authentication middleware (API key, JWT, wallet signature)
- [ ] Rate limiting with Redis
- [ ] TypeScript SDK published to npm
- [ ] Python SDK published to PyPI
- [ ] OpenAPI/Swagger documentation
- [ ] Webhook delivery system
- [ ] Phase 3 completion documentation

---

## Next Phase

With the API Gateway live, **Phase 4** focuses on building the Clearing & Reconciliation engine for merchant reporting and analytics.
