'use client';

import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Search, Hash, Clock, Users, Activity } from 'lucide-react';
import {
  rpcGetBlock,
  rpcGetRecentTxs,
  rpcGetRound,
  rpcGetStatus,
  rpcGetTx,
  type RpcTxRecentEntry,
} from '@/lib/ippan-rpc';

export function BlockchainExplorer() {
  const [searchQuery, setSearchQuery] = useState('');
  const [status, setStatus] = useState<Record<string, any> | null>(null);
  const [recentTxs, setRecentTxs] = useState<RpcTxRecentEntry[]>([]);
  const [selectedTx, setSelectedTx] = useState<any | null>(null);
  const [selectedBlock, setSelectedBlock] = useState<any | null>(null);
  const [selectedRound, setSelectedRound] = useState<any | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setIsLoading(true);
    setError(null);

    const [statusResp, recentResp] = await Promise.all([
      rpcGetStatus(),
      rpcGetRecentTxs(50),
    ]);

    if (statusResp.error) {
      setError(statusResp.error);
    } else {
      setStatus(statusResp.data || null);
    }

    if (recentResp.error) {
      setError((prev) => prev || recentResp.error || 'Failed to load recent txs');
    } else {
      setRecentTxs(recentResp.data || []);
    }

    setIsLoading(false);
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function tryLookup(query: string) {
    const q = query.trim().replace(/^0x/, '').toLowerCase();
    if (!q) return;

    setIsLoading(true);
    setError(null);
    setSelectedTx(null);
    setSelectedBlock(null);
    setSelectedRound(null);

    // Hex 32-byte identifiers are the common case for tx_id and block_hash.
    const looksLikeHex32 = /^[0-9a-f]{64}$/.test(q);

    if (looksLikeHex32) {
      const txResp = await rpcGetTx(q);
      if (!txResp.error && txResp.data) {
        setSelectedTx(txResp.data);
        setIsLoading(false);
        return;
      }

      const blockResp = await rpcGetBlock(q);
      if (!blockResp.error && blockResp.data) {
        setSelectedBlock(blockResp.data);
        setIsLoading(false);
        return;
      }

      setError(txResp.error || blockResp.error || 'Not found');
      setIsLoading(false);
      return;
    }

    // Round ids are numeric.
    if (/^\d+$/.test(q)) {
      const roundResp = await rpcGetRound(q);
      if (!roundResp.error && roundResp.data) {
        setSelectedRound(roundResp.data);
      } else {
        setError(roundResp.error || 'Round not found');
      }
      setIsLoading(false);
      return;
    }

    setError('Paste a tx_id / block_hash (64 hex) or round_id (number).');
    setIsLoading(false);
  }

  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
      >
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
          Blockchain Explorer
        </h2>
        
        {/* Search Bar */}
        <div className="relative mb-6">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-5 h-5" />
          <input
            type="text"
            placeholder="Paste tx_id / block_hash (64 hex) or round_id..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void tryLookup(searchQuery);
            }}
            className="w-full pl-10 pr-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:ring-2 focus:ring-ai-500 focus:border-transparent"
          />
        </div>

        {error && (
          <div className="mb-6 rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-700 dark:border-red-900/40 dark:bg-red-900/20 dark:text-red-200">
            {error}
          </div>
        )}

        {/* Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
            <div className="flex items-center space-x-2">
              <Hash className="w-5 h-5 text-ai-500" />
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">Tip Block</p>
                <p className="text-xs font-mono text-gray-900 dark:text-white break-all">
                  {status?.tip_block_hash || '—'}
                </p>
              </div>
            </div>
          </div>
          
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
            <div className="flex items-center space-x-2">
              <Clock className="w-5 h-5 text-green-500" />
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">HashTimer</p>
                <p className="text-xs font-mono text-gray-900 dark:text-white break-all">
                  {status?.current_hashtimer || '—'}
                </p>
              </div>
            </div>
          </div>
          
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
            <div className="flex items-center space-x-2">
              <Activity className="w-5 h-5 text-blue-500" />
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">Mempool</p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                  {status?.mempool_size ?? '—'}
                </p>
              </div>
            </div>
          </div>
          
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
            <div className="flex items-center space-x-2">
              <Users className="w-5 h-5 text-purple-500" />
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">Peers</p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                  {status?.peer_count ?? status?.peers?.length ?? '—'}
                </p>
              </div>
            </div>
          </div>
        </div>

        {/* Recent Transactions */}
        <div>
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Recent Transactions
          </h3>
          <div className="space-y-2">
            {recentTxs.length === 0 ? (
              <div className="p-4 bg-gray-50 dark:bg-slate-700 rounded-lg text-sm text-gray-600 dark:text-gray-300">
                {isLoading ? 'Loading…' : 'No recent transactions yet.'}
              </div>
            ) : (
              recentTxs.map((tx) => (
                <button
                  key={tx.tx_id}
                  type="button"
                  onClick={async () => {
                    const resp = await rpcGetTx(tx.tx_id);
                    if (resp.error) {
                      setError(resp.error);
                      return;
                    }
                    setSelectedTx(resp.data || null);
                  }}
                  className="w-full text-left flex items-center justify-between p-4 bg-gray-50 dark:bg-slate-700 rounded-lg hover:bg-gray-100 dark:hover:bg-slate-600 transition-colors"
                >
                  <div className="flex items-center space-x-4">
                    <div className="w-8 h-8 bg-ai-100 dark:bg-ai-900 rounded-full flex items-center justify-center">
                      <Hash className="w-4 h-4 text-ai-600 dark:text-ai-400" />
                    </div>
                    <div>
                      <p className="font-mono text-xs text-gray-900 dark:text-white break-all">
                        {tx.tx_id}
                      </p>
                      <p className="text-sm text-gray-500 dark:text-gray-400">
                        {tx.status}
                        {tx.included?.round_id !== undefined && tx.included?.round_id !== null
                          ? ` • round ${tx.included.round_id}`
                          : ''}
                      </p>
                    </div>
                  </div>
                  <div className="text-right">
                    <p className="text-xs font-mono text-gray-600 dark:text-gray-300 break-all">
                      {tx.tx_hashtimer || '—'}
                    </p>
                  </div>
                </button>
              ))
            )}
          </div>
        </div>

        {/* Detail panels */}
        {(selectedTx || selectedBlock || selectedRound) && (
          <div className="mt-6 grid grid-cols-1 lg:grid-cols-3 gap-4">
            <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
              <div className="flex items-center justify-between mb-2">
                <h4 className="font-semibold text-gray-900 dark:text-white">Tx</h4>
              </div>
              {selectedTx ? (
                <pre className="text-xs overflow-auto whitespace-pre-wrap text-gray-800 dark:text-gray-200">
                  {JSON.stringify(selectedTx, null, 2)}
                </pre>
              ) : (
                <p className="text-sm text-gray-600 dark:text-gray-300">No tx selected.</p>
              )}
            </div>

            <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
              <div className="flex items-center justify-between mb-2">
                <h4 className="font-semibold text-gray-900 dark:text-white">Block</h4>
              </div>
              {selectedBlock ? (
                <pre className="text-xs overflow-auto whitespace-pre-wrap text-gray-800 dark:text-gray-200">
                  {JSON.stringify(selectedBlock, null, 2)}
                </pre>
              ) : (
                <p className="text-sm text-gray-600 dark:text-gray-300">No block selected.</p>
              )}
            </div>

            <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
              <div className="flex items-center justify-between mb-2">
                <h4 className="font-semibold text-gray-900 dark:text-white">Round</h4>
              </div>
              {selectedRound ? (
                <pre className="text-xs overflow-auto whitespace-pre-wrap text-gray-800 dark:text-gray-200">
                  {JSON.stringify(selectedRound, null, 2)}
                </pre>
              ) : (
                <p className="text-sm text-gray-600 dark:text-gray-300">No round selected.</p>
              )}
            </div>
          </div>
        )}
      </motion.div>
    </div>
  );
}