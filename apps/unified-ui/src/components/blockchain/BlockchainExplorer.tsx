'use client';

import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Search, Hash, Clock, Users, Activity } from 'lucide-react';

export function BlockchainExplorer() {
  const [searchQuery, setSearchQuery] = useState('');
  const [blocks, setBlocks] = useState([]);
  const [isLoading, setIsLoading] = useState(false);

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
            placeholder="Search by block hash, transaction hash, or address..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-10 pr-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:ring-2 focus:ring-ai-500 focus:border-transparent"
          />
        </div>

        {/* Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
            <div className="flex items-center space-x-2">
              <Hash className="w-5 h-5 text-ai-500" />
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">Latest Block</p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">#1,234,567</p>
              </div>
            </div>
          </div>
          
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
            <div className="flex items-center space-x-2">
              <Clock className="w-5 h-5 text-green-500" />
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">Block Time</p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">2.3s</p>
              </div>
            </div>
          </div>
          
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
            <div className="flex items-center space-x-2">
              <Activity className="w-5 h-5 text-blue-500" />
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">TPS</p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">1,250</p>
              </div>
            </div>
          </div>
          
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-4">
            <div className="flex items-center space-x-2">
              <Users className="w-5 h-5 text-purple-500" />
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">Validators</p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">25</p>
              </div>
            </div>
          </div>
        </div>

        {/* Recent Blocks */}
        <div>
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Recent Blocks
          </h3>
          <div className="space-y-2">
            {[1, 2, 3, 4, 5].map((block) => (
              <div key={block} className="flex items-center justify-between p-4 bg-gray-50 dark:bg-slate-700 rounded-lg">
                <div className="flex items-center space-x-4">
                  <div className="w-8 h-8 bg-ai-100 dark:bg-ai-900 rounded-full flex items-center justify-center">
                    <Hash className="w-4 h-4 text-ai-600 dark:text-ai-400" />
                  </div>
                  <div>
                    <p className="font-medium text-gray-900 dark:text-white">Block #{1234567 - block}</p>
                    <p className="text-sm text-gray-500 dark:text-gray-400">
                      {new Date(Date.now() - block * 30000).toLocaleString()}
                    </p>
                  </div>
                </div>
                <div className="text-right">
                  <p className="text-sm text-gray-600 dark:text-gray-400">42 transactions</p>
                  <p className="text-sm text-gray-500 dark:text-gray-400">2.3s</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </motion.div>
    </div>
  );
}