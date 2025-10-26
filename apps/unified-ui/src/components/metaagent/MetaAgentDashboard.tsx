'use client';

import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { 
  Brain, 
  Users, 
  Lock, 
  CheckCircle, 
  AlertTriangle, 
  Clock, 
  Activity,
  RefreshCw,
  GitBranch,
  FileText,
  BarChart3,
  Zap
} from 'lucide-react';

interface AgentAssignment {
  id: string;
  name: string;
  color: string;
  activeIssues: number;
  lastActivity: string;
  status: 'active' | 'idle' | 'busy';
}

interface CrateLock {
  crate: string;
  lockedBy: string;
  lockedAt: string;
  prNumber: number;
}

interface Approval {
  prNumber: number;
  approvedAt: string;
  status: 'approved' | 'pending' | 'rejected';
}

interface Conflict {
  prNumber: number;
  crate: string;
  detectedAt: string;
  status: 'resolved' | 'pending';
}

const agents: AgentAssignment[] = [
  { id: 'agent-alpha', name: 'Agent Alpha', color: '#A1D6FF', activeIssues: 3, lastActivity: '2m ago', status: 'active' },
  { id: 'agent-beta', name: 'Agent Beta', color: '#A1FFA1', activeIssues: 1, lastActivity: '5m ago', status: 'idle' },
  { id: 'agent-gamma', name: 'Agent Gamma', color: '#FFA1A1', activeIssues: 0, lastActivity: '1h ago', status: 'idle' },
  { id: 'agent-delta', name: 'Agent Delta', color: '#FFFFA1', activeIssues: 2, lastActivity: '3m ago', status: 'busy' },
  { id: 'agent-epsilon', name: 'Agent Epsilon', color: '#FFA1FF', activeIssues: 1, lastActivity: '10m ago', status: 'active' },
  { id: 'agent-zeta', name: 'Agent Zeta', color: '#A1FFFF', activeIssues: 0, lastActivity: '30m ago', status: 'idle' },
  { id: 'agent-theta', name: 'Agent Theta', color: '#FFD6A1', activeIssues: 4, lastActivity: '1m ago', status: 'busy' },
  { id: 'agent-lambda', name: 'Agent Lambda', color: '#D6A1FF', activeIssues: 1, lastActivity: '7m ago', status: 'active' },
];

const mockCrateLocks: CrateLock[] = [
  { crate: 'crates/consensus', lockedBy: 'PR #123', lockedAt: '2024-01-15T10:30:00Z', prNumber: 123 },
  { crate: 'crates/network', lockedBy: 'PR #125', lockedAt: '2024-01-15T11:15:00Z', prNumber: 125 },
];

const mockApprovals: Approval[] = [
  { prNumber: 120, approvedAt: '2024-01-15T09:45:00Z', status: 'approved' },
  { prNumber: 121, approvedAt: '2024-01-15T10:20:00Z', status: 'approved' },
  { prNumber: 122, approvedAt: '2024-01-15T11:00:00Z', status: 'pending' },
];

const mockConflicts: Conflict[] = [
  { prNumber: 124, crate: 'crates/consensus', detectedAt: '2024-01-15T10:35:00Z', status: 'pending' },
  { prNumber: 126, crate: 'crates/network', detectedAt: '2024-01-15T11:20:00Z', status: 'resolved' },
];

