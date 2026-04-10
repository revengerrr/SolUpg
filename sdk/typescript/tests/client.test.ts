import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SolUPG } from "../src/index";
import type {
  CreatePaymentRequest,
  PaymentResponse,
  CreateEscrowRequest,
  EscrowResponse,
} from "../src/types";

/**
 * Smoke tests for the SolUPG client.
 *
 * These tests exercise the SDK's request shape against a mocked `fetch`.
 * They do NOT require a running API Gateway.
 */

type FetchCall = {
  url: string;
  method: string;
  headers: Record<string, string>;
  body: unknown;
};

function mockFetch(
  responses: Array<{ status?: number; json: unknown }>,
): {
  calls: FetchCall[];
  restore: () => void;
} {
  const calls: FetchCall[] = [];
  const original = globalThis.fetch;
  let idx = 0;

  globalThis.fetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = typeof input === "string" ? input : input.toString();
    const method = (init?.method ?? "GET").toUpperCase();
    const headers = Object.fromEntries(
      Object.entries((init?.headers as Record<string, string>) ?? {}),
    );
    const body = init?.body ? JSON.parse(init.body as string) : undefined;
    calls.push({ url, method, headers, body });

    const resp = responses[idx] ?? responses[responses.length - 1];
    idx += 1;
    return new Response(JSON.stringify(resp.json), {
      status: resp.status ?? 200,
      headers: { "Content-Type": "application/json" },
    });
  }) as unknown as typeof fetch;

  return {
    calls,
    restore: () => {
      globalThis.fetch = original;
    },
  };
}

