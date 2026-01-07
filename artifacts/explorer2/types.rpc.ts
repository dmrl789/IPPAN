/**
 * Data origin for the devnet-only explorer.
 * - "live": fetched from the configured IPPAN devnet RPC
 * - "error": RPC missing/unreachable (no mock/demo fallback)
 */
export type RpcSource = "live" | "error";

export interface AiStatus {
  modelHash: string;
  usingStub: boolean;
  mode: "PoA" | "DLC";
}

export interface HealthStatus {
  consensus: boolean;
  dhtFile: {
    mode: "stub" | "libp2p";
    healthy: boolean;
  };
  dhtHandle: {
    mode: "stub" | "libp2p";
    healthy: boolean;
  };
  storage: boolean;
  rpc: boolean;
}

export interface BlockSummary {
  id: string;
  hash: string;
  timestamp: string;
  hashTimer: string;
  txCount: number;
  ippan_time_us?: string;
  ippan_time_ms?: number;
}

export interface Transaction {
  hash: string;
  from: string;
  to: string;
  amount: number;
  amountAtomic: string;
  fee: number;
  timestamp: string;
  hashTimer: string;
  type: string;
  status: string;
  blockId?: string;
  ippan_time_us?: string;
  ippan_time_ms?: number;
}

export type HashTimerDetail = {
  hash_timer_id: string;
  ippan_time?: string;
  ippan_time_us?: string;
  ippan_time_ms?: number;
  round_height?: number;
  block_height?: number;
  tx_ids?: string[];
  tx_count?: number;
  parents?: string[]; // optional DAG refs
  canonical_digest?: string;
};

export interface BlockDetail extends BlockSummary {
  parents: string[];
  transactions: Transaction[];
}

export interface AccountSummary {
  address: string;
  balance: number;
  balanceAtomic: string;
  nonce: number;
  handles?: string[];
}

export interface PaymentEntry {
  direction: "incoming" | "outgoing" | "self";
  amount: number;
  fee: number;
  counterparty: string;
  timestamp: string;
  hash: string;
}

export interface HandleRecord {
  /**
   * NOTE: This is a legacy shape used by early mock pages.
   * Prefer `IpndhtHandleRecord` for IPNDHT handle resolution.
   */
  handle: string;
  owner: string;
  expiresAt: string;
  dhtStatus: string;
}

export interface FileRecord {
  /**
   * NOTE: This is a legacy shape used by early mock pages.
   * Prefer `IpndhtFileDescriptor` for IPNDHT file descriptors.
   */
  id: string;
  owner: string;
  size: number;
  mimeType?: string;
  mode: "stub" | "libp2p";
  description?: string;
}

export type StatusResponseV1 = {
  head: {
    hash_timer_id: string;
    ippan_time?: string;
    ippan_time_us?: string;
    ippan_time_ms: number;
    /**
     * Preferred field name from current L1 RPC (if available).
     * Some mocks/older payloads used `round_height` instead.
     */
    round_id?: number;
    /**
     * Legacy/mock compatibility: do not assume this exists on live RPC.
     */
    round_height?: number;
    block_height: number;
    finalized: boolean;
    hash_timer_seq?: string;
  };
  counters?: {
    finalized_rounds?: number;
    transactions_total?: number;
    total_issuance?: number;
    accounts_total?: number;
    holders_total?: number;
    hash_timers_total?: number;
    ai_requests_total?: number;
  };
  live: {
    round_time_avg_ms: number;
    finality_time_ms: number;
    current_epoch: number;
    epoch_progress_pct: number;
    /**
     * Preferred field name for "validators online" if present.
     */
    validators_online?: number;
    /**
     * Legacy/mock compatibility: do not assume this exists on live RPC.
     */
    active_operators?: number;
  };
  /**
   * Optional: a "recent activity" window some nodes provide on /status.
   * If absent, the UI must show a placeholder and not invent history.
   */
  latest_blocks?: Array<{
    block_height: number;
    hash_timer_id: string;
    round_height?: number;
    tx_count?: number;
    age_ms?: number;
    proposer?: string;
  }>;
  latest_rounds?: Array<{
    round_height: number;
    finalized: boolean;
    block_count?: number;
    tx_count?: number;
    finality_ms?: number;
    start_hash_timer_id?: string;
    end_hash_timer_id?: string;
  }>;
  consensus?: {
    metrics_available?: boolean;
    validators?: Array<{
      validator_id: string;
      uptime_ratio_7d?: number;
      validated_blocks_7d?: number;
      missed_blocks_7d?: number;
      avg_latency_ms?: number;
      slashing_events_90d?: number;
      stake_normalized?: number;
      peer_reports_quality?: number;
      fairness_score?: number;
      reward_weight?: number;
      status?: string;
      last_seen_hash_timer?: string;
    }>;
  };
};

export type PeerInfo = {
  peer_id: string;
  addr?: string;
  agent?: string;
  last_seen_ms?: number;
};

export type PeersResponse = {
  source: RpcSource;
  peers: PeerInfo[];
};

export type IpndhtHandleRecord = {
  handle: string;
  owner?: string;
  expires_at?: string;
  hash_timer_id?: string;
};

export type IpndhtFileDescriptor = {
  /**
   * Canonical identifier (UI uses this). Some RPCs return this as `file_id`.
   */
  id: string;
  file_id?: string;
  owner?: string;
  content_hash?: string;
  size_bytes?: number;
  created_at?: string;
  mime_type?: string;
  availability?: string | number;
  dht_published?: boolean;
  tags?: string[];
  /**
   * Optional descriptor metadata (shape is RPC-defined).
   * Commonly contains `doctype` for AI materials (ai_model, ai_dataset, ai_report).
   */
  meta?: Record<string, unknown> & { doctype?: string };
  /**
   * Optional retrieval hints (shape is RPC-defined). UI must warn users to verify `content_hash`.
   */
  retrieval?: Record<string, unknown>;
  hash_timer_id?: string;
};

export type IpndhtProvider = {
  peer_id: string;
  provides: "handles" | "files" | "both";
};

export type IpndhtResponse = {
  source: RpcSource;
  sections?: {
    handles: RpcSource;
    files: RpcSource;
    providers: RpcSource;
    peers: RpcSource;
  };
  summary: {
    handles_count: number;
    files_count: number;
    providers_count?: number;
    /**
     * Peers observed in /peers (not necessarily DHT-enabled).
     */
    peers_count: number;
    /**
     * Optional: how many peers appear to advertise DHT participation.
     */
    dht_peers_count?: number;
  };
  latest_handles: IpndhtHandleRecord[];
  latest_files: IpndhtFileDescriptor[];
  providers: IpndhtProvider[];
};
