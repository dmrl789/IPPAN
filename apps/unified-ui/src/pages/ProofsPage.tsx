import React, { useState, useEffect } from 'react';
import { Card, Button, Input, Badge, Select, SelectTrigger, SelectValue, SelectContent, SelectItem, Label, Textarea } from '../components/UI';
import {
  Shield, CheckCircle, AlertCircle, Clock, Eye, Download, Copy, ExternalLink,
  Plus, Filter, Grid, List, Star, Users, Activity, Zap, TrendingUp, Brain,
  Network, Settings, BarChart3, FileText, Image, Music, Video, Code, Globe,
  Upload, RefreshCw, Save, Share2, Bookmark, History, Timer, Target, Award,
  Crown, Medal, TrendingDown, ArrowUp, ArrowDown, Minus, X, Check, AlertTriangle,
  Lock, Unlock, Key, Database, Cpu, HardDrive, Link, Info, Search, Filter as FilterIcon
} from 'lucide-react';

// =================== Types ===================
type ProofType = 'zk-stark' | 'zk-snark' | 'plonk' | 'bulletproof' | 'range-proof' | 'membership-proof';
type ProofStatus = 'pending' | 'verifying' | 'verified' | 'failed' | 'expired' | 'revoked';
type ProofCategory = 'model' | 'computation' | 'data' | 'identity' | 'ownership' | 'consensus';

type Proof = {
  id: string;
  type: ProofType;
  category: ProofCategory;
  status: ProofStatus;
  title: string;
  description: string;
  modelId?: string;
  modelName?: string;
  prover: string;
  verifier?: string;
  createdAt: string;
  verifiedAt?: string;
  expiresAt?: string;
  size: number; // bytes
  verificationTime: number; // ms
  gasCost?: number;
  publicInputs: string[];
  publicOutputs: string[];
  proofData: string;
  verificationKey: string;
  metadata?: Record<string, any>;
  tags: string[];
  confidence: number; // percentage
  trustScore: number; // 0-100
};

type VerificationResult = {
  proofId: string;
  isValid: boolean;
  verifiedAt: string;
  verificationTime: number;
  gasUsed: number;
  error?: string;
  details?: Record<string, any>;
};