describe("SolUPG client", () => {
  const baseUrl = "http://localhost:3002";
  const apiKey = "solupg_live_test_123";
  let env: ReturnType<typeof mockFetch>;

  beforeEach(() => {
    env = mockFetch([{ json: {} }]);
  });

  afterEach(() => {
    env.restore();
  });

  it("constructs with required config and exposes all namespaces", () => {
    const client = new SolUPG({ apiKey, baseUrl });
    expect(client.payments).toBeDefined();
    expect(client.escrows).toBeDefined();
    expect(client.directory).toBeDefined();
    expect(client.merchants).toBeDefined();
    expect(client.webhooks).toBeDefined();
  });

  it("uses default baseUrl when not provided", async () => {
    env.restore();
    env = mockFetch([{ json: { id: "p_1", status: "pending" } }]);
    const client = new SolUPG({ apiKey });
    await client.payments.get("p_1");
    expect(env.calls[0].url).toBe("http://localhost:3002/v1/payments/p_1");
  });

  it("sends X-API-Key header on every request", async () => {
    env.restore();
    env = mockFetch([{ json: { id: "p_1", status: "pending" } }]);
    const client = new SolUPG({ apiKey, baseUrl });
    await client.payments.get("p_1");
    expect(env.calls[0].headers["X-API-Key"]).toBe(apiKey);
    expect(env.calls[0].headers["Content-Type"]).toBe("application/json");
  });

  it("creates a payment with correct body mapping (camelCase → snake_case)", async () => {
    env.restore();
    const fakeResp: PaymentResponse = {
      id: "p_abc",
      status: "pending",
      route_type: "DirectPay",
      tx_signature: "5ab..xyz",
    };
    env = mockFetch([{ json: fakeResp }]);

    const client = new SolUPG({ apiKey, baseUrl });
    const req: CreatePaymentRequest = {
      payer: "PayerWalletAddr11111111111111111111111111111",
      recipient: { type: "Wallet", value: "RecipWallet111111111111111111111111111111111" },
      amount: 1_000_000,
      sourceToken: "SOL",
      destinationToken: "USDC",
      routeType: "SwapPay",
      slippageBps: 50,
      metadata: "test-payment",
    };

    const resp = await client.payments.create(req);

    expect(resp.id).toBe("p_abc");
    expect(env.calls).toHaveLength(1);
    const call = env.calls[0];
    expect(call.method).toBe("POST");
    expect(call.url).toBe(`${baseUrl}/v1/payments`);

    const body = call.body as Record<string, unknown>;
    expect(body.payer).toBe(req.payer);
    expect(body.amount).toBe(1_000_000);
    // Crucial contract: SDK must translate camelCase → snake_case for the API.
    expect(body.source_token).toBe("SOL");
    expect(body.destination_token).toBe("USDC");
    expect(body.route_type).toBe("SwapPay");
    expect(body.slippage_bps).toBe(50);
    expect(body.metadata).toBe("test-payment");
  });

  it("lists payments and passes query params", async () => {
    env.restore();
    env = mockFetch([{ json: { payments: [], total: 0 } }]);
    const client = new SolUPG({ apiKey, baseUrl });
    await client.payments.list({ status: "pending", limit: 10, offset: 20 });

    const call = env.calls[0];
    expect(call.method).toBe("GET");
    expect(call.url).toContain("status=pending");
    expect(call.url).toContain("limit=10");
    expect(call.url).toContain("offset=20");
  });

  it("throws SolUPGError on non-2xx response", async () => {
    env.restore();
    env = mockFetch([{ status: 429, json: { error: "rate limited" } }]);
    const client = new SolUPG({ apiKey, baseUrl });

    await expect(client.payments.get("p_1")).rejects.toMatchObject({
      statusCode: 429,
      message: "rate limited",
    });
  });

  it("creates an escrow with correct body mapping", async () => {
    env.restore();
    const fakeResp: EscrowResponse = { id: "e_xyz", status: "locked" };
    env = mockFetch([{ json: fakeResp }]);

    const client = new SolUPG({ apiKey, baseUrl });
    const req: CreateEscrowRequest = {
      payer: "PayerWalletAddr11111111111111111111111111111",
      recipient: { type: "Merchant", value: "MER-abcdef12" },
      amount: 500_000,
      sourceToken: "USDC",
      condition: "TimeBased",
      expiry: 1735689600,
    };

    const resp = await client.escrows.create(req);
    expect(resp.id).toBe("e_xyz");

    const body = env.calls[0].body as Record<string, unknown>;
    expect(body.source_token).toBe("USDC");
    expect(body.condition).toBe("TimeBased");
    expect(body.expiry).toBe(1735689600);
  });

  it("escrows.release posts to the right path", async () => {
    env.restore();
    env = mockFetch([{ json: { id: "e_xyz", status: "released" } }]);
    const client = new SolUPG({ apiKey, baseUrl });
    await client.escrows.release("e_xyz");
    expect(env.calls[0].method).toBe("POST");
    expect(env.calls[0].url).toBe(`${baseUrl}/v1/escrows/e_xyz/release`);
  });

  it("directory.resolve URL-encodes the alias", async () => {
    env.restore();
    env = mockFetch([
      {
        json: {
          id: "a_1",
          alias_type: "email",
          alias_value: "alice+test@example.com",
          wallet_address: "W",
          verified: true,
        },
      },
    ]);
    const client = new SolUPG({ apiKey, baseUrl });
    await client.directory.resolve("alice+test@example.com");
    expect(env.calls[0].url).toContain(
      "/v1/resolve/alice%2Btest%40example.com",
    );
  });

  it("merchants.login hits the expected endpoint", async () => {
    env.restore();
    env = mockFetch([{ json: { token: "jwt...", expires_in: 3600 } }]);
    const client = new SolUPG({ apiKey, baseUrl });
    const resp = await client.merchants.login({
      merchant_id: "MER-1",
      wallet_address: "W",
    });
    expect(resp.token).toBe("jwt...");
    expect(env.calls[0].url).toBe(`${baseUrl}/v1/merchants/login`);
    expect(env.calls[0].method).toBe("POST");
  });

  it("strips trailing slash on custom baseUrl", async () => {
    env.restore();
    env = mockFetch([{ json: { id: "p", status: "pending" } }]);
    const client = new SolUPG({
      apiKey,
      baseUrl: "http://api.example.com/",
    });
    await client.payments.get("p");
    expect(env.calls[0].url).toBe("http://api.example.com/v1/payments/p");
  });

  it("aborts after timeout", async () => {
    env.restore();
    // Install a fetch that never resolves.
    const original = globalThis.fetch;
    globalThis.fetch = vi.fn((_input, init: RequestInit | undefined) => {
      return new Promise((_resolve, reject) => {
        init?.signal?.addEventListener("abort", () =>
          reject(new DOMException("aborted", "AbortError")),
        );
      });
    }) as unknown as typeof fetch;

    const client = new SolUPG({ apiKey, baseUrl, timeout: 50 });
    await expect(client.payments.get("p")).rejects.toBeTruthy();

    globalThis.fetch = original;
  });
});
