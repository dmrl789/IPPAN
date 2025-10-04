import React, { useState, useEffect } from 'react';
import { Card, Button, Badge, LoadingSpinner, Modal, Field, Input } from '../components/UI';
import { useToast } from '../components/Toast';
import { buildApiUrl } from '../lib/api';

interface L2Network {
  id: string;
  proofType: string;
  daMode: string;
  lastEpoch: number;
  totalCommits: number;
  totalExits: number;
  status: 'active' | 'inactive' | 'challenged';
  challengeWindow?: number;
  lastCommitTime?: string;
  totalValueLocked?: number;
  exitQueueLength?: number;
}

interface L2ExitRequest {
  l2Id: string;
  epoch: number;
  proofOfInclusion: string;
  account: string;
  amount: string;
  nonce: number;
}

interface L2ExitRecord {
  id: string;
  l2Id: string;
  account: string;
  amount: string;
  status: 'pending' | 'finalized' | 'rejected' | 'challenge_window';
  submittedAt: string;
  finalizedAt?: string;
  rejectionReason?: string;
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
  const [exitData, setExitData] = useState<L2ExitRequest>({
    l2Id: '',
    epoch: 0,
    proofOfInclusion: '',
    account: '',
    amount: '',
    nonce: 0
  });
  const [exitRecords, setExitRecords] = useState<L2ExitRecord[]>([]);
  const [isExitModalOpen, setIsExitModalOpen] = useState(false);
  const [isSubmittingExit, setIsSubmittingExit] = useState(false);
  const { toasts, removeToast, success, error } = useToast();

  // Mock data for demonstration
  const l2Networks: L2Network[] = [
    {
      id: 'rollup-1',
      proofType: 'zk-groth16',
      daMode: 'external',
      lastEpoch: 5,
      totalCommits: 25,
      totalExits: 3,
      status: 'active',
      lastCommitTime: new Date(Date.now() - 300000).toISOString(),
      totalValueLocked: 1250000,
      exitQueueLength: 2
    },
    {
      id: 'optimistic-rollup',
      proofType: 'optimistic',
      daMode: 'inline',
      lastEpoch: 12,
      totalCommits: 48,
      totalExits: 7,
      status: 'active',
      challengeWindow: 60000,
      lastCommitTime: new Date(Date.now() - 180000).toISOString(),
      totalValueLocked: 890000,
      exitQueueLength: 5
    },
    {
      id: 'app-chain-xyz',
      proofType: 'external',
      daMode: 'external',
      lastEpoch: 3,
      totalCommits: 8,
      totalExits: 1,
      status: 'challenged',
      lastCommitTime: new Date(Date.now() - 720000).toISOString(),
      totalValueLocked: 45000,
      exitQueueLength: 0
    }
  ];

  // Mock exit records
  useEffect(() => {
    const mockExits: L2ExitRecord[] = [
      {
        id: 'exit-1',
        l2Id: 'rollup-1',
        account: '0x1234...5678',
        amount: '100.5 IPPAN',
        status: 'finalized',
        submittedAt: new Date(Date.now() - 3600000).toISOString(),
        finalizedAt: new Date(Date.now() - 3000000).toISOString()
      },
      {
        id: 'exit-2',
        l2Id: 'optimistic-rollup',
        account: '0xabcd...efgh',
        amount: '50.0 IPPAN',
        status: 'challenge_window',
        submittedAt: new Date(Date.now() - 300000).toISOString()
      },
      {
        id: 'exit-3',
        l2Id: 'rollup-1',
        account: '0x9876...5432',
        amount: '25.75 IPPAN',
        status: 'pending',
        submittedAt: new Date(Date.now() - 60000).toISOString()
      }
    ];
    setExitRecords(mockExits);
  }, []);

  const handleCommitSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      if (!commitData.l2Id || !commitData.epoch || !commitData.stateRoot || !commitData.daHash) {
        throw new Error('Please complete all required commit fields');
      }

      const epoch = Number(commitData.epoch);
      if (!Number.isFinite(epoch) || epoch <= 0) {
        throw new Error('Epoch must be a positive number');
      }