// =================== Mock Data ===================
const generateMockProofs = (): Proof[] => [
  {
    id: 'proof_001',
    type: 'zk-stark',
    category: 'model',
    status: 'verified',
    title: 'GPT-4o Model Integrity Proof',
    description: 'Zero-knowledge proof verifying the integrity and authenticity of GPT-4o model weights',
    modelId: 'model_001',
    modelName: 'GPT-4o',
    prover: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
    verifier: 'i1B2zP2eP6QGefi3DMPTfTL6SLmv8DivfNb',
    createdAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
    verifiedAt: new Date(Date.now() - 1 * 60 * 60 * 1000).toISOString(),
    expiresAt: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString(),
    size: 2048,
    verificationTime: 1200,
    gasCost: 150000,
    publicInputs: ['model_hash', 'commitment'],
    publicOutputs: ['integrity_check', 'authenticity_verified'],
    proofData: '0x1a2b3c4d5e6f7890abcdef1234567890...',
    verificationKey: '0x9f8e7d6c5b4a3928170605040302010f...',
    metadata: {
      modelSize: '175B parameters',
      accuracy: '95.8%',
      trainingData: 'verified_dataset_001'
    },
    tags: ['ai', 'model-integrity', 'zk-stark', 'gpt'],
    confidence: 99.8,
    trustScore: 95
  },
  {
    id: 'proof_002',
    type: 'zk-snark',
    category: 'computation',
    status: 'verified',
    title: 'ImageNet Training Computation Proof',
    description: 'SNARK proof verifying the correctness of ImageNet model training computation',
    modelId: 'model_002',
    modelName: 'ResNet-50',
    prover: 'i1C3zP3eP7QGefi4DMPTfTL7SLmv9DivfNc',
    verifier: 'i1D4zP4eP8QGefi5DMPTfTL8SLmv0DivfNd',
    createdAt: new Date(Date.now() - 4 * 60 * 60 * 1000).toISOString(),
    verifiedAt: new Date(Date.now() - 3 * 60 * 60 * 1000).toISOString(),
    expiresAt: new Date(Date.now() + 15 * 24 * 60 * 60 * 1000).toISOString(),
    size: 1024,
    verificationTime: 800,
    gasCost: 120000,
    publicInputs: ['training_data_hash', 'hyperparameters'],
    publicOutputs: ['model_weights_hash', 'accuracy_claim'],
    proofData: '0x2b3c4d5e6f7890abcdef1234567890ab...',
    verificationKey: '0x8e7d6c5b4a3928170605040302010f9e...',
    metadata: {
      epochs: 100,
      batchSize: 256,
      learningRate: 0.001,
      datasetSize: '1.2M images'
    },
    tags: ['training', 'computation', 'zk-snark', 'imagenet'],
    confidence: 98.5,
    trustScore: 92
  },
  {
    id: 'proof_003',
    type: 'plonk',
    category: 'data',
    status: 'verifying',
    title: 'Dataset Privacy Preservation Proof',
    description: 'PLONK proof ensuring dataset privacy while maintaining utility for training',
    modelId: 'model_003',
    modelName: 'BERT-Base',
    prover: 'i1E5zP5eP9QGefi6DMPTfTL9SLmv1DivfNe',
    createdAt: new Date(Date.now() - 30 * 60 * 1000).toISOString(),
    expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(),
    size: 1536,
    verificationTime: 0,
    gasCost: 180000,
    publicInputs: ['privacy_budget', 'utility_threshold'],
    publicOutputs: ['privacy_preserved', 'utility_maintained'],
    proofData: '0x3c4d5e6f7890abcdef1234567890abcd...',
    verificationKey: '0x7d6c5b4a3928170605040302010f9e8d...',
    metadata: {
      privacyLevel: 'differential_privacy',
      epsilon: 1.0,
      delta: 1e-5,
      utilityMetric: 'f1_score'
    },
    tags: ['privacy', 'differential-privacy', 'plonk', 'data'],
    confidence: 0,
    trustScore: 0
  },
  {
    id: 'proof_004',
    type: 'bulletproof',
    category: 'ownership',
    status: 'verified',
    title: 'Model Ownership Proof',
    description: 'Bulletproof demonstrating ownership of a proprietary AI model',
    modelId: 'model_004',
    modelName: 'Custom-Transformer',
    prover: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
    verifier: 'i1F6zP6eP0QGefi7DMPTfTL0SLmv2DivfNf',
    createdAt: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000).toISOString(),
    verifiedAt: new Date(Date.now() - 20 * 60 * 60 * 1000).toISOString(),
    expiresAt: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString(),
    size: 512,
    verificationTime: 400,
    gasCost: 80000,
    publicInputs: ['ownership_claim', 'model_fingerprint'],
    publicOutputs: ['ownership_verified', 'exclusivity_proven'],
    proofData: '0x4d5e6f7890abcdef1234567890abcdef...',
    verificationKey: '0x6c5b4a3928170605040302010f9e8d7c...',
    metadata: {
      ownershipType: 'exclusive',
      licenseType: 'proprietary',
      creationDate: '2024-01-15',
      originalAuthor: 'verified_author_001'
    },
    tags: ['ownership', 'proprietary', 'bulletproof', 'exclusive'],
    confidence: 99.9,
    trustScore: 98
  },
  {
    id: 'proof_005',
    type: 'range-proof',
    category: 'consensus',
    status: 'failed',
    title: 'Consensus Participation Proof',
    description: 'Range proof demonstrating valid stake participation in network consensus',
    prover: 'i1G7zP7eP1QGefi8DMPTfTL1SLmv3DivfNg',
    createdAt: new Date(Date.now() - 6 * 60 * 60 * 1000).toISOString(),
    expiresAt: new Date(Date.now() + 1 * 24 * 60 * 60 * 1000).toISOString(),
    size: 256,
    verificationTime: 0,
    gasCost: 50000,
    publicInputs: ['stake_amount', 'consensus_round'],
    publicOutputs: ['valid_participation', 'stake_verified'],
    proofData: '0x5e6f7890abcdef1234567890abcdef12...',
    verificationKey: '0x5b4a3928170605040302010f9e8d7c6b...',
    metadata: {
      stakeAmount: '10000 IPPAN',
      consensusRound: 12345,
      participationType: 'validator',
      failureReason: 'insufficient_stake'
    },
    tags: ['consensus', 'staking', 'range-proof', 'validator'],
    confidence: 0,
    trustScore: 0
  }
];

