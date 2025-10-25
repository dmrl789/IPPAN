'use client';

import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { 
  Brain, 
  Zap, 
  Shield, 
  BarChart3, 
  Activity,
  TrendingUp,
  AlertTriangle,
  CheckCircle,
  Clock,
  Cpu,
  Database,
  Network
} from 'lucide-react';
import { useAI } from '@/contexts/AIContext';
import { useWebSocket } from '@/contexts/WebSocketContext';

interface AIMetric {
  name: string;
  value: number;
  unit: string;
  trend: 'up' | 'down' | 'stable';
  change: number;
  status: 'good' | 'warning' | 'critical';
}

interface AIInsight {
  id: string;
  type: 'performance' | 'security' | 'optimization' | 'prediction';
  title: string;
  description: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  confidence: number;
  timestamp: Date;
}

export function AIDashboard() {
  const { aiStatus } = useAI();
  const { messages, subscribe } = useWebSocket();
  const [metrics, setMetrics] = useState<AIMetric[]>([]);
  const [insights, setInsights] = useState<AIInsight[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Subscribe to AI metrics updates
    const unsubscribe = subscribe('ai_metrics', (data) => {
      setMetrics(data.metrics || []);
    });

    // Subscribe to AI insights updates
    const unsubscribeInsights = subscribe('ai_insights', (data) => {
      setInsights(data.insights || []);
    });

    // Load initial data
    loadAIData();

    return () => {
      unsubscribe();
      unsubscribeInsights();
    };
  }, [subscribe]);

  const loadAIData = async () => {
    try {
      setIsLoading(true);
      
      // Simulate loading AI data
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      // Mock data for demonstration
      setMetrics([
        {
          name: 'AI Processing Speed',
          value: 1250,
          unit: 'ops/sec',
          trend: 'up',
          change: 12.5,
          status: 'good'
        },
        {
          name: 'Model Accuracy',
          value: 94.2,
          unit: '%',
          trend: 'up',
          change: 2.1,
          status: 'good'
        },
        {
          name: 'Memory Usage',
          value: 78.5,
          unit: '%',
          trend: 'up',
          change: 5.2,
          status: 'warning'
        },
        {
          name: 'Response Time',
          value: 45,
          unit: 'ms',
          trend: 'down',
          change: -8.3,
          status: 'good'
        }
      ]);

      setInsights([
        {
          id: '1',
          type: 'optimization',
          title: 'Gas Fee Optimization Opportunity',
          description: 'AI detected 15% potential gas savings in recent transactions',
          severity: 'medium',
          confidence: 0.87,
          timestamp: new Date()
        },
        {
          id: '2',
          type: 'security',
          title: 'Suspicious Activity Detected',
          description: 'Unusual transaction patterns detected in address 0x123...',
          severity: 'high',
          confidence: 0.92,
          timestamp: new Date()
        },
        {
          id: '3',
          type: 'performance',
          title: 'Network Congestion Predicted',
          description: 'AI predicts high network activity in next 2 hours',
          severity: 'low',
          confidence: 0.75,
          timestamp: new Date()
        }
      ]);
    } catch (error) {
      console.error('Failed to load AI data:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'good': return 'text-green-500';
      case 'warning': return 'text-yellow-500';
      case 'critical': return 'text-red-500';
      default: return 'text-gray-500';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'good': return <CheckCircle className="w-4 h-4" />;
      case 'warning': return <AlertTriangle className="w-4 h-4" />;
      case 'critical': return <AlertTriangle className="w-4 h-4" />;
      default: return <Clock className="w-4 h-4" />;
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'low': return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200';
      case 'medium': return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
      case 'high': return 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200';
      case 'critical': return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200';
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200';
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-ai-500"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* AI Status Overview */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6"
      >
        <div className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600 dark:text-gray-400">AI Status</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {aiStatus.isActive ? 'Active' : 'Inactive'}
              </p>
            </div>
            <div className={`p-3 rounded-full ${aiStatus.isActive ? 'bg-green-100 dark:bg-green-900' : 'bg-red-100 dark:bg-red-900'}`}>
              <Brain className={`w-6 h-6 ${aiStatus.isActive ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}`} />
            </div>
          </div>
        </div>

        <div className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600 dark:text-gray-400">Models Active</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">4</p>
            </div>
            <div className="p-3 rounded-full bg-ai-100 dark:bg-ai-900">
              <Cpu className="w-6 h-6 text-ai-600 dark:text-ai-400" />
            </div>
          </div>
        </div>

        <div className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600 dark:text-gray-400">Insights Generated</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">{insights.length}</p>
            </div>
            <div className="p-3 rounded-full bg-purple-100 dark:bg-purple-900">
              <BarChart3 className="w-6 h-6 text-purple-600 dark:text-purple-400" />
            </div>
          </div>
        </div>

        <div className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600 dark:text-gray-400">Optimizations</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">23</p>
            </div>
            <div className="p-3 rounded-full bg-yellow-100 dark:bg-yellow-900">
              <Zap className="w-6 h-6 text-yellow-600 dark:text-yellow-400" />
            </div>
          </div>
        </div>
      </motion.div>

      {/* AI Metrics */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
        className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
      >
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">AI Performance Metrics</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {metrics.map((metric, index) => (
            <div key={index} className="p-4 rounded-lg bg-gray-50 dark:bg-slate-700">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-medium text-gray-600 dark:text-gray-400">{metric.name}</span>
                <div className={`flex items-center ${getStatusColor(metric.status)}`}>
                  {getStatusIcon(metric.status)}
                </div>
              </div>
              <div className="flex items-center space-x-2">
                <span className="text-2xl font-bold text-gray-900 dark:text-white">
                  {metric.value.toLocaleString()}
                </span>
                <span className="text-sm text-gray-500 dark:text-gray-400">{metric.unit}</span>
              </div>
              <div className="flex items-center mt-2">
                <TrendingUp className={`w-4 h-4 mr-1 ${
                  metric.trend === 'up' ? 'text-green-500' : 
                  metric.trend === 'down' ? 'text-red-500' : 'text-gray-500'
                }`} />
                <span className={`text-sm ${
                  metric.trend === 'up' ? 'text-green-500' : 
                  metric.trend === 'down' ? 'text-red-500' : 'text-gray-500'
                }`}>
                  {metric.change > 0 ? '+' : ''}{metric.change.toFixed(1)}%
                </span>
              </div>
            </div>
          ))}
        </div>
      </motion.div>

      {/* AI Insights */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.2 }}
        className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
      >
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">AI Insights & Recommendations</h3>
        <div className="space-y-4">
          {insights.map((insight) => (
            <div key={insight.id} className="p-4 rounded-lg border border-gray-200 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-slate-700 transition-colors">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center space-x-2 mb-2">
                    <span className={`px-2 py-1 rounded-full text-xs font-medium ${getSeverityColor(insight.severity)}`}>
                      {insight.severity.toUpperCase()}
                    </span>
                    <span className="text-xs text-gray-500 dark:text-gray-400">
                      {insight.type.toUpperCase()}
                    </span>
                    <span className="text-xs text-gray-500 dark:text-gray-400">
                      {Math.round(insight.confidence * 100)}% confidence
                    </span>
                  </div>
                  <h4 className="font-medium text-gray-900 dark:text-white mb-1">{insight.title}</h4>
                  <p className="text-sm text-gray-600 dark:text-gray-400">{insight.description}</p>
                </div>
                <div className="ml-4">
                  <span className="text-xs text-gray-500 dark:text-gray-400">
                    {insight.timestamp.toLocaleTimeString()}
                  </span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </motion.div>

      {/* Real-time Activity */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.3 }}
        className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
      >
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Real-time AI Activity</h3>
        <div className="space-y-3">
          {messages.slice(-5).map((message, index) => (
            <div key={index} className="flex items-center space-x-3 p-3 rounded-lg bg-gray-50 dark:bg-slate-700">
              <Activity className="w-4 h-4 text-ai-500" />
              <div className="flex-1">
                <p className="text-sm text-gray-900 dark:text-white">{message.type}</p>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {new Date(message.timestamp).toLocaleTimeString()}
                </p>
              </div>
            </div>
          ))}
        </div>
      </motion.div>
    </div>
  );
}