import React, { useState, useEffect } from 'react';
import { Card, Button, Input, Badge, Select, SelectTrigger, SelectValue, SelectContent, SelectItem, Label, Textarea } from '../components/UI';
import {
  Upload, Database, Search, BarChart3, Download, Eye, Edit, Trash2, 
  Plus, Filter, Grid, List, Star, Users, Clock, CheckCircle, AlertCircle,
  FileText, Image, Music, Video, Code, Globe, Zap, Activity, HardDrive,
  TrendingUp, Shield, Link, Copy, ExternalLink, Info, Play, Pause
} from 'lucide-react';

// =================== Types ===================
type DatasetType = 'image' | 'text' | 'audio' | 'video' | 'tabular' | 'multimodal' | 'code';
type DatasetStatus = 'active' | 'processing' | 'archived' | 'failed' | 'uploading';
type DatasetLicense = 'MIT' | 'Apache-2.0' | 'GPL-3.0' | 'BSD-3-Clause' | 'Custom' | 'Commercial';
type DatasetVisibility = 'public' | 'private' | 'restricted';

type Dataset = {
  id: string;
  name: string;
  description: string;
  type: DatasetType;
  status: DatasetStatus;
  license: DatasetLicense;
  visibility: DatasetVisibility;
  size: number; // bytes
  records: number;
  owner: string;
  createdAt: string;
  updatedAt: string;
  version: string;
  tags: string[];
  downloadCount: number;
  rating: number;
  qualityScore: number;
  format: string;
  schema?: Record<string, any>;
  preview?: any[];
  metadata?: Record<string, any>;
  checksum: string;
  replicas: number;
  lastAccessed?: string;
};

type DatasetJob = {
  id: string;
  datasetId: string;
  type: 'training' | 'inference' | 'validation' | 'preprocessing';
  status: 'pending' | 'running' | 'completed' | 'failed';
  progress: number;
  startedAt: string;
  completedAt?: string;
  modelId?: string;
  results?: any;
};