// =================== Utils ===================
function formatTimeAgo(timestamp: string): string {
  const now = new Date();
  const time = new Date(timestamp);
  const diffMs = now.getTime() - time.getTime();
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffDays = Math.floor(diffHours / 24);

  if (diffDays > 0) return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`;
  if (diffHours > 0) return `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`;
  const diffMinutes = Math.floor(diffMs / (1000 * 60));
  if (diffMinutes > 0) return `${diffMinutes} min ago`;
  return 'Just now';
}

function formatTimeRemaining(timestamp: string): string {
  const now = new Date();
  const endTime = new Date(timestamp);
  const diffMs = endTime.getTime() - now.getTime();
  
  if (diffMs <= 0) return 'Expired';
  
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffDays = Math.floor(diffHours / 24);
  const remainingHours = diffHours % 24;

  if (diffDays > 0) return `${diffDays}d ${remainingHours}h`;
  if (diffHours > 0) return `${diffHours}h`;
  const remainingMinutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));
  return `${remainingMinutes}m`;
}

function getStatusColor(status: ProofStatus): string {
  switch (status) {
    case 'verified': return 'bg-green-100 text-green-800';
    case 'verifying': return 'bg-blue-100 text-blue-800';
    case 'pending': return 'bg-yellow-100 text-yellow-800';
    case 'failed': return 'bg-red-100 text-red-800';
    case 'expired': return 'bg-gray-100 text-gray-800';
    case 'revoked': return 'bg-orange-100 text-orange-800';
    default: return 'bg-gray-100 text-gray-800';
  }
}

function getTypeIcon(type: ProofType): string {
  switch (type) {
    case 'zk-stark': return '⚡';
    case 'zk-snark': return '🔒';
    case 'plonk': return '🔑';
    case 'bulletproof': return '🛡️';
    case 'range-proof': return '📊';
    case 'membership-proof': return '👥';
    default: return '🔐';
  }
}

function getCategoryIcon(category: ProofCategory): string {
  switch (category) {
    case 'model': return '🤖';
    case 'computation': return '⚙️';
    case 'data': return '📊';
    case 'identity': return '🆔';
    case 'ownership': return '👑';
    case 'consensus': return '🌐';
    default: return '📋';
  }
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatDuration(ms: number): string {
  if (ms === 0) return '0ms';
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

// =================== Component ===================
export default function ProofsPage() {
  // Data
  const [proofs, setProofs] = useState<Proof[]>([]);
  const [selectedProof, setSelectedProof] = useState<Proof | null>(null);
  const [verificationResults, setVerificationResults] = useState<VerificationResult[]>([]);

  // Verification form
  const [proofId, setProofId] = useState('');
  const [isVerifying, setIsVerifying] = useState(false);
  const [verificationResult, setVerificationResult] = useState<VerificationResult | null>(null);

  // UI state
  const [activeTab, setActiveTab] = useState<'proofs' | 'verify' | 'results'>('proofs');
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const [showProofDetails, setShowProofDetails] = useState(false);
  const [showVerificationModal, setShowVerificationModal] = useState(false);

  // Filters
  const [statusFilter, setStatusFilter] = useState<'all' | ProofStatus>('all');
  const [typeFilter, setTypeFilter] = useState<'all' | ProofType>('all');
  const [categoryFilter, setCategoryFilter] = useState<'all' | ProofCategory>('all');
  const [searchQuery, setSearchQuery] = useState('');

  // Initialize data
  useEffect(() => {
    setProofs(generateMockProofs());
  }, []);

  // Filter proofs
  const filteredProofs = proofs.filter(proof => {
    const matchesSearch = proof.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         proof.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         proof.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesStatus = statusFilter === 'all' || proof.status === statusFilter;
    const matchesType = typeFilter === 'all' || proof.type === typeFilter;
    const matchesCategory = categoryFilter === 'all' || proof.category === categoryFilter;
    
    return matchesSearch && matchesStatus && matchesType && matchesCategory;
  });

  // Verify proof
  const handleVerify = async () => {
    if (!proofId) return;
    
    setIsVerifying(true);
    
    // Simulate verification
    await new Promise(resolve => setTimeout(resolve, 3000));
    
    const result: VerificationResult = {
      proofId,
      isValid: Math.random() > 0.2, // 80% success rate
      verifiedAt: new Date().toISOString(),
      verificationTime: Math.floor(Math.random() * 2000) + 500,
      gasUsed: Math.floor(Math.random() * 200000) + 50000,
      details: {
        proofType: 'zk-stark',
        verificationMethod: 'on-chain',
        trustScore: Math.floor(Math.random() * 40) + 60
      }
    };
    
    setVerificationResult(result);
    setVerificationResults(prev => [result, ...prev]);
    setIsVerifying(false);
    setShowVerificationModal(true);
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <header className="flex justify-between items-start">
        <div className="space-y-1">
          <h1 className="text-3xl font-bold text-gray-900">Zero-Knowledge Proofs</h1>
          <p className="text-gray-600">Verify, manage, and track cryptographic proofs for AI models and computations</p>
        </div>
        <div className="flex items-center gap-3">
          <Badge variant="success" className="flex items-center gap-1">
            <Shield className="h-3 w-3" />
            ZK System Active
          </Badge>
        </div>
      </header>

      {/* Statistics Dashboard */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Proofs</p>
              <p className="text-2xl font-bold text-gray-900">{proofs.length}</p>
            </div>
            <Shield className="h-8 w-8 text-blue-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Verified</p>
              <p className="text-2xl font-bold text-green-600">
                {proofs.filter(p => p.status === 'verified').length}
              </p>
            </div>
            <CheckCircle className="h-8 w-8 text-green-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Pending</p>
              <p className="text-2xl font-bold text-yellow-600">
                {proofs.filter(p => p.status === 'pending' || p.status === 'verifying').length}
              </p>
            </div>
            <Clock className="h-8 w-8 text-yellow-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Avg Trust Score</p>
              <p className="text-2xl font-bold text-purple-600">
                {Math.round(proofs.reduce((sum, p) => sum + p.trustScore, 0) / proofs.length)}
              </p>
            </div>
            <Star className="h-8 w-8 text-purple-600" />
          </div>
        </Card>
      </div>

      {/* Tabs */}
      <div className="flex border-b">
        <Button 
          variant={activeTab === 'proofs' ? 'secondary' : 'ghost'} 
          onClick={() => setActiveTab('proofs')}
        >
          <Shield className="h-4 w-4 mr-2" />
          Proofs
        </Button>
        <Button 
          variant={activeTab === 'verify' ? 'secondary' : 'ghost'} 
          onClick={() => setActiveTab('verify')}
        >
          <CheckCircle className="h-4 w-4 mr-2" />
          Verify
        </Button>
        <Button 
          variant={activeTab === 'results' ? 'secondary' : 'ghost'} 
          onClick={() => setActiveTab('results')}
        >
          <BarChart3 className="h-4 w-4 mr-2" />
          Results
        </Button>
      </div>

      {/* Content */}
      {activeTab === 'proofs' && (
        <>
          {/* Filters */}
          <Card>
            <div className="flex flex-wrap items-center gap-4">
              <div className="flex items-center gap-2">
                <Search className="h-4 w-4 text-gray-400" />
                <Input 
                  placeholder="Search proofs..." 
                  value={searchQuery} 
                  onChange={e => setSearchQuery(e.target.value)}
                  className="w-64"
                />
              </div>
              <Select value={statusFilter} onValueChange={(v) => setStatusFilter(v as any)}>
                <SelectTrigger className="w-40">
                  <SelectValue placeholder="Status" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Status</SelectItem>
                  <SelectItem value="verified">Verified</SelectItem>
                  <SelectItem value="verifying">Verifying</SelectItem>
                  <SelectItem value="pending">Pending</SelectItem>
                  <SelectItem value="failed">Failed</SelectItem>
                  <SelectItem value="expired">Expired</SelectItem>
                </SelectContent>
              </Select>
              <Select value={typeFilter} onValueChange={(v) => setTypeFilter(v as any)}>
                <SelectTrigger className="w-40">
                  <SelectValue placeholder="Type" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Types</SelectItem>
                  <SelectItem value="zk-stark">ZK-STARK</SelectItem>
                  <SelectItem value="zk-snark">ZK-SNARK</SelectItem>
                  <SelectItem value="plonk">PLONK</SelectItem>
                  <SelectItem value="bulletproof">Bulletproof</SelectItem>
                  <SelectItem value="range-proof">Range Proof</SelectItem>
                </SelectContent>
              </Select>
              <Select value={categoryFilter} onValueChange={(v) => setCategoryFilter(v as any)}>
                <SelectTrigger className="w-40">
                  <SelectValue placeholder="Category" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All Categories</SelectItem>
                  <SelectItem value="model">Models</SelectItem>
                  <SelectItem value="computation">Computation</SelectItem>
                  <SelectItem value="data">Data</SelectItem>
                  <SelectItem value="identity">Identity</SelectItem>
                  <SelectItem value="ownership">Ownership</SelectItem>
                  <SelectItem value="consensus">Consensus</SelectItem>
                </SelectContent>
              </Select>
              <Button 
                variant="outline" 
                size="sm"
                onClick={() => setViewMode(viewMode === 'grid' ? 'list' : 'grid')}
              >
                {viewMode === 'grid' ? 'List View' : 'Grid View'}
              </Button>
            </div>
          </Card>

          {/* Proofs Grid/List */}
          <Card>
            <div className="space-y-4">
              {viewMode === 'grid' ? (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                  {filteredProofs.map(proof => (
                    <div key={proof.id} className="border rounded-lg p-4 hover:shadow-md transition-shadow">
                      <div className="flex items-start justify-between mb-3">
                        <div className="flex items-center gap-2">
                          <span className="text-2xl">{getTypeIcon(proof.type)}</span>
                          <div>
                            <h3 className="font-semibold text-gray-900">{proof.title}</h3>
                            <p className="text-sm text-gray-600">{proof.type} • {proof.category}</p>
                          </div>
                        </div>
                        <Badge className={getStatusColor(proof.status)}>
                          {proof.status}
                        </Badge>
                      </div>
                      
                      <p className="text-sm text-gray-600 mb-3 line-clamp-2">{proof.description}</p>
                      
                      <div className="space-y-2 mb-4">
                        <div className="flex justify-between text-sm">
                          <span className="text-gray-500">Size:</span>
                          <span className="font-medium">{formatSize(proof.size)}</span>
                        </div>
                        <div className="flex justify-between text-sm">
                          <span className="text-gray-500">Verification Time:</span>
                          <span className="font-medium">{formatDuration(proof.verificationTime)}</span>
                        </div>
                        <div className="flex justify-between text-sm">
                          <span className="text-gray-500">Trust Score:</span>
                          <span className="font-medium">{proof.trustScore}/100</span>
                        </div>
                        <div className="flex justify-between text-sm">
                          <span className="text-gray-500">Confidence:</span>
                          <span className="font-medium">{proof.confidence}%</span>
                        </div>
                        {proof.expiresAt && (
                          <div className="flex justify-between text-sm">
                            <span className="text-gray-500">Expires:</span>
                            <span className="font-medium">{formatTimeRemaining(proof.expiresAt)}</span>
                          </div>
                        )}
                      </div>
                      
                      <div className="flex flex-wrap gap-1 mb-4">
                        {proof.tags.slice(0, 3).map(tag => (
                          <Badge key={tag} variant="secondary" className="text-xs">
                            {tag}
                          </Badge>
                        ))}
                        {proof.tags.length > 3 && (
                          <Badge variant="secondary" className="text-xs">
                            +{proof.tags.length - 3}
                          </Badge>
                        )}
                      </div>
                      
                      <div className="flex gap-2">
                        <Button 
                          size="sm" 
                          variant="outline"
                          onClick={() => {
                            setSelectedProof(proof);
                            setShowProofDetails(true);
                          }}
                        >
                          <Eye className="h-3 w-3 mr-1" />
                          Details
                        </Button>
                        <Button 
                          size="sm"
                          onClick={() => {
                            setProofId(proof.id);
                            setActiveTab('verify');
                          }}
                        >
                          <CheckCircle className="h-3 w-3 mr-1" />
                          Verify
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="space-y-3">
                  {filteredProofs.map(proof => (
                    <div key={proof.id} className="border rounded-lg p-4 hover:bg-gray-50">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-4">
                          <span className="text-2xl">{getTypeIcon(proof.type)}</span>
                          <div>
                            <div className="flex items-center gap-2">
                              <h3 className="font-semibold text-gray-900">{proof.title}</h3>
                              <Badge className={getStatusColor(proof.status)}>
                                {proof.status}
                              </Badge>
                            </div>
                            <p className="text-sm text-gray-600">{proof.description}</p>
                            <p className="text-xs text-gray-500">
                              {proof.type} • {proof.category} • 
                              Trust: {proof.trustScore}/100 • 
                              {formatTimeAgo(proof.createdAt)}
                            </p>
                          </div>
                        </div>
                        <div className="text-right">
                          <div className="font-semibold text-green-600">{proof.confidence}%</div>
                          <div className="text-sm text-gray-500">Confidence</div>
                          <div className="flex gap-2 mt-2">
                            <Button 
                              size="sm" 
                              variant="outline"
                              onClick={() => {
                                setSelectedProof(proof);
                                setShowProofDetails(true);
                              }}
                            >
                              <Eye className="h-3 w-3 mr-1" />
                              Details
                            </Button>
                            <Button 
                              size="sm"
                              onClick={() => {
                                setProofId(proof.id);
                                setActiveTab('verify');
                              }}
                            >
                              <CheckCircle className="h-3 w-3 mr-1" />
                              Verify
                            </Button>
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
              
              {filteredProofs.length === 0 && (
                <div className="text-center py-12">
                  <Shield className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                  <h3 className="text-lg font-medium text-gray-900 mb-2">No proofs found</h3>
                  <p className="text-gray-600">Try adjusting your search criteria.</p>
                </div>
              )}
            </div>
          </Card>
        </>
      )}

      {activeTab === 'verify' && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <Card>
            <div className="space-y-4">
              <h3 className="text-lg font-semibold">Verify Proof</h3>
              <div>
                <Label>Proof ID</Label>
                <Input
                  placeholder="Enter proof ID..."
                  value={proofId}
                  onChange={(e) => setProofId(e.target.value)}
                  className="mt-1"
                />
              </div>
              <Button 
                onClick={handleVerify}
                disabled={!proofId || isVerifying}
                className="w-full"
                size="lg"
              >
                {isVerifying ? (
                  <>
                    <Activity className="h-4 w-4 mr-2 animate-spin" />
                    Verifying...
                  </>
                ) : (
                  <>
                    <CheckCircle className="h-4 w-4 mr-2" />
                    Verify Proof
                  </>
                )}
              </Button>
            </div>
          </Card>

          <Card>
            <div className="space-y-4">
              <h3 className="text-lg font-semibold">Verification Info</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-600">Supported Types:</span>
                  <span>ZK-STARK, ZK-SNARK, PLONK, Bulletproof</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Verification Method:</span>
                  <span>On-chain verification</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Average Time:</span>
                  <span>1.2 seconds</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Gas Cost:</span>
                  <span>50,000 - 200,000</span>
                </div>
              </div>
            </div>
          </Card>
        </div>
      )}

      {activeTab === 'results' && (
        <Card>
          <div className="space-y-4">
            <h3 className="text-lg font-semibold">Verification Results</h3>
            <div className="space-y-3">
              {verificationResults.map(result => (
                <div key={result.proofId} className="border rounded-lg p-4">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-4">
                      <div className={`w-3 h-3 rounded-full ${result.isValid ? 'bg-green-500' : 'bg-red-500'}`} />
                      <div>
                        <h4 className="font-semibold text-gray-900">Proof #{result.proofId}</h4>
                        <p className="text-sm text-gray-600">
                          {result.isValid ? 'Verification successful' : 'Verification failed'}
                        </p>
                        <p className="text-xs text-gray-500">
                          {formatTimeAgo(result.verifiedAt)} • {formatDuration(result.verificationTime)} • {result.gasUsed.toLocaleString()} gas
                        </p>
                      </div>
                    </div>
                    <Badge className={result.isValid ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}>
                      {result.isValid ? 'Valid' : 'Invalid'}
                    </Badge>
                  </div>
                </div>
              ))}
              
              {verificationResults.length === 0 && (
                <div className="text-center py-8 text-gray-500">
                  No verification results yet. Verify a proof to see results here.
                </div>
              )}
            </div>
          </div>
        </Card>
      )}

      {/* Proof Details Modal */}
      {showProofDetails && selectedProof && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-4xl max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-6">
              <div className="flex items-center gap-3">
                <span className="text-3xl">{getTypeIcon(selectedProof.type)}</span>
                <div>
                  <h2 className="text-xl font-semibold">{selectedProof.title}</h2>
                  <p className="text-gray-600">{selectedProof.type} • {selectedProof.category}</p>
                </div>
              </div>
              <Button variant="outline" onClick={() => setShowProofDetails(false)}>
                ×
              </Button>
            </div>
            
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="space-y-4">
                <div>
                  <h3 className="font-medium mb-2">Proof Information</h3>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-600">Proof ID:</span>
                      <span className="font-mono text-xs">{selectedProof.id}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Prover:</span>
                      <span className="font-mono text-xs">{selectedProof.prover}</span>
                    </div>
                    {selectedProof.verifier && (
                      <div className="flex justify-between">
                        <span className="text-gray-600">Verifier:</span>
                        <span className="font-mono text-xs">{selectedProof.verifier}</span>
                      </div>
                    )}
                    <div className="flex justify-between">
                      <span className="text-gray-600">Status:</span>
                      <Badge className={getStatusColor(selectedProof.status)}>
                        {selectedProof.status}
                      </Badge>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Created:</span>
                      <span>{formatTimeAgo(selectedProof.createdAt)}</span>
                    </div>
                    {selectedProof.verifiedAt && (
                      <div className="flex justify-between">
                        <span className="text-gray-600">Verified:</span>
                        <span>{formatTimeAgo(selectedProof.verifiedAt)}</span>
                      </div>
                    )}
                    {selectedProof.expiresAt && (
                      <div className="flex justify-between">
                        <span className="text-gray-600">Expires:</span>
                        <span>{formatTimeRemaining(selectedProof.expiresAt)}</span>
                      </div>
                    )}
                  </div>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Technical Details</h3>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-600">Size:</span>
                      <span>{formatSize(selectedProof.size)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Verification Time:</span>
                      <span>{formatDuration(selectedProof.verificationTime)}</span>
                    </div>
                    {selectedProof.gasCost && (
                      <div className="flex justify-between">
                        <span className="text-gray-600">Gas Cost:</span>
                        <span>{selectedProof.gasCost.toLocaleString()}</span>
                      </div>
                    )}
                    <div className="flex justify-between">
                      <span className="text-gray-600">Trust Score:</span>
                      <span>{selectedProof.trustScore}/100</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Confidence:</span>
                      <span>{selectedProof.confidence}%</span>
                    </div>
                  </div>
                </div>
              </div>
              
              <div className="space-y-4">
                <div>
                  <h3 className="font-medium mb-2">Description</h3>
                  <p className="text-sm text-gray-600">{selectedProof.description}</p>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Public Inputs</h3>
                  <div className="space-y-1">
                    {selectedProof.publicInputs.map((input, index) => (
                      <div key={index} className="text-sm font-mono bg-gray-100 px-2 py-1 rounded">
                        {input}
                      </div>
                    ))}
                  </div>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Public Outputs</h3>
                  <div className="space-y-1">
                    {selectedProof.publicOutputs.map((output, index) => (
                      <div key={index} className="text-sm font-mono bg-gray-100 px-2 py-1 rounded">
                        {output}
                      </div>
                    ))}
                  </div>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Tags</h3>
                  <div className="flex flex-wrap gap-1">
                    {selectedProof.tags.map(tag => (
                      <Badge key={tag} variant="secondary">
                        {tag}
                      </Badge>
                    ))}
                  </div>
                </div>
                
                {selectedProof.metadata && (
                  <div>
                    <h3 className="font-medium mb-2">Metadata</h3>
                    <div className="space-y-2 text-sm">
                      {Object.entries(selectedProof.metadata).map(([key, value]) => (
                        <div key={key} className="flex justify-between">
                          <span className="text-gray-600">{key}:</span>
                          <span>{String(value)}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
            
            <div className="flex justify-end gap-3 mt-6">
              <Button variant="outline" onClick={() => setShowProofDetails(false)}>
                Close
              </Button>
              <Button onClick={() => {
                setProofId(selectedProof.id);
                setShowProofDetails(false);
                setActiveTab('verify');
              }}>
                <CheckCircle className="h-4 w-4 mr-2" />
                Verify This Proof
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Verification Result Modal */}
      {showVerificationModal && verificationResult && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-xl font-semibold">Verification Result</h2>
              <Button variant="outline" onClick={() => setShowVerificationModal(false)}>
                ×
              </Button>
            </div>
            
            <div className="space-y-4">
              <div className={`border rounded-lg p-4 ${verificationResult.isValid ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'}`}>
                <div className="flex items-center gap-3 mb-2">
                  {verificationResult.isValid ? (
                    <CheckCircle className="h-6 w-6 text-green-600" />
                  ) : (
                    <AlertCircle className="h-6 w-6 text-red-600" />
                  )}
                  <h3 className={`font-semibold ${verificationResult.isValid ? 'text-green-800' : 'text-red-800'}`}>
                    {verificationResult.isValid ? 'Verification Successful' : 'Verification Failed'}
                  </h3>
                </div>
                <p className={`text-sm ${verificationResult.isValid ? 'text-green-600' : 'text-red-600'}`}>
                  Proof #{verificationResult.proofId} has been {verificationResult.isValid ? 'verified' : 'rejected'}.
                </p>
              </div>
              
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-gray-600">Proof ID:</span>
                  <span className="font-mono text-xs">{verificationResult.proofId}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Verified At:</span>
                  <span>{formatTimeAgo(verificationResult.verifiedAt)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Verification Time:</span>
                  <span>{formatDuration(verificationResult.verificationTime)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-gray-600">Gas Used:</span>
                  <span>{verificationResult.gasUsed.toLocaleString()}</span>
                </div>
                {verificationResult.details && (
                  <div className="flex justify-between">
                    <span className="text-gray-600">Trust Score:</span>
                    <span>{verificationResult.details.trustScore}/100</span>
                  </div>
                )}
              </div>
              
              {verificationResult.error && (
                <div className="bg-red-50 border border-red-200 rounded p-3">
                  <h4 className="font-medium text-red-800 mb-1">Error Details</h4>
                  <p className="text-sm text-red-600">{verificationResult.error}</p>
                </div>
              )}
            </div>
            
            <div className="flex justify-end gap-3 mt-6">
              <Button variant="outline" onClick={() => setShowVerificationModal(false)}>
                Close
              </Button>
              <Button onClick={() => {
                setShowVerificationModal(false);
                setActiveTab('results');
              }}>
                View All Results
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
