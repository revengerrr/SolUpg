// Shared randomized fixtures for SolUPG load tests.
//
// Every scenario pulls its payloads through these helpers so the load
// profile covers the same shape of real traffic (multiple payers,
// multiple recipient types, varying amounts).

// Base58-looking placeholder pubkeys — good enough for plan-building
// on the routing engine. Real chain submission on localnet will fail,
// which is expected for throughput testing of the HTTP path.
const PAYERS = [
  "PayerLoad11111111111111111111111111111111111",
  "PayerLoad22222222222222222222222222222222222",
  "PayerLoad33333333333333333333333333333333333",
  "PayerLoad44444444444444444444444444444444444",
];

const RECIPIENTS_WALLET = [
  "RecipLoad1111111111111111111111111111111111",
  "RecipLoad2222222222222222222222222222222222",
  "RecipLoad3333333333333333333333333333333333",
];

const RECIPIENTS_ALIAS = [
  { type: "Email", value: "alice+load@example.com" },
  { type: "Phone", value: "+15551230001" },
  { type: "SolDomain", value: "bob.sol" },
];

/**
 * Uniform random integer in [min, max].
 */
export function randInt(min, max) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

/**
 * Picks a random element from a non-empty array.
 */
export function pick(arr) {
  return arr[Math.floor(Math.random() * arr.length)];
}

/**
 * A DirectPay request payload with randomized payer / recipient / amount.
 */
export function randomDirectPayment() {
  return {
    payer: pick(PAYERS),
    recipient: { type: "Wallet", value: pick(RECIPIENTS_WALLET) },
    amount: randInt(1_000, 1_000_000),
    source_token: "SOL",
    destination_token: "SOL",
    route_type: "DirectPay",
    metadata: "load-test",
  };
}

/**
 * A SwapPay payload (SOL -> USDC devnet mint) for write-mix scenarios.
 */
export function randomSwapPayment() {
  return {
    payer: pick(PAYERS),
    recipient: { type: "Wallet", value: pick(RECIPIENTS_WALLET) },
    amount: randInt(10_000, 500_000),
    source_token: "SOL",
    destination_token: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU",
    route_type: "SwapPay",
    slippage_bps: 100,
    metadata: "load-test:swap",
  };
}

/**
 * An Escrow request payload for the mixed workflow.
 */
export function randomEscrowPayment() {
  return {
    payer: pick(PAYERS),
    recipient: pick(RECIPIENTS_ALIAS),
    amount: randInt(50_000, 2_000_000),
    source_token: "SOL",
    destination_token: "SOL",
    condition: "AuthorityApproval",
    metadata: "load-test:escrow",
  };
}