// =================== Mock Data ===================
const generateMockDatasets = (): Dataset[] => [
  {
    id: 'dataset_001',
    name: 'ImageNet-21K',
    description: 'Large-scale image dataset with 21,841 classes and 14 million images',
    type: 'image',
    status: 'active',
    license: 'Apache-2.0',
    visibility: 'public',
    size: 150 * 1024 * 1024 * 1024, // 150 GB
    records: 14000000,
    owner: 'i0000000000000000000000000000000000000000000000000000000000000000',
    createdAt: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString(),
    updatedAt: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString(),
    version: 'v2.1',
    tags: ['computer-vision', 'classification', 'imagenet', 'large-scale'],
    downloadCount: 45230,
    rating: 4.8,
    qualityScore: 98.5,
    format: 'JPEG',
    checksum: 'sha256:abc123...',
    replicas: 5,
    lastAccessed: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
    metadata: {
      resolution: '224x224',
      colorSpace: 'RGB',
      compression: 'JPEG'
    }
  },
  {
    id: 'dataset_002',
    name: 'Common Crawl Text',
    description: 'Massive web crawl dataset for language modeling and NLP research',
    type: 'text',
    status: 'active',
    license: 'Apache-2.0',
    visibility: 'public',
    size: 45 * 1024 * 1024 * 1024, // 45 GB
    records: 5000000000,
    owner: 'i1B2zP2eP6QGefi3DMPTfTL6SLmv8DivfNb',
    createdAt: new Date(Date.now() - 15 * 24 * 60 * 60 * 1000).toISOString(),
    updatedAt: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000).toISOString(),
    version: 'v1.3',
    tags: ['nlp', 'language-modeling', 'web-crawl', 'text'],
    downloadCount: 12890,
    rating: 4.6,
    qualityScore: 95.2,
    format: 'JSONL',
    checksum: 'sha256:def456...',
    replicas: 3,
    lastAccessed: new Date(Date.now() - 30 * 60 * 1000).toISOString(),
    metadata: {
      languages: ['en', 'es', 'fr', 'de'],
      avgTokensPerDoc: 512,
      totalTokens: 2.5e12
    }
  },
  {
    id: 'dataset_003',
    name: 'LibriSpeech Audio',
    description: 'Large-scale English speech recognition dataset from audiobooks',
    type: 'audio',
    status: 'active',
    license: 'MIT',
    visibility: 'public',
    size: 100 * 1024 * 1024 * 1024, // 100 GB
    records: 1000,
    owner: 'i1C3zP3eP7QGefi4DMPTfTL7SLmv9DivfNc',
    createdAt: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
    updatedAt: new Date(Date.now() - 6 * 60 * 60 * 1000).toISOString(),
    version: 'v1.0',
    tags: ['speech', 'recognition', 'audio', 'english'],
    downloadCount: 8750,
    rating: 4.9,
    qualityScore: 99.1,
    format: 'FLAC',
    checksum: 'sha256:ghi789...',
    replicas: 4,
    lastAccessed: new Date(Date.now() - 15 * 60 * 1000).toISOString(),
    metadata: {
      sampleRate: '16kHz',
      duration: '1000 hours',
      speakers: 2338
    }
  },
  {
    id: 'dataset_004',
    name: 'Financial Time Series',
    description: 'High-frequency trading data with market indicators and price movements',
    type: 'tabular',
    status: 'processing',
    license: 'Commercial',
    visibility: 'restricted',
    size: 2.5 * 1024 * 1024 * 1024, // 2.5 GB
    records: 50000000,
    owner: 'i0000000000000000000000000000000000000000000000000000000000000000',
    createdAt: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000).toISOString(),
    updatedAt: new Date(Date.now() - 30 * 60 * 1000).toISOString(),
    version: 'v0.9',
    tags: ['finance', 'time-series', 'trading', 'market-data'],
    downloadCount: 2340,
    rating: 4.3,
    qualityScore: 92.8,
    format: 'Parquet',
    checksum: 'sha256:jkl012...',
    replicas: 2,
    metadata: {
      timeRange: '2020-2024',
      frequency: '1-minute',
      symbols: 5000
    }
  },
  {
    id: 'dataset_005',
    name: 'CodeSearchNet',
    description: 'Large-scale dataset of code with natural language queries',
    type: 'code',
    status: 'active',
    license: 'MIT',
    visibility: 'public',
    size: 8.7 * 1024 * 1024 * 1024, // 8.7 GB
    records: 2000000,
    owner: 'i1D4zP4eP8QGefi5DMPTfTL8SLmv0DivfNd',
    createdAt: new Date(Date.now() - 20 * 24 * 60 * 60 * 1000).toISOString(),
    updatedAt: new Date(Date.now() - 12 * 60 * 60 * 1000).toISOString(),
    version: 'v1.2',
    tags: ['code', 'search', 'nlp', 'programming'],
    downloadCount: 15670,
    rating: 4.7,
    qualityScore: 96.4,
    format: 'JSON',
    checksum: 'sha256:mno345...',
    replicas: 3,
    lastAccessed: new Date(Date.now() - 45 * 60 * 1000).toISOString(),
    metadata: {
      languages: ['python', 'javascript', 'java', 'go'],
      avgLinesPerFile: 45,
      totalLines: 90e6
    }
  }
];

// =================== Utils ===================
function bytesToSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

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

function getStatusColor(status: DatasetStatus): string {
  switch (status) {
    case 'active': return 'bg-green-100 text-green-800';
    case 'processing': return 'bg-yellow-100 text-yellow-800';
    case 'archived': return 'bg-gray-100 text-gray-800';
    case 'failed': return 'bg-red-100 text-red-800';
    case 'uploading': return 'bg-blue-100 text-blue-800';
    default: return 'bg-gray-100 text-gray-800';
  }
}

