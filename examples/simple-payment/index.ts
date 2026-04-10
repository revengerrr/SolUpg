/**
 * SolUPG example: simple payment flow.
 *
 * What this script does, end to end:
 *   1. Register a new merchant (skipped if SOLUPG_API_KEY is already set).
 *   2. Create a payment intent from a payer wallet to the merchant wallet.
 *   3. Poll the payment status until it reaches a terminal state.
 *
 * Run it with:
 *   cp .env.example .env
 *   npm install
 *   npm start
 */

import "dotenv/config";
import { SolUPG } from "@solupg/sdk";

const baseUrl = process.env.SOLUPG_BASE_URL ?? "http://localhost:3002";
let apiKey = process.env.SOLUPG_API_KEY ?? "";
const merchantWallet = requireEnv("MERCHANT_WALLET");
const payerWallet = requireEnv("PAYER_WALLET");

async function main(): Promise<void> {
  if (!apiKey) {
    apiKey = await registerMerchantAndGetKey();
    console.log("\nSave this key in your .env as SOLUPG_API_KEY to skip registration next time.\n");
  }

  const client = new SolUPG({ baseUrl, apiKey });

  console.log("Creating payment intent:");
  const payment = await client.payments.create({
    payer: payerWallet,
    recipient: { type: "Wallet", value: merchantWallet },
    amount: 1_000_000, // 1 USDC in smallest units (6 decimals)
    sourceToken: "USDC",
    destinationToken: "USDC",
    metadata: "solupg-simple-payment-example",
  });

  console.log("  id:         ", payment.id);
  console.log("  status:     ", payment.status);
  console.log("  route_type: ", payment.route_type ?? "(not routed yet)");

  console.log("\nPolling for terminal status:");
  const finalStatus = await pollUntilTerminal(client, payment.id);

  console.log("\nFinal payment state:");
  console.log("  id:          ", finalStatus.id);
  console.log("  status:      ", finalStatus.status);
  console.log("  tx_signature:", finalStatus.tx_signature ?? "(none)");
  if (finalStatus.error) {
    console.log("  error:       ", finalStatus.error);
  }
}

/**
 * Registers a brand new merchant on the target SolUPG instance and returns
 * the auto generated API key. This only needs to happen once per merchant.
 */
async function registerMerchantAndGetKey(): Promise<string> {
  const bootstrap = new SolUPG({ baseUrl, apiKey: "bootstrap" });

  console.log("No SOLUPG_API_KEY in env, registering a fresh merchant:");
  const resp = await bootstrap.merchants.register({
    name: "Simple Payment Example Merchant",
    wallet_address: merchantWallet,
    preferred_token: "USDC",
  });

  const merchant = resp.merchant as { id?: string };
  console.log("  merchant id:", merchant.id ?? "(unknown)");
  console.log("  api_key:    ", resp.api_key);

  return resp.api_key;
}

/**
 * Polls the payment endpoint until the payment reaches a terminal state
 * (confirmed, failed, or cancelled), or until the max attempts is reached.
 */
async function pollUntilTerminal(
  client: SolUPG,
  id: string,
  maxAttempts = 20,
  intervalMs = 1_500,
) {
  const terminal = new Set(["confirmed", "failed", "cancelled", "error"]);

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    const current = await client.payments.get(id);
    const suffix = current.route_type ? ` (${current.route_type})` : "";
    console.log(`  attempt ${attempt}: ${current.status}${suffix}`);

    if (terminal.has(current.status)) {
      return current;
    }

    await sleep(intervalMs);
  }

  throw new Error(`payment ${id} did not reach a terminal state within ${maxAttempts} attempts`);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value || value.startsWith("REPLACE_WITH_")) {
    console.error(`Missing required env var: ${name}`);
    console.error(`Copy .env.example to .env and fill it in, then run again.`);
    process.exit(1);
  }
  return value;
}

main().catch((err) => {
  console.error("\nExample failed:");
  console.error(err);
  process.exit(1);
});
