import { describe, expect, it } from "vitest";
import type {
  RecipientIdentifier,
  CreatePaymentRequest,
  CreateEscrowRequest,
  PaymentResponse,
  ListPaymentsResponse,
  RegisterMerchantResponse,
  AliasResponse,
} from "../src/types";

/**
 * Structural/contract tests for public SDK types.
 *
 * These are type-level checks; they will fail to compile if the shape
 * of an exported type drifts. Keep the asserted values aligned with the
 * API Gateway's JSON contract.
 */

describe("SDK type contracts", () => {
  it("RecipientIdentifier allows all documented variants", () => {
    const wallet: RecipientIdentifier = { type: "Wallet", value: "W" };
    const email: RecipientIdentifier = { type: "Email", value: "a@b.co" };
    const phone: RecipientIdentifier = { type: "Phone", value: "+15551234567" };
    const sol: RecipientIdentifier = { type: "SolDomain", value: "alice.sol" };
    const merchant: RecipientIdentifier = { type: "Merchant", value: "MER-1" };

    for (const r of [wallet, email, phone, sol, merchant]) {
      expect(r.type).toBeDefined();
      expect(r.value).toBeTypeOf("string");
    }
  });

  it("CreatePaymentRequest supports minimal payload", () => {
    const minimal: CreatePaymentRequest = {
      payer: "W",
      recipient: { type: "Wallet", value: "R" },
      amount: 100,
    };
    expect(minimal.amount).toBe(100);
    expect(minimal.sourceToken).toBeUndefined();
    expect(minimal.routeType).toBeUndefined();
  });

  it("CreatePaymentRequest supports all optional fields", () => {
    const full: CreatePaymentRequest = {
      payer: "W",
      recipient: { type: "Merchant", value: "MER-1" },
      amount: 1_000_000,
      sourceToken: "SOL",
      destinationToken: "USDC",
      metadata: "order-42",
      routeType: "SwapPay",
      slippageBps: 100,
    };
    expect(full.routeType).toBe("SwapPay");
    expect(full.slippageBps).toBe(100);
  });

  it("CreateEscrowRequest condition is a bounded union", () => {
    const conditions: Array<NonNullable<CreateEscrowRequest["condition"]>> = [
      "TimeBased",
      "AuthorityApproval",
      "MutualApproval",
    ];
    expect(conditions).toHaveLength(3);
  });

  it("PaymentResponse is parseable from a realistic API payload", () => {
    const payload = {
      id: "p_01HZYX...",
      status: "confirmed",
      route_type: "DirectPay",
      tx_signature: "5abc...xyz",
    } satisfies PaymentResponse;
    expect(payload.status).toBe("confirmed");
    expect(payload.route_type).toBe("DirectPay");
  });

  it("ListPaymentsResponse shape", () => {
    const resp: ListPaymentsResponse = {
      payments: [
        {
          intent_id: "p_1",
          payer: "W",
          recipient_wallet: "R",
          amount: 100,
          status: "confirmed",
          route_type: "DirectPay",
          created_at: "2026-04-01T00:00:00Z",
        },
      ],
      total: 1,
    };
    expect(resp.total).toBe(1);
    expect(resp.payments[0].intent_id).toBe("p_1");
  });

  it("AliasResponse optional preferred_token", () => {
    const without: AliasResponse = {
      id: "a_1",
      alias_type: "email",
      alias_value: "alice@example.com",
      wallet_address: "W",
      verified: true,
    };
    const withPref: AliasResponse = { ...without, preferred_token: "USDC" };
    expect(without.preferred_token).toBeUndefined();
    expect(withPref.preferred_token).toBe("USDC");
  });

  it("route types enumerate the full set supported by the gateway", () => {
    const routes: Array<NonNullable<CreatePaymentRequest["routeType"]>> = [
      "DirectPay",
      "SwapPay",
      "Escrow",
      "SplitPay",
    ];
    expect(routes).toHaveLength(4);
  });

  it("RegisterMerchantResponse surfaces api_key at top level", () => {
    const payload: RegisterMerchantResponse = {
      merchant: { merchant_id: "MER-1", name: "Acme", kyc_status: "pending" },
      api_key: "solupg_live_test",
    };
    expect(payload.api_key).toMatch(/^solupg_/);
    expect(payload.merchant).toBeTypeOf("object");
  });
});
