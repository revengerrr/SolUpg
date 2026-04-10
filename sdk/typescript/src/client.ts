import type {
  SolUPGConfig,
  CreatePaymentRequest,
  PaymentResponse,
  ListPaymentsQuery,
  ListPaymentsResponse,
  CreateEscrowRequest,
  EscrowResponse,
  CreateAliasRequest,
  AliasResponse,
  RegisterMerchantRequest,
  RegisterMerchantResponse,
  LoginRequest,
  LoginResponse,
  DashboardResponse,
  CreateWebhookRequest,
  WebhookResponse,
  UpdateWebhookRequest,
  GenerateSolanaPayRequest,
  GenerateSolanaPayResponse,
  ApiError,
} from "./types";

class SolUPGError extends Error {
  constructor(
    public statusCode: number,
    message: string,
  ) {
    super(message);
    this.name = "SolUPGError";
  }
}

class HttpClient {
  private baseUrl: string;
  private apiKey: string;
  private timeout: number;

  constructor(config: SolUPGConfig) {
    this.baseUrl = (config.baseUrl || "http://localhost:3002").replace(
      /\/$/,
      "",
    );
    this.apiKey = config.apiKey;
    this.timeout = config.timeout || 30000;
  }

  async request<T>(
    method: string,
    path: string,
    body?: unknown,
    query?: Record<string, string | number | undefined>,
  ): Promise<T> {
    const url = new URL(`${this.baseUrl}${path}`);
    if (query) {
      for (const [k, v] of Object.entries(query)) {
        if (v !== undefined) url.searchParams.set(k, String(v));
      }
    }

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeout);

    try {
      const resp = await fetch(url.toString(), {
        method,
        headers: {
          "Content-Type": "application/json",
          "X-API-Key": this.apiKey,
        },
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      if (!resp.ok) {
        const err = (await resp.json().catch(() => ({
          error: resp.statusText,
        }))) as ApiError;
        throw new SolUPGError(resp.status, err.error);
      }

      return (await resp.json()) as T;
    } finally {
      clearTimeout(timer);
    }
  }
}

/** Payments API client. */
class Payments {
  constructor(private http: HttpClient) {}

  async create(req: CreatePaymentRequest): Promise<PaymentResponse> {
    return this.http.request("POST", "/v1/payments", {
      payer: req.payer,
      recipient: req.recipient,
      amount: req.amount,
      source_token: req.sourceToken,
      destination_token: req.destinationToken,
      metadata: req.metadata,
      route_type: req.routeType,
      slippage_bps: req.slippageBps,
    });
  }

  async get(id: string): Promise<PaymentResponse> {
    return this.http.request("GET", `/v1/payments/${id}`);
  }

  async list(query?: ListPaymentsQuery): Promise<ListPaymentsResponse> {
    return this.http.request("GET", "/v1/payments", undefined, query as unknown as Record<string, string | number | undefined>);
  }

  async cancel(id: string): Promise<PaymentResponse> {
    return this.http.request("POST", `/v1/payments/${id}/cancel`);
  }
}

/** Escrows API client. */
class Escrows {
  constructor(private http: HttpClient) {}

  async create(req: CreateEscrowRequest): Promise<EscrowResponse> {
    return this.http.request("POST", "/v1/escrows", {
      payer: req.payer,
      recipient: req.recipient,
      amount: req.amount,
      source_token: req.sourceToken,
      destination_token: req.destinationToken,
      condition: req.condition,
      expiry: req.expiry,
      metadata: req.metadata,
    });
  }

  async get(id: string): Promise<EscrowResponse> {
    return this.http.request("GET", `/v1/escrows/${id}`);
  }

  async release(id: string): Promise<EscrowResponse> {
    return this.http.request("POST", `/v1/escrows/${id}/release`);
  }

  async cancel(id: string): Promise<EscrowResponse> {
    return this.http.request("POST", `/v1/escrows/${id}/cancel`);
  }

  async dispute(id: string): Promise<EscrowResponse> {
    return this.http.request("POST", `/v1/escrows/${id}/dispute`);
  }
}

/** Directory (aliases) API client. */
class Directory {
  constructor(private http: HttpClient) {}

  async createAlias(req: CreateAliasRequest): Promise<AliasResponse> {
    return this.http.request("POST", "/v1/aliases", req);
  }

  async resolve(alias: string): Promise<AliasResponse> {
    return this.http.request("GET", `/v1/resolve/${encodeURIComponent(alias)}`);
  }
}

/** Merchants API client. */
class Merchants {
  constructor(private http: HttpClient) {}

  async register(
    req: RegisterMerchantRequest,
  ): Promise<RegisterMerchantResponse> {
    return this.http.request("POST", "/v1/merchants/register", req);
  }

  async login(req: LoginRequest): Promise<LoginResponse> {
    return this.http.request("POST", "/v1/merchants/login", req);
  }

  async dashboard(): Promise<DashboardResponse> {
    return this.http.request("GET", "/v1/merchants/dashboard");
  }
}

/** Webhooks API client. */
class Webhooks {
  constructor(private http: HttpClient) {}

  async create(req: CreateWebhookRequest): Promise<WebhookResponse> {
    return this.http.request("POST", "/v1/webhooks", req);
  }

  async list(): Promise<WebhookResponse[]> {
    return this.http.request("GET", "/v1/webhooks");
  }

  async get(id: string): Promise<WebhookResponse> {
    return this.http.request("GET", `/v1/webhooks/${id}`);
  }

  async update(id: string, req: UpdateWebhookRequest): Promise<WebhookResponse> {
    return this.http.request("PUT", `/v1/webhooks/${id}`, req);
  }

  async delete(id: string): Promise<{ deleted: boolean }> {
    return this.http.request("DELETE", `/v1/webhooks/${id}`);
  }
}

/** Solana Pay URL and QR generation client. */
class SolanaPay {
  constructor(private http: HttpClient) {}

  async generate(req: GenerateSolanaPayRequest): Promise<GenerateSolanaPayResponse> {
    return this.http.request("POST", "/v1/solana-pay/generate", {
      recipient: req.recipient,
      amount: req.amount,
      spl_token: req.splToken,
      reference: req.reference,
      label: req.label,
      message: req.message,
      memo: req.memo,
      qr_size: req.qrSize,
    });
  }
}

/** Main SolUPG SDK client. */
export class SolUPG {
  public readonly payments: Payments;
  public readonly escrows: Escrows;
  public readonly directory: Directory;
  public readonly merchants: Merchants;
  public readonly webhooks: Webhooks;
  public readonly solanaPay: SolanaPay;

  constructor(config: SolUPGConfig) {
    const http = new HttpClient(config);
    this.payments = new Payments(http);
    this.escrows = new Escrows(http);
    this.directory = new Directory(http);
    this.merchants = new Merchants(http);
    this.webhooks = new Webhooks(http);
    this.solanaPay = new SolanaPay(http);
  }
}