      const response = await fetch(buildApiUrl('/api/v1/l2/commit'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          l2_id: commitData.l2Id,
          epoch,
          state_root: commitData.stateRoot,
          da_hash: commitData.daHash,
          proof_type: commitData.proofType,
          proof: commitData.proof || null,
          inline_data: commitData.inlineData || null,
        }),
      });

      const payload = await response.json();
      if (!response.ok || !payload.success) {
        throw new Error(payload.error || 'Failed to submit L2 commit');
      }

      const data = payload.data;
      success(
        'L2 Commit Submitted',
        data
          ? `Commit ${data.commit_id} accepted for epoch ${data.epoch}.`
          : 'Your L2 commit has been submitted successfully.'
      );

      setCommitData({
        l2Id: '',
        epoch: '',
        stateRoot: '',
        daHash: '',
        proofType: 'zk-groth16',
        proof: '',
        inlineData: ''
      });
    } catch (err) {
      error(
        'Commit Submission Failed',
        err instanceof Error ? err.message : 'An unexpected error occurred'
      );
    }
  };

  const handleExitSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmittingExit(true);

    try {
      // Validate form
      if (!exitData.l2Id || !exitData.account || !exitData.amount || !exitData.proofOfInclusion) {
        throw new Error('Please fill in all required fields');
      }

      // Convert amount to number for validation
      const amount = parseFloat(exitData.amount);
      if (isNaN(amount) || amount <= 0) {
        throw new Error('Please enter a valid amount');
      }

      // Submit to API
      const response = await fetch(buildApiUrl('/api/v1/l2/verify_exit'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          l2_id: exitData.l2Id,
          epoch: exitData.epoch,
          proof_of_inclusion: exitData.proofOfInclusion,
          account: exitData.account,
          amount: amount,
          nonce: exitData.nonce,
        }),
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to submit L2 exit');
      }

      const result = await response.json();
      console.log('L2 exit submitted:', result);

      // Add to local records
      const newExit: L2ExitRecord = {
        id: `exit-${Date.now()}`,
        l2Id: exitData.l2Id,
        account: exitData.account,
        amount: `${exitData.amount} IPPAN`,
        status: 'pending',
        submittedAt: new Date().toISOString(),
      };

      setExitRecords(prev => [newExit, ...prev]);
      setIsExitModalOpen(false);
      setExitData({
        l2Id: '',
        epoch: 0,
        proofOfInclusion: '',
        account: '',
        amount: '',
        nonce: 0
      });

      success(
        'L2 Exit Submitted',
        `Your exit request for ${exitData.amount} IPPAN from ${exitData.l2Id} has been submitted and is pending verification.`
      );

    } catch (err) {
      error(
        'Exit Submission Failed',
        err instanceof Error ? err.message : 'An unexpected error occurred'
      );
    } finally {
      setIsSubmittingExit(false);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active': return 'text-green-600 bg-green-100 dark:bg-green-900/20';
      case 'inactive': return 'text-red-600 bg-red-100 dark:bg-red-900/20';
      case 'challenged': return 'text-yellow-600 bg-yellow-100 dark:bg-yellow-900/20';
      default: return 'text-gray-600 bg-gray-100 dark:bg-gray-900/20';
    }
  };

  const getProofTypeIcon = (proofType: string) => {
    switch (proofType) {
      case 'zk-groth16': return '🔐';
      case 'optimistic': return '⚡';
      case 'external': return '🔗';
      default: return '❓';
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
        <h2 className="text-xl font-semibold mb-4 text-gray-900 dark:text-white">L2 Networks</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {l2Networks.map((network) => (
            <div
              key={network.id}
              className="border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:shadow-md transition-shadow cursor-pointer bg-gray-50 dark:bg-gray-700/50"
              onClick={() => setSelectedL2(network.id)}
            >
              <div className="flex justify-between items-start mb-3">
                <div className="flex items-center space-x-2">
                  <span className="text-xl">{getProofTypeIcon(network.proofType)}</span>
                  <h3 className="font-medium text-gray-900 dark:text-white">{network.id}</h3>
                </div>
                <span className={`px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(network.status)}`}>
                  {network.status}
                </span>
              </div>
              <div className="space-y-2 text-sm text-gray-600 dark:text-gray-400">
                <div className="flex justify-between">
                  <span>Proof:</span>
                  <span className="font-medium">{network.proofType}</span>
                </div>
                <div className="flex justify-between">
                  <span>DA:</span>
                  <span className="font-medium">{network.daMode}</span>
                </div>
                <div className="flex justify-between">
                  <span>Epoch:</span>
                  <span className="font-medium">{network.lastEpoch}</span>
                </div>
                <div className="flex justify-between">
                  <span>TVL:</span>
                  <span className="font-medium">${network.totalValueLocked?.toLocaleString()}</span>
                </div>
                <div className="flex justify-between">
                  <span>Commits:</span>
                  <span className="font-medium">{network.totalCommits}</span>
                </div>
                <div className="flex justify-between">
                  <span>Exits:</span>
                  <span className="font-medium">{network.totalExits}</span>
                </div>
                <div className="flex justify-between">
                  <span>Exit Queue:</span>
                  <span className="font-medium">{network.exitQueueLength || 0}</span>
                </div>
                {network.challengeWindow && (
                  <div className="flex justify-between">
                    <span>Challenge:</span>
                    <span className="font-medium">{network.challengeWindow / 1000}s</span>
                  </div>
                )}
                {network.lastCommitTime && (
                  <div className="flex justify-between">
                    <span>Last Commit:</span>
                    <span className="font-medium text-xs">
                      {new Date(network.lastCommitTime).toLocaleTimeString()}
                    </span>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* L2 Commit Form */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h2 className="text-xl font-semibold mb-4 text-gray-900 dark:text-white">Submit L2 Commit</h2>
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
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white"
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
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white"
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
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white"
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
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white"
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
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white"
              >
                <option value="zk-groth16">🔐 ZK-Groth16</option>
                <option value="optimistic">⚡ Optimistic</option>
                <option value="external">🔗 External</option>
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
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white"
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
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white"
              rows={3}
              placeholder="Enter inline data for inline DA mode..."
            />
          </div>

          <button
            type="submit"
            className="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors"
          >
            Submit L2 Commit
          </button>
        </form>
      </div>

      {/* L2 Statistics */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h2 className="text-xl font-semibold mb-4 text-gray-900 dark:text-white">L2 Statistics</h2>
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

      {/* L2 Exit Management */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Exit History */}
        <Card title="Exit History">
          <div className="space-y-3">
            {exitRecords.length === 0 ? (
              <div className="text-center py-8 text-gray-500">
                <div className="text-4xl mb-2">📤</div>
                <p>No exit requests yet</p>
                <p className="text-sm">Submit your first L2 exit to see it here</p>
              </div>
            ) : (
              exitRecords.map((exit) => (
                <div key={exit.id} className="p-3 border rounded-lg bg-gray-50 dark:bg-gray-700/50">
                  <div className="flex justify-between items-start mb-2">
                    <div>
                      <div className="font-medium text-sm">{exit.l2Id}</div>
                      <div className="text-xs text-gray-500">{exit.account}</div>
                    </div>
                    <Badge variant={
                      exit.status === 'finalized' ? 'success' :
                      exit.status === 'pending' ? 'warning' :
                      exit.status === 'challenge_window' ? 'blue' : 'error'
                    }>
                      {exit.status.replace('_', ' ')}
                    </Badge>
                  </div>
                  <div className="flex justify-between items-center text-sm">
                    <span className="font-medium">{exit.amount}</span>
                    <span className="text-gray-500">
                      {new Date(exit.submittedAt).toLocaleDateString()}
                    </span>
                  </div>
                  {exit.finalizedAt && (
                    <div className="text-xs text-green-600 mt-1">
                      Finalized: {new Date(exit.finalizedAt).toLocaleString()}
                    </div>
                  )}
                  {exit.rejectionReason && (
                    <div className="text-xs text-red-600 mt-1">
                      Rejected: {exit.rejectionReason}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        </Card>

        {/* Exit Actions */}
        <Card title="L2 Exit Actions">
          <div className="space-y-4">
            <div className="text-center py-4">
              <div className="text-4xl mb-2">🚪</div>
              <h3 className="font-medium mb-2">Withdraw from L2</h3>
              <p className="text-sm text-gray-600 mb-4">
                Submit a proof to withdraw your assets from an L2 network
              </p>
              <Button 
                onClick={() => setIsExitModalOpen(true)}
                className="w-full"
              >
                Submit L2 Exit
              </Button>
            </div>
            
            <div className="border-t pt-4">
              <h4 className="font-medium mb-2">Quick Stats</h4>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <div className="text-gray-600">Pending Exits</div>
                  <div className="font-medium">
                    {exitRecords.filter(e => e.status === 'pending').length}
                  </div>
                </div>
                <div>
                  <div className="text-gray-600">Total Exits</div>
                  <div className="font-medium">{exitRecords.length}</div>
                </div>
                <div>
                  <div className="text-gray-600">Finalized</div>
                  <div className="font-medium text-green-600">
                    {exitRecords.filter(e => e.status === 'finalized').length}
                  </div>
                </div>
                <div>
                  <div className="text-gray-600">In Challenge</div>
                  <div className="font-medium text-blue-600">
                    {exitRecords.filter(e => e.status === 'challenge_window').length}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </Card>
      </div>

      {/* L2 Exit Modal */}
      <Modal 
        isOpen={isExitModalOpen} 
        onClose={() => setIsExitModalOpen(false)}
        title="Submit L2 Exit"
      >
        <form onSubmit={handleExitSubmit} className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Field label="L2 Network" required>
              <select
                value={exitData.l2Id}
                onChange={(e) => setExitData({...exitData, l2Id: e.target.value})}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                required
              >
                <option value="">Select L2 Network</option>
                {l2Networks.filter(n => n.status === 'active').map(network => (
                  <option key={network.id} value={network.id}>
                    {network.id} ({network.proofType})
                  </option>
                ))}
              </select>
            </Field>

            <Field label="Epoch" required>
              <Input
                type="number"
                value={exitData.epoch}
                onChange={(e) => setExitData({...exitData, epoch: parseInt(e.target.value) || 0})}
                placeholder="Latest epoch"
                required
              />
            </Field>
          </div>

          <Field label="Account Address" required>
            <Input
              type="text"
              value={exitData.account}
              onChange={(e) => setExitData({...exitData, account: e.target.value})}
              placeholder="0x1234...5678"
              required
            />
          </Field>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Field label="Amount" required>
              <Input
                type="number"
                step="0.000001"
                value={exitData.amount}
                onChange={(e) => setExitData({...exitData, amount: e.target.value})}
                placeholder="0.0"
                required
              />
            </Field>

            <Field label="Nonce" required>
              <Input
                type="number"
                value={exitData.nonce}
                onChange={(e) => setExitData({...exitData, nonce: parseInt(e.target.value) || 0})}
                placeholder="0"
                required
              />
            </Field>
          </div>

          <Field label="Proof of Inclusion" required>
            <textarea
              value={exitData.proofOfInclusion}
              onChange={(e) => setExitData({...exitData, proofOfInclusion: e.target.value})}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              rows={4}
              placeholder="Enter the merkle proof of inclusion in the L2 state..."
              required
            />
          </Field>

          <div className="bg-blue-50 border border-blue-200 rounded-md p-4">
            <h4 className="font-medium text-blue-900 mb-2">Exit Process</h4>
            <ul className="text-sm text-blue-800 space-y-1">
              <li>• Submit proof of inclusion in L2 state</li>
              <li>• Wait for verification (1-7 days depending on L2)</li>
              <li>• Assets will be released to your L1 account</li>
              <li>• Challenge window applies for optimistic rollups</li>
            </ul>
          </div>

          <div className="flex justify-end space-x-3">
            <Button
              type="button"
              variant="secondary"
              onClick={() => setIsExitModalOpen(false)}
              disabled={isSubmittingExit}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={isSubmittingExit}
              className="min-w-[120px]"
            >
              {isSubmittingExit ? (
                <>
                  <LoadingSpinner />
                  <span className="ml-2">Submitting...</span>
                </>
              ) : (
                'Submit Exit'
              )}
            </Button>
          </div>
        </form>
      </Modal>

      {/* Toast Notifications */}
      <div className="fixed top-4 right-4 z-50 space-y-2">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            className={`
              ${toast.type === 'success' ? 'bg-green-50 border-green-200 text-green-800' :
                toast.type === 'error' ? 'bg-red-50 border-red-200 text-red-800' :
                'bg-blue-50 border-blue-200 text-blue-800'}
              border rounded-lg p-4 shadow-lg max-w-sm w-full
              transform transition-all duration-300 ease-in-out
            `}
          >
            <div className="flex items-start">
              <div className="flex-shrink-0 text-lg mr-3">
                {toast.type === 'success' ? '✅' : toast.type === 'error' ? '❌' : 'ℹ️'}
              </div>
              <div className="flex-1">
                <h4 className="font-medium text-sm">
                  {toast.title}
                </h4>
                <p className="text-sm mt-1 opacity-90">
                  {toast.message}
                </p>
              </div>
              <button
                onClick={() => removeToast(toast.id)}
                className="flex-shrink-0 ml-2 text-gray-400 hover:text-gray-600"
              >
                ✕
              </button>
            </div>
        </div>
        ))}
      </div>
    </div>
  );
}
