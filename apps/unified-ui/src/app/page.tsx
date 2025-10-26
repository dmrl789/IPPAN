'use client';

import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { 
  Brain, 
  Zap, 
  Shield, 
  BarChart3, 
  Code, 
  Settings,
  Play,
  Pause,
  RefreshCw
} from 'lucide-react';
import { AIDashboard } from '@/components/ai/AIDashboard';
import { BlockchainExplorer } from '@/components/blockchain/BlockchainExplorer';
import { SmartContractStudio } from '@/components/smart-contracts/SmartContractStudio';
import { AnalyticsPanel } from '@/components/analytics/AnalyticsPanel';
import { MonitoringCenter } from '@/components/monitoring/MonitoringCenter';
import { MetaAgentDashboard } from '@/components/metaagent/MetaAgentDashboard';
import { Navigation } from '@/components/layout/Navigation';
import { Header } from '@/components/layout/Header';
import { useAI } from '@/contexts/AIContext';

const features = [
  {
    icon: Brain,
    title: 'AI-Powered Consensus',
    description: 'Advanced AI algorithms for validator selection and network optimization',
    color: 'from-ai-500 to-purple-600',
  },
  {
    icon: Zap,
    title: 'Smart Transactions',
    description: 'AI-optimized transaction processing and gas fee management',
    color: 'from-yellow-500 to-orange-600',
  },
  {
    icon: Shield,
    title: 'Security Analysis',
    description: 'Real-time security monitoring and threat detection',
    color: 'from-green-500 to-emerald-600',
  },
  {
    icon: BarChart3,
    title: 'Predictive Analytics',
    description: 'Machine learning insights for network performance and trends',
    color: 'from-blue-500 to-cyan-600',
  },
  {
    icon: Code,
    title: 'Smart Contract AI',
    description: 'AI-assisted contract development and optimization',
    color: 'from-purple-500 to-pink-600',
  },
  {
    icon: Settings,
    title: 'Auto-Optimization',
    description: 'Automatic network parameter tuning and performance optimization',
    color: 'from-gray-500 to-slate-600',
  },
];

export default function HomePage() {
  const [activeTab, setActiveTab] = useState('dashboard');
  const [isAIActive, setIsAIActive] = useState(true);
  const { aiStatus, toggleAI } = useAI();

  const tabs = [
    { id: 'dashboard', label: 'AI Dashboard', icon: Brain },
    { id: 'metaagent', label: 'MetaAgent', icon: Users },
    { id: 'explorer', label: 'Blockchain Explorer', icon: BarChart3 },
    { id: 'contracts', label: 'Smart Contracts', icon: Code },
    { id: 'analytics', label: 'Analytics', icon: BarChart3 },
    { id: 'monitoring', label: 'Monitoring', icon: Shield },
  ];

  const renderContent = () => {
    switch (activeTab) {
      case 'dashboard':
        return <AIDashboard />;
      case 'metaagent':
        return <MetaAgentDashboard />;
      case 'explorer':
        return <BlockchainExplorer />;
      case 'contracts':
        return <SmartContractStudio />;
      case 'analytics':
        return <AnalyticsPanel />;
      case 'monitoring':
        return <MonitoringCenter />;
      default:
        return <AIDashboard />;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 to-blue-50 dark:from-slate-900 dark:to-blue-900">
      <Header />
      
      <div className="flex">
        <Navigation 
          tabs={tabs} 
          activeTab={activeTab} 
          onTabChange={setActiveTab}
        />
        
        <main className="flex-1 p-6">
          {/* AI Status Bar */}
          <motion.div 
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            className="mb-6"
          >
            <div className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-4 border border-gray-200 dark:border-gray-700">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-3">
                  <div className={`w-3 h-3 rounded-full ${aiStatus.isActive ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
                  <span className="font-medium text-gray-900 dark:text-white">
                    AI System {aiStatus.isActive ? 'Active' : 'Inactive'}
                  </span>
                  <span className="text-sm text-gray-500 dark:text-gray-400">
                    v{aiStatus.version}
                  </span>
                </div>
                
                <div className="flex items-center space-x-2">
                  <button
                    onClick={toggleAI}
                    className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                      aiStatus.isActive
                        ? 'bg-red-500 hover:bg-red-600 text-white'
                        : 'bg-green-500 hover:bg-green-600 text-white'
                    }`}
                  >
                    {aiStatus.isActive ? (
                      <>
                        <Pause className="w-4 h-4 mr-2 inline" />
                        Disable AI
                      </>
                    ) : (
                      <>
                        <Play className="w-4 h-4 mr-2 inline" />
                        Enable AI
                      </>
                    )}
                  </button>
                  
                  <button className="p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 transition-colors">
                    <RefreshCw className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          </motion.div>

          {/* Features Grid */}
          {activeTab === 'dashboard' && (
            <motion.div 
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="mb-8"
            >
              <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-6">
                AI-Powered Features
              </h2>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {features.map((feature, index) => (
                  <motion.div
                    key={feature.title}
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: index * 0.1 }}
                    className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 card-hover border border-gray-200 dark:border-gray-700"
                  >
                    <div className={`w-12 h-12 rounded-lg bg-gradient-to-r ${feature.color} flex items-center justify-center mb-4`}>
                      <feature.icon className="w-6 h-6 text-white" />
                    </div>
                    <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
                      {feature.title}
                    </h3>
                    <p className="text-gray-600 dark:text-gray-400 text-sm">
                      {feature.description}
                    </p>
                  </motion.div>
                ))}
              </div>
            </motion.div>
          )}

          {/* Main Content */}
          <motion.div
            key={activeTab}
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.3 }}
          >
            {renderContent()}
          </motion.div>
        </main>
      </div>
    </div>
  );
}