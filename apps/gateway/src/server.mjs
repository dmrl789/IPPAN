#!/usr/bin/env node
/**
 * IPPAN Gateway — Explorer Service Layer
 * --------------------------------------
 * Serves normalized data to ippan.com explorer.
 * Fetches from upstream RPC + WS endpoints and caches summaries,
 * metrics, block/transaction/account data.
 */

import express from "express";
import morgan from "morgan";
import cors from "cors";
import { createProxyMiddleware } from "http-proxy-middleware";

const app = express();
const PORT = process.env.PORT || 8080;
const UPSTREAM = process.env.UPSTREAM || "http://127.0.0.1:8545";
const CACHE_TTL = Number(process.env.CACHE_TTL || 3); // seconds

class MemoryCache {
  constructor(ttlSeconds) {
    this.ttlMs = Math.max(0, Number(ttlSeconds) * 1000);
    this.map = new Map();
  }

  get(key) {
    const entry = this.map.get(key);
    if (!entry) return null;
    if (this.ttlMs > 0 && Date.now() > entry.expiresAt) {
      this.map.delete(key);
      return null;
    }
    return entry.value;
  }

  set(key, value) {
    const expiresAt = this.ttlMs > 0 ? Date.now() + this.ttlMs : Number.POSITIVE_INFINITY;
    this.map.set(key, { value, expiresAt });
    return value;
  }
}

const cache = new MemoryCache(CACHE_TTL);

app.use(morgan("tiny"));
app.use(express.json({ limit: "2mb" }));
app.use(cors({ origin: "*" }));

/** ------------------ Utilities ------------------- */

async function cachedFetch(key, url, normalize) {
  const cached = cache.get(key);
  if (cached) return cached;
  const r = await fetch(url);
  const data = await r.json();
  const result = normalize ? normalize(data) : data;
  cache.set(key, result);
  return result;
}

function handleError(res, e) {
  console.error("[Gateway error]", e);
  res.status(500).json({
    ok: false,
    error: e.message || "Internal Gateway Error",
  });
}

/** ------------------ Normalizers ------------------- */

const normalizeHealth = (data) => ({
  status: data?.status || "unknown",
  version: data?.version || "n/a",
  height: data?.height || 0,
  peers: data?.peers || 0,
  uptime: data?.uptime || 0,
  lastRound: data?.last_round || null,
});

const normalizeBlock = (b) => ({
  id: b.id,
  height: b.height,
  time: b.timestamp,
  hashTimer: b.hash_timer,
  txs: b.transactions?.length || 0,
  validator: b.validator || "unknown",
});

const normalizeTx = (t) => ({
  id: t.id,
  block: t.block_id,
  sender: t.sender,
  receiver: t.receiver,
  amount: t.amount,
  fee: t.fee,
  hashTimer: t.hash_timer,
  time: t.timestamp,
});

/** ------------------ Routes ------------------- */

/* Root health check */
app.get("/", async (req, res) => {
  try {
    const health = await cachedFetch(
      "health",
      `${UPSTREAM}/health`,
      normalizeHealth
    );
    res.json({ ok: true, ...health });
  } catch (e) {
    handleError(res, e);
  }
});

/* Explorer summary */
app.get("/api/explorer/summary", async (req, res) => {
  try {
    const [health, blocks] = await Promise.all([
      cachedFetch("health", `${UPSTREAM}/health`, normalizeHealth),
      cachedFetch("blocks", `${UPSTREAM}/blocks/latest?limit=10`, (d) =>
        (d?.blocks || []).map(normalizeBlock)
      ),
    ]);
    res.json({ ok: true, health, latestBlocks: blocks });
  } catch (e) {
    handleError(res, e);
  }
});

/* Blocks list */
app.get("/api/explorer/blocks", async (req, res) => {
  try {
    const { limit = 25, offset = 0 } = req.query;
    const blocks = await cachedFetch(
      `blocks_${limit}_${offset}`,
      `${UPSTREAM}/blocks?limit=${limit}&offset=${offset}`,
      (d) => (d?.blocks || []).map(normalizeBlock)
    );
    res.json({ ok: true, blocks });
  } catch (e) {
    handleError(res, e);
  }
});

/* Single block */
app.get("/api/explorer/block/:id", async (req, res) => {
  try {
    const block = await cachedFetch(
      `block_${req.params.id}`,
      `${UPSTREAM}/blocks/${req.params.id}`,
      normalizeBlock
    );
    res.json({ ok: true, block });
  } catch (e) {
    handleError(res, e);
  }
});

/* Transactions list */
app.get("/api/explorer/transactions", async (req, res) => {
  try {
    const { limit = 25, offset = 0 } = req.query;
    const txs = await cachedFetch(
      `txs_${limit}_${offset}`,
      `${UPSTREAM}/transactions?limit=${limit}&offset=${offset}`,
      (d) => (d?.transactions || []).map(normalizeTx)
    );
    res.json({ ok: true, transactions: txs });
  } catch (e) {
    handleError(res, e);
  }
});

/* Single transaction */
app.get("/api/explorer/tx/:id", async (req, res) => {
  try {
    const tx = await cachedFetch(
      `tx_${req.params.id}`,
      `${UPSTREAM}/transactions/${req.params.id}`,
      normalizeTx
    );
    res.json({ ok: true, transaction: tx });
  } catch (e) {
    handleError(res, e);
  }
});

/* Account info */
app.get("/api/explorer/account/:id", async (req, res) => {
  try {
    const acc = await cachedFetch(
      `acc_${req.params.id}`,
      `${UPSTREAM}/accounts/${req.params.id}`,
      (d) => ({
        id: d.id,
        balance: d.balance,
        nonce: d.nonce,
        lastTx: d.last_tx,
      })
    );
    res.json({ ok: true, account: acc });
  } catch (e) {
    handleError(res, e);
  }
});

/* Search endpoint: block, tx, or account */
app.get("/api/explorer/search/:term", async (req, res) => {
  try {
    const term = req.params.term;
    const results = await cachedFetch(
      `search_${term}`,
      `${UPSTREAM}/search/${term}`,
      (d) => d
    );
    res.json({ ok: true, results });
  } catch (e) {
    handleError(res, e);
  }
});

/* Proxy raw API (for debug) */
app.use(
  "/rpc",
  createProxyMiddleware({
    target: UPSTREAM,
    changeOrigin: true,
    pathRewrite: { "^/rpc": "" },
    onError(err, req, res) {
      handleError(res, err);
    },
  })
);

/** ------------------ Start Server ------------------- */

app.listen(PORT, () => {
  console.log(`✅ IPPAN Explorer Gateway running on :${PORT}`);
  console.log(`Upstream: ${UPSTREAM}`);
});
