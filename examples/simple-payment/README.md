# Simple Payment Example

The smallest possible SolUPG integration: register a merchant, create a payment intent, poll for confirmation. About 100 lines of commented TypeScript.

## What it shows

1. How to bootstrap a fresh merchant and grab an API key
2. How to construct a `CreatePaymentRequest` using the typed SDK
3. How to poll `GET /v1/payments/{id}` until the payment reaches a terminal state

## Prerequisites

- Node.js 20 or newer
- A running SolUPG stack, either local (`docker compose up` in `services/`) or hosted
- Two Solana wallet addresses (base58): one for the merchant, one for the payer
- On localnet or devnet both wallets can be throwaway test wallets

## Run it

```bash
cd examples/simple-payment

# Install dependencies
npm install

# Create your env file and fill in wallet addresses
cp .env.example .env

# Run the example
npm start
```

On first run, leave `SOLUPG_API_KEY` blank in your `.env`. The script will register a new merchant for you and print the generated key. Save that key into `.env` to skip registration on subsequent runs.

## Expected output

```
No SOLUPG_API_KEY in env, registering a fresh merchant:
  merchant id: 4f1c...e91
  api_key:     solupg_live_...

Save this key in your .env as SOLUPG_API_KEY to skip registration next time.

Creating payment intent:
  id:          a3b5c7d9...
  status:      pending
  route_type:  (not routed yet)

Polling for terminal status:
  attempt 1: pending
  attempt 2: routed (DirectPay)
  attempt 3: confirmed (DirectPay)

Final payment state:
  id:           a3b5c7d9...
  status:       confirmed
  tx_signature: 5xK2...9wZ
```

## Troubleshooting

**`ECONNREFUSED` on `localhost:3002`**
The API gateway is not running. From the repo root: `cd services && docker compose up -d` and verify with `curl http://localhost:3002/health`.

**`401 unauthorized`**
Your `SOLUPG_API_KEY` is wrong or the merchant was registered against a different stack. Delete the key from `.env` and rerun to register a new merchant.

**Payment never confirms**
The routing engine needs access to the Solana cluster to finalize transactions. On localnet you need `solana-test-validator` running. On devnet the default RPC endpoint usually works out of the box.

## What next

Once this works, try:

- Swap `sourceToken` to `SOL` and `destinationToken` to `USDC` to exercise the auto swap path
- Change the recipient to `{ type: "Email", value: "someone@example.com" }` after registering an alias with the directory service
- Subscribe to a webhook so you do not have to poll: see `POST /v1/webhooks` in the API reference
