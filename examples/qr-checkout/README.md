# QR Checkout Example

A tiny SolUPG checkout page that generates Solana Pay QR codes on demand. About 250 lines of TypeScript and inline HTML, no framework, no build step.

## What it shows

1. How to call `client.solanaPay.generate()` from a Node server
2. How to embed the resulting PNG as a `data:image/png;base64,...` data URI
3. How to wire a minimal merchant checkout page using only the Node `http` module

## Prerequisites

- Node.js 20 or newer
- A running SolUPG stack with the QR endpoint enabled (already in the api gateway)
- A SolUPG API key (run the `simple-payment` example once to register a merchant and grab one)
- A Solana wallet address that should receive payments

## Run it

```bash
cd examples/qr-checkout

# Install dependencies
npm install

# Create your env file and fill in the API key plus merchant wallet
cp .env.example .env

# Start the demo
npm start

# Then open http://localhost:4000 in your browser
```

## Try it out

1. Open `http://localhost:4000`
2. Enter an amount, for example `1.50`
3. Optionally fill in a label and message
4. Click **Generate QR**
5. Scan the QR with any Solana Pay compatible wallet (Phantom, Solflare, Backpack)

The wallet will see the recipient, amount, label, and message you configured. If you set `SPL_TOKEN_MINT` in your `.env`, the wallet will request that specific token. Leave it blank for native SOL.

## Architecture

```
+-------------------+        +------------------+        +-------------------+
|   Browser         |  POST  |   server.ts      |  POST  |   SolUPG api      |
|   /api/checkout   | -----> |  (Node http)     | -----> |   gateway         |
|                   |        |                  |        |  /v1/solana-pay   |
|                   | <----- |                  | <----- |  /generate        |
|   Render QR       |  JSON  |   forward result |  JSON  |                   |
+-------------------+        +------------------+        +-------------------+
```

The browser only ever talks to the local demo server. The local demo server is the one holding the API key, which is the right boundary: API keys should never live in client side JavaScript.

## Files

| File | Purpose |
|---|---|
| `server.ts` | Node http server with two routes plus the embedded HTML |
| `package.json` | Just `@solupg/sdk`, `dotenv`, and `tsx` to run TS without a build step |
| `tsconfig.json` | Strict TS config that targets ES2022 |
| `.env.example` | All the env vars the demo needs |

## Troubleshooting

**`Missing required env var: SOLUPG_API_KEY`**
You forgot to copy `.env.example` to `.env` and fill in your real values.

**`ECONNREFUSED` from the SDK call**
The SolUPG api gateway is not reachable at `SOLUPG_BASE_URL`. Start the stack: `cd ../../services && docker compose up -d`.

**`401 unauthorized` from the QR endpoint**
Your `SOLUPG_API_KEY` is wrong, expired, or registered against a different stack. Run the `simple-payment` example to mint a fresh key.

**QR scans but the wallet shows the wrong amount**
The Solana Pay spec uses decimal UI units, not lamports. `1.50` means 1.5 of whatever token the merchant requested. If you set `SPL_TOKEN_MINT` to USDC, `1.50` is 1.5 USDC. Native SOL works the same way: `1.50` is 1.5 SOL.

## What next

Once this works locally, try:

- Change the SPL token mint in `.env` to a real devnet token and pay yourself with Phantom on devnet
- Add a `reference` field to the request so you can correlate the on chain transfer back to a specific checkout
- Add a polling step that watches for the payment to land on chain and shows a confirmation screen
- Deploy the demo to Railway, Fly.io, or any VPS so a real merchant can use it
