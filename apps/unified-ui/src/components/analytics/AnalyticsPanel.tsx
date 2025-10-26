'use client';

import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { BarChart3, TrendingUp, Activity, Users, DollarSign } from 'lucide-react';

export function AnalyticsPanel() {
  const [timeRange, setTimeRange] = useState('24h');
  const [isLoading, setIsLoading] = useState(false);

  const mockData = {
    transactions: [
      { time: '00:00', count: 120 },
      { time: '04:00', count: 180 },
      { time: '08:00', count: 250 },
      { time: '12:00', count: 320 },
      { time: '16:00', count: 280 },
      { time: '20:00', count: 200 },
    ],
    gasUsage: [
      { time: '00:00', gas: 1500000 },
      { time: '04:00', gas: 1800000 },
      { time: '08:00', gas: 2200000 },
      { time: '12:00', gas: 2500000 },
      { time: '16:00', gas: 2100000 },
      { time: '20:00', gas: 1900000 },
    ],
    activeAddresses: 1250,
    totalVolume: 1250000,
    avgGasPrice: 25.5,
  };

  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
      >
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Analytics Dashboard
          </h2>
          <div className="flex items-center space-x-2">
            <select
              value={timeRange}
              onChange={(e) => setTimeRange(e.target.value)}
              className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-slate-700 text-gray-900 dark:text-white"
            >
              <option value="1h">Last Hour</option>
              <option value="24h">Last 24 Hours</option>
              <option value="7d">Last 7 Days</option>
              <option value="30d">Last 30 Days</option>
            </select>
          </div>
        </div>

        {/* Key Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
          <div className="bg-gradient-to-r from-blue-500 to-blue-600 rounded-lg p-6 text-white">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-blue-100">Total Transactions</p>
                <p className="text-3xl font-bold">1,234,567</p>
                <p className="text-blue-100 text-sm">+12.5% from yesterday</p>
              </div>
              <Activity className="w-8 h-8 text-blue-200" />
            </div>
          </div>

          <div className="bg-gradient-to-r from-green-500 to-green-600 rounded-lg p-6 text-white">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-green-100">Active Addresses</p>
                <p className="text-3xl font-bold">{mockData.activeAddresses.toLocaleString()}</p>
                <p className="text-green-100 text-sm">+8.2% from yesterday</p>
              </div>
              <Users className="w-8 h-8 text-green-200" />
            </div>
          </div>

          <div className="bg-gradient-to-r from-purple-500 to-purple-600 rounded-lg p-6 text-white">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-purple-100">Total Volume</p>
                <p className="text-3xl font-bold">${mockData.totalVolume.toLocaleString()}</p>
                <p className="text-purple-100 text-sm">+15.3% from yesterday</p>
              </div>
              <DollarSign className="w-8 h-8 text-purple-200" />
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500 to-orange-600 rounded-lg p-6 text-white">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-orange-100">Avg Gas Price</p>
                <p className="text-3xl font-bold">{mockData.avgGasPrice} Gwei</p>
                <p className="text-orange-100 text-sm">-5.1% from yesterday</p>
              </div>
              <TrendingUp className="w-8 h-8 text-orange-200" />
            </div>
          </div>
        </div>

        {/* Charts */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Transaction Volume Chart */}
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              Transaction Volume
            </h3>
            <div className="h-64 flex items-end space-x-2">
              {mockData.transactions.map((point, index) => (
                <div key={index} className="flex-1 flex flex-col items-center">
                  <div
                    className="w-full bg-blue-500 rounded-t"
                    style={{ height: `${(point.count / 400) * 200}px` }}
                  />
                  <span className="text-xs text-gray-500 dark:text-gray-400 mt-2">
                    {point.time}
                  </span>
                </div>
              ))}
            </div>
          </div>

          {/* Gas Usage Chart */}
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              Gas Usage
            </h3>
            <div className="h-64 flex items-end space-x-2">
              {mockData.gasUsage.map((point, index) => (
                <div key={index} className="flex-1 flex flex-col items-center">
                  <div
                    className="w-full bg-green-500 rounded-t"
                    style={{ height: `${(point.gas / 3000000) * 200}px` }}
                  />
                  <span className="text-xs text-gray-500 dark:text-gray-400 mt-2">
                    {point.time}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* AI Insights */}
        <div className="mt-8 bg-gradient-to-r from-ai-50 to-purple-50 dark:from-ai-900 dark:to-purple-900 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center">
            <BarChart3 className="w-5 h-5 mr-2 text-ai-500" />
            AI Insights
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="bg-white dark:bg-slate-800 rounded-lg p-4">
              <h4 className="font-medium text-gray-900 dark:text-white mb-2">
                Peak Usage Prediction
              </h4>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                AI predicts peak transaction volume will occur at 2:30 PM today, 
                with 15% higher than average activity.
              </p>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg p-4">
              <h4 className="font-medium text-gray-900 dark:text-white mb-2">
                Gas Optimization
              </h4>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Recommended gas price of 22 Gwei for optimal transaction inclusion 
                based on current network conditions.
              </p>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}