function getTypeIcon(type: DatasetType): string {
  switch (type) {
    case 'image': return '🖼️';
    case 'text': return '📝';
    case 'audio': return '🎵';
    case 'video': return '🎬';
    case 'code': return '💻';
    case 'tabular': return '📊';
    case 'multimodal': return '🔀';
    default: return '📁';
  }
}

// =================== Component ===================
export default function DatasetsPage() {
  // Data
  const [datasets, setDatasets] = useState<Dataset[]>([]);
  const [selectedDataset, setSelectedDataset] = useState<Dataset | null>(null);
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [typeFilter, setTypeFilter] = useState<'all' | DatasetType>('all');
  const [statusFilter, setStatusFilter] = useState<'all' | DatasetStatus>('all');
  const [visibilityFilter, setVisibilityFilter] = useState<'all' | DatasetVisibility>('all');

  // Upload form
  const [showUploadModal, setShowUploadModal] = useState(false);
  const [uploadForm, setUploadForm] = useState({
    name: '',
    description: '',
    type: 'text' as DatasetType,
    license: 'MIT' as DatasetLicense,
    visibility: 'public' as DatasetVisibility,
    tags: '',
    file: null as File | null
  });
  const [isUploading, setIsUploading] = useState(false);

  // Details modal
  const [showDetailsModal, setShowDetailsModal] = useState(false);

  // Jobs
  const [jobs, setJobs] = useState<DatasetJob[]>([]);

  // Initialize data
  useEffect(() => {
    setDatasets(generateMockDatasets());
    
    // Mock jobs
    setJobs([
      {
        id: 'job_001',
        datasetId: 'dataset_001',
        type: 'training',
        status: 'running',
        progress: 65,
        startedAt: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
        modelId: 'model_001'
      },
      {
        id: 'job_002',
        datasetId: 'dataset_002',
        type: 'preprocessing',
        status: 'completed',
        progress: 100,
        startedAt: new Date(Date.now() - 4 * 60 * 60 * 1000).toISOString(),
        completedAt: new Date(Date.now() - 1 * 60 * 60 * 1000).toISOString()
      }
    ]);
  }, []);

  // Filter datasets
  const filteredDatasets = datasets.filter(dataset => {
    const matchesSearch = dataset.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         dataset.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         dataset.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesType = typeFilter === 'all' || dataset.type === typeFilter;
    const matchesStatus = statusFilter === 'all' || dataset.status === statusFilter;
    const matchesVisibility = visibilityFilter === 'all' || dataset.visibility === visibilityFilter;
    
    return matchesSearch && matchesType && matchesStatus && matchesVisibility;
  });

  // Upload dataset
  const handleUpload = async () => {
    if (!uploadForm.name || !uploadForm.file) return;
    
    setIsUploading(true);
    
    // Simulate upload
    await new Promise(resolve => setTimeout(resolve, 3000));
    
    const newDataset: Dataset = {
      id: `dataset_${Date.now()}`,
      name: uploadForm.name,
      description: uploadForm.description,
      type: uploadForm.type,
      status: 'active',
      license: uploadForm.license,
      visibility: uploadForm.visibility,
      size: uploadForm.file?.size || 0,
      records: Math.floor(Math.random() * 1000000),
      owner: 'i0000000000000000000000000000000000000000000000000000000000000000',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      version: 'v1.0',
      tags: uploadForm.tags.split(',').map(t => t.trim()).filter(Boolean),
      downloadCount: 0,
      rating: 0,
      qualityScore: 95.0,
      format: uploadForm.file?.name.split('.').pop()?.toUpperCase() || 'UNKNOWN',
      checksum: 'sha256:new...',
      replicas: 1
    };
    
    setDatasets(prev => [newDataset, ...prev]);
    setShowUploadModal(false);
    setIsUploading(false);
    
    // Reset form
    setUploadForm({
      name: '',
      description: '',
      type: 'text',
      license: 'MIT',
      visibility: 'public',
      tags: '',
      file: null
    });
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <header className="flex justify-between items-start">
        <div className="space-y-1">
          <h1 className="text-3xl font-bold text-gray-900">Datasets</h1>
          <p className="text-gray-600">Discover, upload, and manage datasets for AI training and research</p>
        </div>
        <div className="flex items-center gap-3">
          <Button 
            onClick={() => setViewMode(viewMode === 'grid' ? 'list' : 'grid')}
            variant="outline"
            size="sm"
          >
            {viewMode === 'grid' ? 'List View' : 'Grid View'}
          </Button>
          <Button onClick={() => setShowUploadModal(true)}>
            <Plus className="h-4 w-4 mr-2" />
            Upload Dataset
          </Button>
        </div>
      </header>

      {/* Statistics Dashboard */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Datasets</p>
              <p className="text-2xl font-bold text-gray-900">{datasets.length}</p>
            </div>
            <Database className="h-8 w-8 text-blue-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Size</p>
              <p className="text-2xl font-bold text-gray-900">
                {bytesToSize(datasets.reduce((sum, d) => sum + d.size, 0))}
              </p>
            </div>
            <HardDrive className="h-8 w-8 text-purple-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Active Jobs</p>
              <p className="text-2xl font-bold text-orange-600">
                {jobs.filter(j => j.status === 'running').length}
              </p>
            </div>
            <Activity className="h-8 w-8 text-orange-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Avg Quality</p>
              <p className="text-2xl font-bold text-green-600">
                {(datasets.reduce((sum, d) => sum + d.qualityScore, 0) / datasets.length).toFixed(1)}%
              </p>
            </div>
            <Shield className="h-8 w-8 text-green-600" />
          </div>
        </Card>
      </div>

      {/* Filters */}
      <Card>
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex items-center gap-2">
            <Search className="h-4 w-4 text-gray-400" />
            <Input 
              placeholder="Search datasets..." 
              value={searchQuery} 
              onChange={e => setSearchQuery(e.target.value)}
              className="w-64"
            />
          </div>
          <Select value={typeFilter} onValueChange={(v) => setTypeFilter(v as any)}>
            <SelectTrigger className="w-40">
              <SelectValue placeholder="Type" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Types</SelectItem>
              <SelectItem value="image">Image</SelectItem>
              <SelectItem value="text">Text</SelectItem>
              <SelectItem value="audio">Audio</SelectItem>
              <SelectItem value="video">Video</SelectItem>
              <SelectItem value="code">Code</SelectItem>
              <SelectItem value="tabular">Tabular</SelectItem>
              <SelectItem value="multimodal">Multimodal</SelectItem>
            </SelectContent>
          </Select>
          <Select value={statusFilter} onValueChange={(v) => setStatusFilter(v as any)}>
            <SelectTrigger className="w-40">
              <SelectValue placeholder="Status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Status</SelectItem>
              <SelectItem value="active">Active</SelectItem>
              <SelectItem value="processing">Processing</SelectItem>
              <SelectItem value="archived">Archived</SelectItem>
              <SelectItem value="failed">Failed</SelectItem>
              <SelectItem value="uploading">Uploading</SelectItem>
            </SelectContent>
          </Select>
          <Select value={visibilityFilter} onValueChange={(v) => setVisibilityFilter(v as any)}>
            <SelectTrigger className="w-40">
              <SelectValue placeholder="Visibility" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Visibility</SelectItem>
              <SelectItem value="public">Public</SelectItem>
              <SelectItem value="private">Private</SelectItem>
              <SelectItem value="restricted">Restricted</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </Card>

      {/* Datasets Grid/List */}
      <Card>
        <div className="space-y-4">
          {viewMode === 'grid' ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              {filteredDatasets.map(dataset => (
                <div key={dataset.id} className="border rounded-lg p-4 hover:shadow-md transition-shadow">
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-2">
                      <span className="text-2xl">{getTypeIcon(dataset.type)}</span>
                      <div>
                        <h3 className="font-semibold text-gray-900">{dataset.name}</h3>
                        <p className="text-sm text-gray-600">{dataset.type} • {dataset.format}</p>
                      </div>
                    </div>
                    <Badge className={getStatusColor(dataset.status)}>
                      {dataset.status}
                    </Badge>
                  </div>
                  
                  <p className="text-sm text-gray-600 mb-3 line-clamp-2">{dataset.description}</p>
                  
                  <div className="space-y-2 mb-4">
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Size:</span>
                      <span className="font-medium">{bytesToSize(dataset.size)}</span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Records:</span>
                      <span className="font-medium">{dataset.records.toLocaleString()}</span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Downloads:</span>
                      <span className="font-medium">{dataset.downloadCount.toLocaleString()}</span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Quality:</span>
                      <span className="font-medium">{dataset.qualityScore}%</span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Rating:</span>
                      <div className="flex items-center gap-1">
                        <Star className="h-3 w-3 fill-yellow-400 text-yellow-400" />
                        <span className="font-medium">{dataset.rating}</span>
                      </div>
                    </div>
                  </div>
                  
                  <div className="flex flex-wrap gap-1 mb-4">
                    {dataset.tags.slice(0, 3).map(tag => (
                      <Badge key={tag} variant="secondary" className="text-xs">
                        {tag}
                      </Badge>
                    ))}
                    {dataset.tags.length > 3 && (
                      <Badge variant="secondary" className="text-xs">
                        +{dataset.tags.length - 3}
                      </Badge>
                    )}
                  </div>
                  
                  <div className="flex gap-2">
                    <Button 
                      size="sm" 
                      variant="outline"
                      onClick={() => {
                        setSelectedDataset(dataset);
                        setShowDetailsModal(true);
                      }}
                    >
                      <Eye className="h-3 w-3 mr-1" />
                      Details
                    </Button>
                    <Button size="sm">
                      <Download className="h-3 w-3 mr-1" />
                      Download
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="space-y-3">
              {filteredDatasets.map(dataset => (
                <div key={dataset.id} className="border rounded-lg p-4 hover:bg-gray-50">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-4">
                      <span className="text-2xl">{getTypeIcon(dataset.type)}</span>
                      <div>
                        <div className="flex items-center gap-2">
                          <h3 className="font-semibold text-gray-900">{dataset.name}</h3>
                          <Badge className={getStatusColor(dataset.status)}>
                            {dataset.status}
                          </Badge>
                        </div>
                        <p className="text-sm text-gray-600">{dataset.description}</p>
                        <p className="text-xs text-gray-500">
                          {bytesToSize(dataset.size)} • {dataset.records.toLocaleString()} records • 
                          {dataset.downloadCount.toLocaleString()} downloads • 
                          <Star className="h-3 w-3 inline fill-yellow-400 text-yellow-400 mx-1" />
                          {dataset.rating} • Quality: {dataset.qualityScore}%
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button 
                        size="sm" 
                        variant="outline"
                        onClick={() => {
                          setSelectedDataset(dataset);
                          setShowDetailsModal(true);
                        }}
                      >
                        <Eye className="h-3 w-3 mr-1" />
                        Details
                      </Button>
                      <Button size="sm">
                        <Download className="h-3 w-3 mr-1" />
                        Download
                      </Button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
          
          {filteredDatasets.length === 0 && (
            <div className="text-center py-12">
              <Database className="h-12 w-12 text-gray-400 mx-auto mb-4" />
              <h3 className="text-lg font-medium text-gray-900 mb-2">No datasets found</h3>
              <p className="text-gray-600 mb-4">Try adjusting your search criteria or upload a new dataset.</p>
              <Button onClick={() => setShowUploadModal(true)}>
                <Plus className="h-4 w-4 mr-2" />
                Upload Dataset
              </Button>
            </div>
          )}
        </div>
      </Card>

      {/* Upload Modal */}
      {showUploadModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-xl font-semibold">Upload Dataset</h2>
              <Button variant="outline" onClick={() => setShowUploadModal(false)}>
                ×
              </Button>
            </div>
            
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Dataset Name</Label>
                  <Input 
                    value={uploadForm.name} 
                    onChange={e => setUploadForm(prev => ({ ...prev, name: e.target.value }))}
                    placeholder="My Dataset"
                  />
                </div>
                <div>
                  <Label>Type</Label>
                  <Select value={uploadForm.type} onValueChange={(v) => setUploadForm(prev => ({ ...prev, type: v as DatasetType }))}>
                    <SelectTrigger><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="image">Image</SelectItem>
                      <SelectItem value="text">Text</SelectItem>
                      <SelectItem value="audio">Audio</SelectItem>
                      <SelectItem value="video">Video</SelectItem>
                      <SelectItem value="code">Code</SelectItem>
                      <SelectItem value="tabular">Tabular</SelectItem>
                      <SelectItem value="multimodal">Multimodal</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              
              <div>
                <Label>Description</Label>
                <Textarea 
                  value={uploadForm.description} 
                  onChange={e => setUploadForm(prev => ({ ...prev, description: e.target.value }))}
                  placeholder="Describe your dataset..."
                  rows={3}
                />
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>License</Label>
                  <Select value={uploadForm.license} onValueChange={(v) => setUploadForm(prev => ({ ...prev, license: v as DatasetLicense }))}>
                    <SelectTrigger><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="MIT">MIT</SelectItem>
                      <SelectItem value="Apache-2.0">Apache-2.0</SelectItem>
                      <SelectItem value="GPL-3.0">GPL-3.0</SelectItem>
                      <SelectItem value="BSD-3-Clause">BSD-3-Clause</SelectItem>
                      <SelectItem value="Custom">Custom</SelectItem>
                      <SelectItem value="Commercial">Commercial</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label>Visibility</Label>
                  <Select value={uploadForm.visibility} onValueChange={(v) => setUploadForm(prev => ({ ...prev, visibility: v as DatasetVisibility }))}>
                    <SelectTrigger><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="public">Public</SelectItem>
                      <SelectItem value="private">Private</SelectItem>
                      <SelectItem value="restricted">Restricted</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              
              <div>
                <Label>Tags (comma separated)</Label>
                <Input 
                  value={uploadForm.tags} 
                  onChange={e => setUploadForm(prev => ({ ...prev, tags: e.target.value }))}
                  placeholder="machine-learning, nlp, research"
                />
              </div>
              
              <div>
                <Label>Dataset File</Label>
                <div className="border-2 border-dashed border-gray-300 rounded-lg p-6 text-center">
                  <Upload className="h-8 w-8 text-gray-400 mx-auto mb-2" />
                  <p className="text-sm text-gray-600 mb-2">Drop your dataset file here or click to browse</p>
                  <Input 
                    type="file" 
                    onChange={e => setUploadForm(prev => ({ ...prev, file: e.target.files?.[0] || null }))}
                    className="max-w-xs mx-auto"
                  />
                  {uploadForm.file && (
                    <p className="text-sm text-green-600 mt-2">
                      Selected: {uploadForm.file.name} ({bytesToSize(uploadForm.file.size)})
                    </p>
                  )}
                </div>
              </div>
            </div>
            
            <div className="flex justify-end gap-3 mt-6">
              <Button variant="outline" onClick={() => setShowUploadModal(false)}>
                Cancel
              </Button>
              <Button 
                onClick={handleUpload}
                disabled={!uploadForm.name || !uploadForm.file || isUploading}
              >
                {isUploading ? (
                  <Activity className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Upload className="h-4 w-4 mr-2" />
                )}
                {isUploading ? 'Uploading...' : 'Upload Dataset'}
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Dataset Details Modal */}
      {showDetailsModal && selectedDataset && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-4xl max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-6">
              <div className="flex items-center gap-3">
                <span className="text-3xl">{getTypeIcon(selectedDataset.type)}</span>
                <div>
                  <h2 className="text-xl font-semibold">{selectedDataset.name}</h2>
                  <p className="text-gray-600">{selectedDataset.type} • {selectedDataset.format} • v{selectedDataset.version}</p>
                </div>
              </div>
              <Button variant="outline" onClick={() => setShowDetailsModal(false)}>
                ×
              </Button>
            </div>
            
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="space-y-4">
                <div>
                  <h3 className="font-medium mb-2">Dataset Information</h3>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-600">Dataset ID:</span>
                      <span className="font-mono text-xs">{selectedDataset.id}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Owner:</span>
                      <span className="font-mono text-xs">{selectedDataset.owner}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">License:</span>
                      <span>{selectedDataset.license}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Visibility:</span>
                      <Badge className={getStatusColor(selectedDataset.visibility as any)}>
                        {selectedDataset.visibility}
                      </Badge>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Status:</span>
                      <Badge className={getStatusColor(selectedDataset.status)}>
                        {selectedDataset.status}
                      </Badge>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Checksum:</span>
                      <span className="font-mono text-xs">{selectedDataset.checksum}</span>
                    </div>
                  </div>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Statistics</h3>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-600">Size:</span>
                      <span>{bytesToSize(selectedDataset.size)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Records:</span>
                      <span>{selectedDataset.records.toLocaleString()}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Downloads:</span>
                      <span>{selectedDataset.downloadCount.toLocaleString()}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Quality Score:</span>
                      <span>{selectedDataset.qualityScore}%</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Rating:</span>
                      <div className="flex items-center gap-1">
                        <Star className="h-3 w-3 fill-yellow-400 text-yellow-400" />
                        <span>{selectedDataset.rating}</span>
                      </div>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Replicas:</span>
                      <span>{selectedDataset.replicas}</span>
                    </div>
                  </div>
                </div>
              </div>
              
              <div className="space-y-4">
                <div>
                  <h3 className="font-medium mb-2">Description</h3>
                  <p className="text-sm text-gray-600">{selectedDataset.description}</p>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Tags</h3>
                  <div className="flex flex-wrap gap-1">
                    {selectedDataset.tags.map(tag => (
                      <Badge key={tag} variant="secondary">
                        {tag}
                      </Badge>
                    ))}
                  </div>
                </div>
                
                {selectedDataset.metadata && (
                  <div>
                    <h3 className="font-medium mb-2">Metadata</h3>
                    <div className="space-y-2 text-sm">
                      {Object.entries(selectedDataset.metadata).map(([key, value]) => (
                        <div key={key} className="flex justify-between">
                          <span className="text-gray-600">{key}:</span>
                          <span>{String(value)}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
                
                <div>
                  <h3 className="font-medium mb-2">Timestamps</h3>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-600">Created:</span>
                      <span>{formatTimeAgo(selectedDataset.createdAt)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Updated:</span>
                      <span>{formatTimeAgo(selectedDataset.updatedAt)}</span>
                    </div>
                    {selectedDataset.lastAccessed && (
                      <div className="flex justify-between">
                        <span className="text-gray-600">Last Accessed:</span>
                        <span>{formatTimeAgo(selectedDataset.lastAccessed)}</span>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </div>
            
            <div className="flex justify-end gap-3 mt-6">
              <Button variant="outline" onClick={() => setShowDetailsModal(false)}>
                Close
              </Button>
              <Button>
                <Download className="h-4 w-4 mr-2" />
                Download Dataset
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
