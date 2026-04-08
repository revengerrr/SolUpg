/** SDK configuration options. */
export interface SolUPGConfig {
  /** API key for authentication (solupg_live_...) */
  apiKey: string;
  /** Base URL of the API Gateway (default: http://localhost:3002) */
  baseUrl?: string;
  /** Request timeout in ms (default: 30000) */
  timeout?: number;
}

/** Recipient identifier for payments. */
export type RecipientIdentifier =
  | { type: "Wallet"; value: string }
  | { type: "Email"; value: string }
  | { type: "Phone"; value: string }
  | { type: "SolDomain"; value: string }
  | { type: "Merchant"; value: string };

/** Payment creation request. */
export interface CreatePaymentRequest {
  payer: string;
  recipient: RecipientIdentifier;
  amount: number;
  sourceToken?: string;
  destinationToken?: string;
  metadata?: string;
  routeType?: "DirectPay" | "SwapPay" | "Escrow" | "SplitPay";
  slippageBps?: number;
}

/** Payment response from the API. */
export interface PaymentResponse {
  id: string;
  status: string;
  route_type?: string;
  tx_signature?: string;
  error?: string;
}

/** Payment list query parameters. */
export interface ListPaymentsQuery {
  status?: string;
  limit?: number;
  offset?: number;
}

/** Payment list response. */
export interface ListPaymentsResponse {
  payments: PaymentSummary[];
  total: number;
}

/** Payment summary in list results. */
export interface PaymentSummary {
  intent_id: string;
  payer: string;
  recipient_wallet: string;
  amount: number;
  status: string;
  route_type: string;
  created_at: string;
}

/** Escrow creation request. */
export interface CreateEscrowRequest {
  payer: string;
  recipient: RecipientIdentifier;
  amount: number;
  sourceToken?: string;
  destinationToken?: string;
  condition?: "TimeBased" | "AuthorityApproval" | "MutualApproval";
  expiry?: number;
  metadata?: string;
}

/** Escrow response. */
export interface EscrowResponse {
  id: string;
  status: string;
  tx_signature?: string;
  error?: string;
}

/** Alias creation request. */
export interface CreateAliasRequest {
  alias_type: string;
  alias_value: string;
  wallet_address: string;
  preferred_token?: string;
}

/** Alias response. */
export interface AliasResponse {
  id: string;
  alias_type: string;
  alias_value: string;
  wallet_address: string;
  preferred_token?: string;
  verified: boolean;
}

/** Merchant registration request. */
export interface RegisterMerchantRequest {
  name: string;
  wallet_address: string;
  merchant_id?: string;
  preferred_token?: string;
  webhook_url?: string;
}

/** Merchant registration response. */
export interface RegisterMerchantResponse {
  merchant: Record<string, unknown>;
  api_key: string;
}

/** Merchant login request. */
export interface LoginRequest {
  merchant_id: string;
  wallet_address: string;
}

/** Merchant login response. */
export interface LoginResponse {
  token: string;
  expires_in: number;
}

/** Dashboard analytics. */
export interface DashboardResponse {
  total_payments: number;
  total_volume: number;
  pending_payments: number;
  recent_payments: DashboardPayment[];
}

/** Dashboard payment summary. */
export interface DashboardPayment {
  intent_id: string;
  amount: number;
  status: string;
  created_at: string;
}

/** Webhook creation request. */
export interface CreateWebhookRequest {
  merchant_id: string;
  url: string;
  events: string[];
}

/** Webhook response. */
export interface WebhookResponse {
  id: string;
  merchant_id: string;
  url: string;
  events: string[];
  secret: string;
  is_active: boolean;
  created_at: string;
}

/** Webhook update request. */
export interface UpdateWebhookRequest {
  url?: string;
  events?: string[];
  is_active?: boolean;
}

/** API error response. */
export interface ApiError {
  error: string;
}