export function MetaAgentDashboard() {
  const [isLoading, setIsLoading] = useState(true);
  const [lastRefresh, setLastRefresh] = useState(new Date());
  const [activeTab, setActiveTab] = useState('overview');

  useEffect(() => {
    // Simulate loading
    const timer = setTimeout(() => setIsLoading(false), 1000);
    return () => clearTimeout(timer);
  }, []);

  const handleRefresh = () => {
    setIsLoading(true);
    setTimeout(() => {
      setIsLoading(false);
      setLastRefresh(new Date());
    }, 1000);
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active': return 'bg-green-500';
      case 'busy': return 'bg-yellow-500';
      case 'idle': return 'bg-gray-400';
      default: return 'bg-gray-400';
    }
  };

  const getStatusText = (status: string) => {
    switch (status) {
      case 'active': return 'Active';
      case 'busy': return 'Busy';
      case 'idle': return 'Idle';
      default: return 'Unknown';
    }
  };

  const tabs = [
    { id: 'overview', label: 'Overview', icon: BarChart3 },
    { id: 'agents', label: 'Agents', icon: Users },
    { id: 'locks', label: 'Locks', icon: Lock },
    { id: 'approvals', label: 'Approvals', icon: CheckCircle },
    { id: 'conflicts', label: 'Conflicts', icon: AlertTriangle },
  ];

  const renderOverview = () => (
    <div className="space-y-6">
      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600 dark:text-gray-400">Active Agents</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {agents.filter(a => a.status === 'active').length}
              </p>
            </div>
            <div className="w-12 h-12 bg-blue-100 dark:bg-blue-900 rounded-lg flex items-center justify-center">
              <Users className="w-6 h-6 text-blue-600 dark:text-blue-400" />
            </div>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600 dark:text-gray-400">Crate Locks</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">{mockCrateLocks.length}</p>
            </div>
            <div className="w-12 h-12 bg-red-100 dark:bg-red-900 rounded-lg flex items-center justify-center">
              <Lock className="w-6 h-6 text-red-600 dark:text-red-400" />
            </div>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600 dark:text-gray-400">Approvals Today</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {mockApprovals.filter(a => a.status === 'approved').length}
              </p>
            </div>
            <div className="w-12 h-12 bg-green-100 dark:bg-green-900 rounded-lg flex items-center justify-center">
              <CheckCircle className="w-6 h-6 text-green-600 dark:text-green-400" />
            </div>
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600 dark:text-gray-400">Active Conflicts</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {mockConflicts.filter(c => c.status === 'pending').length}
              </p>
            </div>
            <div className="w-12 h-12 bg-yellow-100 dark:bg-yellow-900 rounded-lg flex items-center justify-center">
              <AlertTriangle className="w-6 h-6 text-yellow-600 dark:text-yellow-400" />
            </div>
          </div>
        </motion.div>
      </div>

      {/* Recent Activity */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.4 }}
        className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
      >
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Recent Activity</h3>
        <div className="space-y-3">
          <div className="flex items-center space-x-3 p-3 bg-gray-50 dark:bg-slate-700 rounded-lg">
            <div className="w-2 h-2 bg-green-500 rounded-full"></div>
            <span className="text-sm text-gray-600 dark:text-gray-400">
              Agent Alpha approved PR #120
            </span>
            <span className="text-xs text-gray-500 dark:text-gray-500 ml-auto">2m ago</span>
          </div>
          <div className="flex items-center space-x-3 p-3 bg-gray-50 dark:bg-slate-700 rounded-lg">
            <div className="w-2 h-2 bg-red-500 rounded-full"></div>
            <span className="text-sm text-gray-600 dark:text-gray-400">
              Conflict detected in crates/consensus (PR #124)
            </span>
            <span className="text-xs text-gray-500 dark:text-gray-500 ml-auto">5m ago</span>
          </div>
          <div className="flex items-center space-x-3 p-3 bg-gray-50 dark:bg-slate-700 rounded-lg">
            <div className="w-2 h-2 bg-blue-500 rounded-full"></div>
            <span className="text-sm text-gray-600 dark:text-gray-400">
              Agent Delta locked crates/network
            </span>
            <span className="text-xs text-gray-500 dark:text-gray-500 ml-auto">10m ago</span>
          </div>
        </div>
      </motion.div>
    </div>
  );

  const renderAgents = () => (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {agents.map((agent, index) => (
          <motion.div
            key={agent.id}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.1 }}
            className="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 border border-gray-200 dark:border-gray-700"
          >
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center space-x-3">
                <div 
                  className="w-4 h-4 rounded-full"
                  style={{ backgroundColor: agent.color }}
                ></div>
                <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                  {agent.name}
                </h3>
              </div>
              <div className={`w-3 h-3 rounded-full ${getStatusColor(agent.status)}`}></div>
            </div>
            
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-gray-600 dark:text-gray-400">Status:</span>
                <span className="font-medium text-gray-900 dark:text-white">
                  {getStatusText(agent.status)}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-600 dark:text-gray-400">Active Issues:</span>
                <span className="font-medium text-gray-900 dark:text-white">
                  {agent.activeIssues}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-600 dark:text-gray-400">Last Activity:</span>
                <span className="font-medium text-gray-900 dark:text-white">
                  {agent.lastActivity}
                </span>
              </div>
            </div>
          </motion.div>
        ))}
      </div>
    </div>
  );

  const renderLocks = () => (
    <div className="space-y-6">
      <div className="bg-white dark:bg-slate-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Active Crate Locks</h3>
        </div>
        <div className="divide-y divide-gray-200 dark:divide-gray-700">
          {mockCrateLocks.map((lock, index) => (
            <motion.div
              key={lock.crate}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: index * 0.1 }}
              className="px-6 py-4 flex items-center justify-between"
            >
              <div className="flex items-center space-x-3">
                <Lock className="w-5 h-5 text-red-500" />
                <div>
                  <p className="font-medium text-gray-900 dark:text-white">{lock.crate}</p>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Locked by {lock.lockedBy}
                  </p>
                </div>
              </div>
              <div className="text-right">
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {new Date(lock.lockedAt).toLocaleString()}
                </p>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </div>
  );

  const renderApprovals = () => (
    <div className="space-y-6">
      <div className="bg-white dark:bg-slate-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Recent Approvals</h3>
        </div>
        <div className="divide-y divide-gray-200 dark:divide-gray-700">
          {mockApprovals.map((approval, index) => (
            <motion.div
              key={approval.prNumber}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: index * 0.1 }}
              className="px-6 py-4 flex items-center justify-between"
            >
              <div className="flex items-center space-x-3">
                <CheckCircle className={`w-5 h-5 ${
                  approval.status === 'approved' ? 'text-green-500' : 
                  approval.status === 'pending' ? 'text-yellow-500' : 'text-red-500'
                }`} />
                <div>
                  <p className="font-medium text-gray-900 dark:text-white">
                    PR #{approval.prNumber}
                  </p>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Status: {approval.status}
                  </p>
                </div>
              </div>
              <div className="text-right">
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {new Date(approval.approvedAt).toLocaleString()}
                </p>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </div>
  );

  const renderConflicts = () => (
    <div className="space-y-6">
      <div className="bg-white dark:bg-slate-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Conflict History</h3>
        </div>
        <div className="divide-y divide-gray-200 dark:divide-gray-700">
          {mockConflicts.map((conflict, index) => (
            <motion.div
              key={`${conflict.prNumber}-${conflict.crate}`}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: index * 0.1 }}
              className="px-6 py-4 flex items-center justify-between"
            >
              <div className="flex items-center space-x-3">
                <AlertTriangle className={`w-5 h-5 ${
                  conflict.status === 'resolved' ? 'text-green-500' : 'text-yellow-500'
                }`} />
                <div>
                  <p className="font-medium text-gray-900 dark:text-white">
                    PR #{conflict.prNumber} - {conflict.crate}
                  </p>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Status: {conflict.status}
                  </p>
                </div>
              </div>
              <div className="text-right">
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {new Date(conflict.detectedAt).toLocaleString()}
                </p>
              </div>
            </motion.div>
          ))}
        </div>
      </div>
    </div>
  );

  const renderContent = () => {
    switch (activeTab) {
      case 'overview': return renderOverview();
      case 'agents': return renderAgents();
      case 'locks': return renderLocks();
      case 'approvals': return renderApprovals();
      case 'conflicts': return renderConflicts();
      default: return renderOverview();
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center space-x-2">
          <RefreshCw className="w-5 h-5 animate-spin text-blue-500" />
          <span className="text-gray-600 dark:text-gray-400">Loading MetaAgent data...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center">
            <Brain className="w-8 h-8 mr-3 text-blue-500" />
            MetaAgent Dashboard
          </h1>
          <p className="text-gray-600 dark:text-gray-400 mt-1">
            Self-governing multi-agent workspace management
          </p>
        </div>
        
        <div className="flex items-center space-x-3">
          <div className="text-sm text-gray-500 dark:text-gray-400">
            Last updated: {lastRefresh.toLocaleTimeString()}
          </div>
          <button
            onClick={handleRefresh}
            className="p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 transition-colors"
          >
            <RefreshCw className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200 dark:border-gray-700">
        <nav className="-mb-px flex space-x-8">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`py-2 px-1 border-b-2 font-medium text-sm flex items-center space-x-2 ${
                activeTab === tab.id
                  ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300'
              }`}
            >
              <tab.icon className="w-4 h-4" />
              <span>{tab.label}</span>
            </button>
          ))}
        </nav>
      </div>

      {/* Content */}
      {renderContent()}
    </div>
  );
}