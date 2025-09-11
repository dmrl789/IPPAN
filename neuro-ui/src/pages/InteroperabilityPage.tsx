import React, { useState } from 'react';

interface L2Network {
  id: string;
  proofType: string;
  daMode: string;
  lastEpoch: number;
  totalCommits: number;
  totalExits: number;
  status: 'active' | 'inactive' | 'challenged';
}

export default function InteroperabilityPage() {
  const [selectedL2, setSelectedL2] = useState<string>('');
  const [commitData, setCommitData] = useState({
    l2Id: '',
    epoch: '',
    stateRoot: '',
    daHash: '',
    proofType: 'zk-groth16',
    proof: '',
    inlineData: ''
  });

  // Mock data for demonstration
  const l2Networks: L2Network[] = [
    {
      id: 'rollup-1',
      proofType: 'zk-groth16',
      daMode: 'external',
      lastEpoch: 5,
      totalCommits: 25,
      totalExits: 3,
      status: 'active'
    },
    {
      id: 'optimistic-rollup',
      proofType: 'optimistic',
      daMode: 'inline',
      lastEpoch: 12,
      totalCommits: 48,
      totalExits: 7,
      status: 'active'
    },
    {
      id: 'app-chain-xyz',
      proofType: 'external',
      daMode: 'external',
      lastEpoch: 3,
      totalCommits: 8,
      totalExits: 1,
      status: 'challenged'
    }
  ];

  const handleCommitSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Here you would integrate with the actual L2 commit API
    console.log('Submitting L2 commit:', commitData);
    alert('L2 commit submitted successfully!');
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active': return 'text-green-600 bg-green-100';
      case 'inactive': return 'text-red-600 bg-red-100';
      case 'challenged': return 'text-yellow-600 bg-yellow-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Interoperability</h1>
        <p className="mt-2 text-gray-600 dark:text-gray-400">
          Manage L2 networks, submit commits, and monitor cross-chain activities
        </p>
      </div>

      {/* L2 Networks Overview */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h2 className="text-xl font-semibold mb-4">L2 Networks</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {l2Networks.map((network) => (
            <div
              key={network.id}
              className="border rounded-lg p-4 hover:shadow-md transition-shadow cursor-pointer"
              onClick={() => setSelectedL2(network.id)}
            >
              <div className="flex justify-between items-start mb-2">
                <h3 className="font-medium text-gray-900 dark:text-white">{network.id}</h3>
                <span className={`px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(network.status)}`}>
                  {network.status}
                </span>
              </div>
              <div className="space-y-1 text-sm text-gray-600 dark:text-gray-400">
                <p>Proof: {network.proofType}</p>
                <p>DA: {network.daMode}</p>
                <p>Epoch: {network.lastEpoch}</p>
                <p>Commits: {network.totalCommits}</p>
                <p>Exits: {network.totalExits}</p>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* L2 Commit Form */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h2 className="text-xl font-semibold mb-4">Submit L2 Commit</h2>
        <form onSubmit={handleCommitSubmit} className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                L2 Network ID
              </label>
              <input
                type="text"
                value={commitData.l2Id}
                onChange={(e) => setCommitData({...commitData, l2Id: e.target.value})}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="e.g., rollup-1"
                required
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Epoch
              </label>
              <input
                type="number"
                value={commitData.epoch}
                onChange={(e) => setCommitData({...commitData, epoch: e.target.value})}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="1"
                required
              />
            </div>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                State Root
              </label>
              <input
                type="text"
                value={commitData.stateRoot}
                onChange={(e) => setCommitData({...commitData, stateRoot: e.target.value})}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="0x1234..."
                required
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                DA Hash
              </label>
              <input
                type="text"
                value={commitData.daHash}
                onChange={(e) => setCommitData({...commitData, daHash: e.target.value})}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="0x5678..."
                required
              />
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Proof Type
              </label>
              <select
                value={commitData.proofType}
                onChange={(e) => setCommitData({...commitData, proofType: e.target.value})}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="zk-groth16">ZK-Groth16</option>
                <option value="optimistic">Optimistic</option>
                <option value="external">External</option>
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Proof Data
              </label>
              <input
                type="text"
                value={commitData.proof}
                onChange={(e) => setCommitData({...commitData, proof: e.target.value})}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="0xabcd..."
                required
              />
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Inline Data (Optional)
            </label>
            <textarea
              value={commitData.inlineData}
              onChange={(e) => setCommitData({...commitData, inlineData: e.target.value})}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              rows={3}
              placeholder="Enter inline data for inline DA mode..."
            />
          </div>

          <button
            type="submit"
            className="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
          >
            Submit L2 Commit
          </button>
        </form>
      </div>

      {/* L2 Statistics */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h2 className="text-xl font-semibold mb-4">L2 Statistics</h2>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div className="text-center p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
            <div className="text-2xl font-bold text-blue-600 dark:text-blue-400">
              {l2Networks.length}
            </div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Active L2s</div>
          </div>
          <div className="text-center p-4 bg-green-50 dark:bg-green-900/20 rounded-lg">
            <div className="text-2xl font-bold text-green-600 dark:text-green-400">
              {l2Networks.reduce((sum, n) => sum + n.totalCommits, 0)}
            </div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Total Commits</div>
          </div>
          <div className="text-center p-4 bg-purple-50 dark:bg-purple-900/20 rounded-lg">
            <div className="text-2xl font-bold text-purple-600 dark:text-purple-400">
              {l2Networks.reduce((sum, n) => sum + n.totalExits, 0)}
            </div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Total Exits</div>
          </div>
          <div className="text-center p-4 bg-orange-50 dark:bg-orange-900/20 rounded-lg">
            <div className="text-2xl font-bold text-orange-600 dark:text-orange-400">
              {l2Networks.filter(n => n.status === 'active').length}
            </div>
            <div className="text-sm text-gray-600 dark:text-gray-400">Healthy L2s</div>
          </div>
        </div>
      </div>
    </div>
  );
}
