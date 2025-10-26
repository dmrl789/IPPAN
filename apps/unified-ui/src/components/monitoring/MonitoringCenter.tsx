'use client';

import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { 
  AlertTriangle, 
  CheckCircle, 
  Clock, 
  Server, 
  Database, 
  Network,
  Cpu,
  Memory,
  HardDrive
} from 'lucide-react';

export function MonitoringCenter() {
  const [alerts, setAlerts] = useState([
    {
      id: 1,
      type: 'warning',
      title: 'High Memory Usage',
      description: 'Memory usage has exceeded 85% on node-3',
      timestamp: new Date(Date.now() - 300000),
      status: 'active'
    },
    {
      id: 2,
      type: 'critical',
      title: 'Network Latency Spike',
      description: 'Network latency increased by 200% in the last 5 minutes',
      timestamp: new Date(Date.now() - 180000),
      status: 'active'
    },
    {
      id: 3,
      type: 'info',
      title: 'Block Production Normal',
      description: 'All validators are producing blocks within expected timeframes',
      timestamp: new Date(Date.now() - 600000),
      status: 'resolved'
    }
  ]);

  const [systemMetrics, setSystemMetrics] = useState({
    cpu: 45,
    memory: 78,
    disk: 32,
    network: 125
  });

  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    // Simulate real-time updates
    const interval = setInterval(() => {
      setSystemMetrics(prev => ({
        cpu: Math.max(0, Math.min(100, prev.cpu + (Math.random() - 0.5) * 10)),
        memory: Math.max(0, Math.min(100, prev.memory + (Math.random() - 0.5) * 5)),
        disk: Math.max(0, Math.min(100, prev.disk + (Math.random() - 0.5) * 2)),
        network: Math.max(0, prev.network + (Math.random() - 0.5) * 20)
      }));
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  const getAlertIcon = (type: string) => {
    switch (type) {
      case 'critical': return <AlertTriangle className="w-5 h-5 text-red-500" />;
      case 'warning': return <AlertTriangle className="w-5 h-5 text-yellow-500" />;
      case 'info': return <CheckCircle className="w-5 h-5 text-blue-500" />;
      default: return <Clock className="w-5 h-5 text-gray-500" />;
    }
  };

  const getAlertColor = (type: string) => {
    switch (type) {
      case 'critical': return 'border-red-200 bg-red-50 dark:bg-red-900 dark:border-red-800';
      case 'warning': return 'border-yellow-200 bg-yellow-50 dark:bg-yellow-900 dark:border-yellow-800';
      case 'info': return 'border-blue-200 bg-blue-50 dark:bg-blue-900 dark:border-blue-800';
      default: return 'border-gray-200 bg-gray-50 dark:bg-gray-900 dark:border-gray-800';
    }
  };

  const getMetricColor = (value: number) => {
    if (value >= 90) return 'text-red-500';
    if (value >= 75) return 'text-yellow-500';
    return 'text-green-500';
  };

  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
      >
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
          Monitoring Center
        </h2>

        {/* System Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center space-x-2">
                <Cpu className="w-5 h-5 text-blue-500" />
                <span className="font-medium text-gray-900 dark:text-white">CPU Usage</span>
              </div>
              <span className={`text-2xl font-bold ${getMetricColor(systemMetrics.cpu)}`}>
                {Math.round(systemMetrics.cpu)}%
              </span>
            </div>
            <div className="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2">
              <div 
                className="bg-blue-500 h-2 rounded-full transition-all duration-500"
                style={{ width: `${systemMetrics.cpu}%` }}
              />
            </div>
          </div>

          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center space-x-2">
                <Memory className="w-5 h-5 text-green-500" />
                <span className="font-medium text-gray-900 dark:text-white">Memory</span>
              </div>
              <span className={`text-2xl font-bold ${getMetricColor(systemMetrics.memory)}`}>
                {Math.round(systemMetrics.memory)}%
              </span>
            </div>
            <div className="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2">
              <div 
                className="bg-green-500 h-2 rounded-full transition-all duration-500"
                style={{ width: `${systemMetrics.memory}%` }}
              />
            </div>
          </div>

          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center space-x-2">
                <HardDrive className="w-5 h-5 text-purple-500" />
                <span className="font-medium text-gray-900 dark:text-white">Disk</span>
              </div>
              <span className={`text-2xl font-bold ${getMetricColor(systemMetrics.disk)}`}>
                {Math.round(systemMetrics.disk)}%
              </span>
            </div>
            <div className="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2">
              <div 
                className="bg-purple-500 h-2 rounded-full transition-all duration-500"
                style={{ width: `${systemMetrics.disk}%` }}
              />
            </div>
          </div>

          <div className="bg-gray-50 dark:bg-slate-700 rounded-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center space-x-2">
                <Network className="w-5 h-5 text-orange-500" />
                <span className="font-medium text-gray-900 dark:text-white">Network</span>
              </div>
              <span className="text-2xl font-bold text-orange-500">
                {Math.round(systemMetrics.network)} MB/s
              </span>
            </div>
            <div className="w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2">
              <div 
                className="bg-orange-500 h-2 rounded-full transition-all duration-500"
                style={{ width: `${Math.min(systemMetrics.network / 200, 1) * 100}%` }}
              />
            </div>
          </div>
        </div>

        {/* Alerts */}
        <div className="space-y-4">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
            Recent Alerts
          </h3>
          <div className="space-y-3">
            {alerts.map((alert) => (
              <motion.div
                key={alert.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                className={`p-4 rounded-lg border ${getAlertColor(alert.type)}`}
              >
                <div className="flex items-start space-x-3">
                  {getAlertIcon(alert.type)}
                  <div className="flex-1">
                    <div className="flex items-center justify-between">
                      <h4 className="font-medium text-gray-900 dark:text-white">
                        {alert.title}
                      </h4>
                      <span className="text-sm text-gray-500 dark:text-gray-400">
                        {alert.timestamp.toLocaleTimeString()}
                      </span>
                    </div>
                    <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                      {alert.description}
                    </p>
                    <div className="flex items-center space-x-2 mt-2">
                      <span className={`px-2 py-1 rounded-full text-xs font-medium ${
                        alert.status === 'active' 
                          ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                          : 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                      }`}>
                        {alert.status.toUpperCase()}
                      </span>
                    </div>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        </div>

        {/* AI Recommendations */}
        <div className="mt-8 bg-gradient-to-r from-ai-50 to-purple-50 dark:from-ai-900 dark:to-purple-900 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            AI Recommendations
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="bg-white dark:bg-slate-800 rounded-lg p-4">
              <h4 className="font-medium text-gray-900 dark:text-white mb-2">
                Memory Optimization
              </h4>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Consider increasing memory allocation for node-3 to prevent performance degradation.
              </p>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg p-4">
              <h4 className="font-medium text-gray-900 dark:text-white mb-2">
                Network Monitoring
              </h4>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Implement additional network monitoring to detect latency spikes earlier.
              </p>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}