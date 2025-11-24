export type AmountLike = string | number | bigint;

export interface IppanClientOptions {
  baseUrl?: string;
  timeoutMs?: number;
  fetchImpl?: typeof fetch;
}

export interface PaymentRequest {
  from: string;
  to: string;
  amount: AmountLike;
  fee?: AmountLike;
  nonce?: number;
  memo?: string;
  signingKey: string;
}

export interface AccountInfo {
  address: string;
  balance_atomic: string;
  nonce: number;
  recent_transactions: TransactionSummary[];
  recent_payments: PaymentSummary[];
}

export interface TransactionSummary {
  hash: string;
  from: string;
  to: string;
  amount_atomic: string;
  fee_atomic: string;
  nonce: number;
  timestamp: number;
  hash_timer: string;
  memo?: string;
  status: PaymentStatus;
}

export interface PaymentSummary {
  hash: string;
  direction: PaymentDirection;
  amount_atomic: string;
  fee_atomic: string;
  total_cost_atomic?: string;
  timestamp: number;
  memo?: string;
  status: PaymentStatus;
}

export interface BlockInfo {
  id: string;
  round: number;
  height?: number;
  creator: string;
  hash_timer: string;
  timestamp: number;
  parent_ids: string[];
  transaction_hashes: string[];
  transactions: TransactionSummary[];
  fee_summary?: FeeSummary;
}

export interface FeeSummary {
  round: number;
  total_fees_atomic: string;
  treasury_fees_atomic: string;
  applied_payments: number;
  rejected_payments: number;
}

export interface PaymentReceipt {
  tx_hash: string;
  status: PaymentStatus;
  from: string;
  to: string;
  nonce: number;
  amount_atomic: string;
  fee_atomic: string;
  timestamp: number;
  hash_timer: string;
  memo?: string;
}

export interface TimeInfo {
  timestamp: number;
  time_us: number;
}

export enum PaymentStatus {
  AcceptedToMempool = "accepted_to_mempool",
  Finalized = "finalized"
}

export enum PaymentDirection {
  Incoming = "incoming",
  Outgoing = "outgoing",
  SelfTransfer = "self_transfer"
}

export class IppanSdkError extends Error {
  public readonly status?: number;
  public readonly code?: string;

  constructor(message: string, status?: number, code?: string) {
    super(message);
    this.name = "IppanSdkError";
    this.status = status;
    this.code = code;
  }
}

export class IppanClient {
  private readonly baseUrl: string;
  private readonly timeoutMs: number;
  private readonly fetchImpl: typeof fetch;

  constructor(opts: IppanClientOptions = {}) {
    this.baseUrl = (opts.baseUrl ?? "http://127.0.0.1:8081/api/").replace(/\/?$/, "/");
    this.timeoutMs = opts.timeoutMs ?? 10000;
    this.fetchImpl = opts.fetchImpl ?? globalThis.fetch;

    if (!this.fetchImpl) {
      throw new Error("No fetch implementation available. Provide `fetchImpl` or use Node 18+.");
    }
  }

  async getAccount(addressHex: string): Promise<AccountInfo> {
    return this.get<AccountInfo>(`account/${addressHex}`);
  }

  async getBlock(id: string | number): Promise<BlockInfo> {
    return this.get<BlockInfo>(`block/${id}`);
  }

  async getTransaction(hash: string): Promise<TransactionSummary> {
    return this.get<TransactionSummary>(`tx/${hash}`);
  }

  async getTime(): Promise<TimeInfo> {
    return this.get<TimeInfo>("time");
  }

  async sendPayment(request: PaymentRequest): Promise<PaymentReceipt> {
    const body = {
      from: request.from,
      to: request.to,
      amount: formatAmount(request.amount),
      fee: request.fee ? formatAmount(request.fee) : undefined,
      nonce: request.nonce,
      memo: request.memo,
      signing_key: request.signingKey
    };

    return this.post<PaymentReceipt>("tx/payment", body);
  }

  private async get<T>(path: string): Promise<T> {
    return this.request<T>(path, { method: "GET" });
  }

  private async post<T>(path: string, body: unknown): Promise<T> {
    return this.request<T>(path, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body)
    });
  }

  private async request<T>(path: string, init: RequestInit): Promise<T> {
    const url = new URL(path, this.baseUrl).toString();
    const controller = typeof AbortController !== "undefined" ? new AbortController() : undefined;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    if (controller) {
      timeout = setTimeout(() => controller.abort(), this.timeoutMs);
      if (typeof (timeout as any).unref === "function") {
        (timeout as any).unref();
      }
    }

    const response = await this.fetchImpl(url, {
      ...init,
      signal: controller?.signal
    }).catch((err: Error) => {
      throw new IppanSdkError(err.message);
    });

    if (timeout) {
      clearTimeout(timeout);
    }

    const text = await response.text();
    let payload: any;
    try {
      payload = text ? JSON.parse(text) : {};
    } catch {
      payload = undefined;
    }

    if (!response.ok) {
      const code = payload?.code ?? "http_error";
      const message = payload?.message ?? text || `request failed with ${response.status}`;
      throw new IppanSdkError(message, response.status, code);
    }

    return payload as T;
  }
}

function formatAmount(value: AmountLike): string {
  if (typeof value === "string") {
    return value;
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error("Amount must be finite");
    }
    return Math.trunc(value).toString();
  }
  return value.toString();
}
