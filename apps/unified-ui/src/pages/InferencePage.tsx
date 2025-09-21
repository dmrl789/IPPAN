import React, { useState, useEffect } from 'react';
import { Card, Button, Input, Badge, Select, SelectTrigger, SelectValue, SelectContent, SelectItem, Label, Textarea } from '../components/UI';
import {
  Play, Pause, Download, Eye, Edit, Trash2, Plus, Filter, Grid, List, 
  Star, Users, Clock, CheckCircle, AlertCircle, Brain, Cpu, HardDrive,
  Network, Activity, Zap, TrendingUp, Shield, Link, Copy, ExternalLink,
  Info, Settings, BarChart3, FileText, Image, Music, Video, Code, Globe,
  Upload, RefreshCw, Save, Share2, Bookmark, History, Timer, Target
} from 'lucide-react';

// =================== Types ===================
type ModelType = 'text' | 'image' | 'audio' | 'video' | 'multimodal' | 'code' | 'tabular';
type InferenceStatus = 'pending' | 'processing' | 'completed' | 'failed' | 'cancelled';
type InferenceMode = 'single' | 'batch' | 'streaming';

type Model = {
  id: string;
  name: string;
  description: string;
  type: ModelType;
  version: string;
  owner: string;
  status: 'active' | 'inactive' | 'training' | 'deployed';
  parameters: number; // in millions
  size: number; // in MB
  accuracy: number; // percentage
  latency: number; // ms
  cost: number; // per inference
  tags: string[];
  endpoint?: string;
  lastUsed?: string;
  usageCount: number;
};

type InferenceJob = {
  id: string;
  modelId: string;
  modelName: string;
  input: string;
  output?: string;
  status: InferenceStatus;
  mode: InferenceMode;
  progress: number;
  startedAt: string;
  completedAt?: string;
  duration?: number; // ms
  cost?: number;
  error?: string;
  metadata?: Record<string, any>;
};

type InferenceTemplate = {
  id: string;
  name: string;
  description: string;
  modelType: ModelType;
  inputTemplate: string;
  outputExample: string;
  category: string;
  tags: string[];
};

// =================== Mock Data ===================
const generateMockModels = (): Model[] => [
  {
    id: 'model_001',
    name: 'GPT-4o',
    description: 'Advanced multimodal language model for text and image understanding',
    type: 'multimodal',
    version: 'v4.0',
    owner: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
    status: 'deployed',
    parameters: 175000,
    size: 350000,
    accuracy: 95.8,
    latency: 1200,
    cost: 0.03,
    tags: ['language-model', 'multimodal', 'gpt', 'openai'],
    endpoint: 'https://api.ippan.net/v1/models/gpt-4o',
    lastUsed: new Date(Date.now() - 5 * 60 * 1000).toISOString(),
    usageCount: 15420
  },
  {
    id: 'model_002',
    name: 'CLIP-ViT-Large',
    description: 'Vision-language model for image understanding and text-image matching',
    type: 'image',
    version: 'v1.0',
    owner: 'i1B2zP2eP6QGefi3DMPTfTL6SLmv8DivfNb',
    status: 'deployed',
    parameters: 428,
    size: 1200,
    accuracy: 88.5,
    latency: 800,
    cost: 0.015,
    tags: ['vision', 'clip', 'vit', 'image-understanding'],
    endpoint: 'https://api.ippan.net/v1/models/clip-vit-large',
    lastUsed: new Date(Date.now() - 15 * 60 * 1000).toISOString(),
    usageCount: 8930
  },
  {
    id: 'model_003',
    name: 'Whisper-Large-v3',
    description: 'Automatic speech recognition model for multilingual audio transcription',
    type: 'audio',
    version: 'v3.0',
    owner: 'i1C3zP3eP7QGefi4DMPTfTL7SLmv9DivfNc',
    status: 'deployed',
    parameters: 1550,
    size: 3100,
    accuracy: 92.3,
    latency: 2000,
    cost: 0.02,
    tags: ['speech', 'asr', 'whisper', 'multilingual'],
    endpoint: 'https://api.ippan.net/v1/models/whisper-large-v3',
    lastUsed: new Date(Date.now() - 30 * 60 * 1000).toISOString(),
    usageCount: 5670
  },
  {
    id: 'model_004',
    name: 'CodeLlama-34B',
    description: 'Large language model specialized for code generation and understanding',
    type: 'code',
    version: 'v1.1',
    owner: 'i1D4zP4eP8QGefi5DMPTfTL8SLmv0DivfNd',
    status: 'deployed',
    parameters: 34000,
    size: 68000,
    accuracy: 89.7,
    latency: 1500,
    cost: 0.025,
    tags: ['code', 'llama', 'programming', 'generation'],
    endpoint: 'https://api.ippan.net/v1/models/codellama-34b',
    lastUsed: new Date(Date.now() - 45 * 60 * 1000).toISOString(),
    usageCount: 12340
  },
  {
    id: 'model_005',
    name: 'Stable Diffusion XL',
    description: 'High-quality image generation model for text-to-image synthesis',
    type: 'image',
    version: 'v1.0',
    owner: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
    status: 'deployed',
    parameters: 3500,
    size: 14000,
    accuracy: 91.2,
    latency: 3000,
    cost: 0.04,
    tags: ['generation', 'diffusion', 'image', 'art'],
    endpoint: 'https://api.ippan.net/v1/models/stable-diffusion-xl',
    lastUsed: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
    usageCount: 9870
  }
];

const generateMockTemplates = (): InferenceTemplate[] => [
  {
    id: 'template_001',
    name: 'Text Summarization',
    description: 'Summarize long text documents into concise summaries',
    modelType: 'text',
    inputTemplate: 'Summarize the following text:\n\n[Your text here]',
    outputExample: 'This text discusses [topic] and covers [key points]. The main conclusion is [summary].',
    category: 'NLP',
    tags: ['summarization', 'text', 'nlp']
  },
  {
    id: 'template_002',
    name: 'Image Classification',
    description: 'Classify images into predefined categories',
    modelType: 'image',
    inputTemplate: 'Upload an image to classify',
    outputExample: 'Image classified as: [category] (confidence: 95.2%)',
    category: 'Computer Vision',
    tags: ['classification', 'image', 'vision']
  },
  {
    id: 'template_003',
    name: 'Code Generation',
    description: 'Generate code based on natural language descriptions',
    modelType: 'code',
    inputTemplate: 'Write a function that [description]',
    outputExample: 'def function_name():\n    # Generated code here\n    return result',
    category: 'Programming',
    tags: ['code', 'generation', 'programming']
  },
  {
    id: 'template_004',
    name: 'Sentiment Analysis',
    description: 'Analyze the sentiment of text (positive, negative, neutral)',
    modelType: 'text',
    inputTemplate: 'Analyze the sentiment of: "[text]"',
    outputExample: 'Sentiment: Positive (confidence: 87.3%)',
    category: 'NLP',
    tags: ['sentiment', 'analysis', 'nlp']
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

function getStatusColor(status: InferenceStatus): string {
  switch (status) {
    case 'completed': return 'bg-green-100 text-green-800';
    case 'processing': return 'bg-blue-100 text-blue-800';
    case 'pending': return 'bg-yellow-100 text-yellow-800';
    case 'failed': return 'bg-red-100 text-red-800';
    case 'cancelled': return 'bg-gray-100 text-gray-800';
    default: return 'bg-gray-100 text-gray-800';
  }
}

function getTypeIcon(type: ModelType): string {
  switch (type) {
    case 'text': return '📝';
    case 'image': return '🖼️';
    case 'audio': return '🎵';
    case 'video': return '🎬';
    case 'code': return '💻';
    case 'multimodal': return '🔀';
    case 'tabular': return '📊';
    default: return '🤖';
  }
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

// =================== Component ===================
export default function InferencePage() {
  // Data
  const [models, setModels] = useState<Model[]>([]);
  const [templates, setTemplates] = useState<InferenceTemplate[]>([]);
  const [jobs, setJobs] = useState<InferenceJob[]>([]);
  const [selectedModel, setSelectedModel] = useState<Model | null>(null);
  const [selectedTemplate, setSelectedTemplate] = useState<InferenceTemplate | null>(null);

  // Inference form
  const [inputText, setInputText] = useState('');
  const [inputFile, setInputFile] = useState<File | null>(null);
  const [inferenceMode, setInferenceMode] = useState<InferenceMode>('single');
  const [isProcessing, setIsProcessing] = useState(false);
  const [currentJob, setCurrentJob] = useState<InferenceJob | null>(null);

  // UI state
  const [activeTab, setActiveTab] = useState<'models' | 'templates' | 'history'>('models');
  const [showModelDetails, setShowModelDetails] = useState(false);
  const [showJobDetails, setShowJobDetails] = useState(false);
  const [selectedJob, setSelectedJob] = useState<InferenceJob | null>(null);

  // Initialize data
  useEffect(() => {
    setModels(generateMockModels());
    setTemplates(generateMockTemplates());
    
    // Mock recent jobs
    setJobs([
      {
        id: 'job_001',
        modelId: 'model_001',
        modelName: 'GPT-4o',
        input: 'Write a Python function to calculate fibonacci numbers',
        output: 'def fibonacci(n):\n    if n <= 1:\n        return n\n    return fibonacci(n-1) + fibonacci(n-2)',
        status: 'completed',
        mode: 'single',
        progress: 100,
        startedAt: new Date(Date.now() - 5 * 60 * 1000).toISOString(),
        completedAt: new Date(Date.now() - 4 * 60 * 1000).toISOString(),
        duration: 1200,
        cost: 0.03
      },
      {
        id: 'job_002',
        modelId: 'model_002',
        modelName: 'CLIP-ViT-Large',
        input: 'cat.jpg',
        output: 'Image classified as: Cat (confidence: 95.2%)',
        status: 'completed',
        mode: 'single',
        progress: 100,
        startedAt: new Date(Date.now() - 15 * 60 * 1000).toISOString(),
        completedAt: new Date(Date.now() - 14 * 60 * 1000).toISOString(),
        duration: 800,
        cost: 0.015
      },
      {
        id: 'job_003',
        modelId: 'model_003',
        modelName: 'Whisper-Large-v3',
        input: 'audio.mp3',
        output: 'Transcription: Hello, this is a test of the speech recognition system.',
        status: 'processing',
        mode: 'single',
        progress: 65,
        startedAt: new Date(Date.now() - 2 * 60 * 1000).toISOString()
      }
    ]);
  }, []);

  // Run inference
  const handleInference = async () => {
    if (!selectedModel || (!inputText && !inputFile)) return;
    
    setIsProcessing(true);
    
    const newJob: InferenceJob = {
      id: `job_${Date.now()}`,
      modelId: selectedModel.id,
      modelName: selectedModel.name,
      input: inputText || inputFile?.name || '',
      status: 'processing',
      mode: inferenceMode,
      progress: 0,
      startedAt: new Date().toISOString()
    };
    
    setCurrentJob(newJob);
    setJobs(prev => [newJob, ...prev]);
    
    // Simulate processing
    for (let i = 0; i <= 100; i += 10) {
      await new Promise(resolve => setTimeout(resolve, 200));
      setCurrentJob(prev => prev ? { ...prev, progress: i } : null);
      setJobs(prev => prev.map(job => 
        job.id === newJob.id ? { ...job, progress: i } : job
      ));
    }
    
    // Complete the job
    const completedJob: InferenceJob = {
      ...newJob,
      status: 'completed',
      progress: 100,
      completedAt: new Date().toISOString(),
      duration: selectedModel.latency + Math.random() * 500,
      cost: selectedModel.cost,
      output: generateMockOutput(selectedModel, inputText)
    };
    
    setCurrentJob(null);
    setJobs(prev => prev.map(job => 
      job.id === newJob.id ? completedJob : job
    ));
    
    setIsProcessing(false);
    setInputText('');
    setInputFile(null);
  };

  // Generate mock output based on model type
  const generateMockOutput = (model: Model, input: string): string => {
    switch (model.type) {
      case 'text':
        return `Generated response: "${input}" has been processed by ${model.name}. This is a sample output demonstrating the model's capabilities.`;
      case 'image':
        return `Image analysis complete. Detected objects: [cat, tree, sky] with confidence scores: [0.95, 0.87, 0.92]`;
      case 'audio':
        return `Transcription: "${input}" - Audio processed successfully with 95.2% confidence.`;
      case 'code':
        return `Generated code:\n\ndef solution():\n    # ${input}\n    return "Implementation complete"`;
      case 'multimodal':
        return `Multimodal analysis: Input contains both text and visual elements. Processing complete with 92.1% accuracy.`;
      default:
        return `Processing complete for ${model.name}. Output generated successfully.`;
    }
  };

  // Apply template
  const applyTemplate = (template: InferenceTemplate) => {
    setSelectedTemplate(template);
    setInputText(template.inputTemplate);
    // Find a compatible model
    const compatibleModel = models.find(m => m.type === template.modelType && m.status === 'deployed');
    if (compatibleModel) {
      setSelectedModel(compatibleModel);
    }
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <header className="flex justify-between items-start">
        <div className="space-y-1">
          <h1 className="text-3xl font-bold text-gray-900">AI Inference</h1>
          <p className="text-gray-600">Run inference on deployed models and manage your AI workloads</p>
        </div>
        <div className="flex items-center gap-3">
          <Badge variant="success" className="flex items-center gap-1">
            <Activity className="h-3 w-3" />
            Neural Network Active
          </Badge>
        </div>
      </header>

      {/* Statistics Dashboard */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Models</p>
              <p className="text-2xl font-bold text-gray-900">{models.length}</p>
            </div>
            <Brain className="h-8 w-8 text-blue-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Jobs Today</p>
              <p className="text-2xl font-bold text-green-600">
                {jobs.filter(j => new Date(j.startedAt).toDateString() === new Date().toDateString()).length}
              </p>
            </div>
            <Activity className="h-8 w-8 text-green-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Success Rate</p>
              <p className="text-2xl font-bold text-purple-600">
                {((jobs.filter(j => j.status === 'completed').length / jobs.length) * 100).toFixed(1)}%
              </p>
            </div>
            <Target className="h-8 w-8 text-purple-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Avg Latency</p>
              <p className="text-2xl font-bold text-orange-600">
                {formatDuration(models.reduce((sum, m) => sum + m.latency, 0) / models.length)}
              </p>
            </div>
            <Timer className="h-8 w-8 text-orange-600" />
          </div>
        </Card>
      </div>

      {/* Main Content */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Inference Panel */}
        <div className="lg:col-span-2 space-y-6">
          {/* Model Selection */}
          <Card>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-semibold">Select Model</h3>
                <Button 
                  variant="outline" 
                  size="sm"
                  onClick={() => setShowModelDetails(true)}
                >
                  <Eye className="h-4 w-4 mr-1" />
                  View All Models
                </Button>
              </div>
              
              {selectedModel ? (
                <div className="border rounded-lg p-4 bg-blue-50">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <span className="text-2xl">{getTypeIcon(selectedModel.type)}</span>
                      <div>
                        <h4 className="font-semibold">{selectedModel.name}</h4>
                        <p className="text-sm text-gray-600">{selectedModel.description}</p>
                        <div className="flex items-center gap-4 mt-1">
                          <span className="text-xs text-gray-500">v{selectedModel.version}</span>
                          <span className="text-xs text-gray-500">{selectedModel.parameters}M params</span>
                          <span className="text-xs text-gray-500">{formatDuration(selectedModel.latency)} latency</span>
                          <span className="text-xs text-gray-500">${selectedModel.cost} per inference</span>
                        </div>
                      </div>
                    </div>
                    <Button 
                      variant="outline" 
                      size="sm"
                      onClick={() => setSelectedModel(null)}
                    >
                      Change
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="text-center py-8 text-gray-500">
                  <Brain className="h-12 w-12 mx-auto mb-2 text-gray-400" />
                  <p>Select a model to start inference</p>
                </div>
              )}
            </div>
          </Card>

          {/* Input Section */}
          <Card>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-semibold">Input</h3>
                <div className="flex items-center gap-2">
                  <Label>Mode:</Label>
                  <Select value={inferenceMode} onValueChange={(v) => setInferenceMode(v as InferenceMode)}>
                    <SelectTrigger className="w-32">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="single">Single</SelectItem>
                      <SelectItem value="batch">Batch</SelectItem>
                      <SelectItem value="streaming">Streaming</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              {selectedModel?.type === 'image' ? (
                <div className="border-2 border-dashed border-gray-300 rounded-lg p-6 text-center">
                  <Upload className="h-8 w-8 text-gray-400 mx-auto mb-2" />
                  <p className="text-sm text-gray-600 mb-2">Upload an image for analysis</p>
                  <Input 
                    type="file" 
                    accept="image/*"
                    onChange={e => setInputFile(e.target.files?.[0] || null)}
                    className="max-w-xs mx-auto"
                  />
                  {inputFile && (
                    <p className="text-sm text-green-600 mt-2">
                      Selected: {inputFile.name}
                    </p>
                  )}
                </div>
              ) : (
                <div>
                  <Label>Input Text</Label>
                  <Textarea
                    placeholder="Enter your input here..."
                    value={inputText}
                    onChange={e => setInputText(e.target.value)}
                    rows={6}
                    className="mt-1"
                  />
                </div>
              )}

              <Button 
                onClick={handleInference}
                disabled={!selectedModel || (!inputText && !inputFile) || isProcessing}
                className="w-full"
                size="lg"
              >
                {isProcessing ? (
                  <>
                    <Activity className="h-4 w-4 mr-2 animate-spin" />
                    Processing... {currentJob?.progress}%
                  </>
                ) : (
                  <>
                    <Play className="h-4 w-4 mr-2" />
                    Run Inference
                  </>
                )}
              </Button>
            </div>
          </Card>

          {/* Current Job Progress */}
          {currentJob && (
            <Card>
              <div className="space-y-4">
                <h3 className="text-lg font-semibold">Processing</h3>
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span>Model: {currentJob.modelName}</span>
                    <span>{currentJob.progress}%</span>
                  </div>
                  <div className="w-full bg-gray-200 rounded-full h-2">
                    <div 
                      className="bg-blue-600 h-2 rounded-full transition-all duration-300"
                      style={{ width: `${currentJob.progress}%` }}
                    />
                  </div>
                </div>
              </div>
            </Card>
          )}
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Templates */}
          <Card>
            <div className="space-y-4">
              <h3 className="text-lg font-semibold">Quick Templates</h3>
              <div className="space-y-2">
                {templates.map(template => (
                  <div 
                    key={template.id}
                    className="border rounded-lg p-3 hover:bg-gray-50 cursor-pointer"
                    onClick={() => applyTemplate(template)}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-lg">{getTypeIcon(template.modelType)}</span>
                      <h4 className="font-medium text-sm">{template.name}</h4>
                    </div>
                    <p className="text-xs text-gray-600">{template.description}</p>
                    <div className="flex flex-wrap gap-1 mt-2">
                      {template.tags.slice(0, 2).map(tag => (
                        <Badge key={tag} variant="secondary" className="text-xs">
                          {tag}
                        </Badge>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </Card>

          {/* Recent Jobs */}
          <Card>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-semibold">Recent Jobs</h3>
                <Button 
                  variant="outline" 
                  size="sm"
                  onClick={() => setActiveTab('history')}
                >
                  View All
                </Button>
              </div>
              <div className="space-y-3">
                {jobs.slice(0, 3).map(job => (
                  <div 
                    key={job.id}
                    className="border rounded-lg p-3 hover:bg-gray-50 cursor-pointer"
                    onClick={() => {
                      setSelectedJob(job);
                      setShowJobDetails(true);
                    }}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="font-medium text-sm">{job.modelName}</span>
                      <Badge className={getStatusColor(job.status)}>
                        {job.status}
                      </Badge>
                    </div>
                    <p className="text-xs text-gray-600 truncate">{job.input}</p>
                    <p className="text-xs text-gray-500 mt-1">
                      {formatTimeAgo(job.startedAt)}
                      {job.duration && ` • ${formatDuration(job.duration)}`}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          </Card>
        </div>
      </div>

      {/* Model Details Modal */}
      {showModelDetails && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-4xl max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-xl font-semibold">Available Models</h2>
              <Button variant="outline" onClick={() => setShowModelDetails(false)}>
                ×
              </Button>
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {models.map(model => (
                <div 
                  key={model.id}
                  className="border rounded-lg p-4 hover:shadow-md cursor-pointer"
                  onClick={() => {
                    setSelectedModel(model);
                    setShowModelDetails(false);
                  }}
                >
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-2">
                      <span className="text-2xl">{getTypeIcon(model.type)}</span>
                      <div>
                        <h3 className="font-semibold">{model.name}</h3>
                        <p className="text-sm text-gray-600">{model.type} • v{model.version}</p>
                      </div>
                    </div>
                    <Badge className={getStatusColor(model.status as any)}>
                      {model.status}
                    </Badge>
                  </div>
                  
                  <p className="text-sm text-gray-600 mb-3">{model.description}</p>
                  
                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div className="flex justify-between">
                      <span className="text-gray-500">Parameters:</span>
                      <span>{model.parameters}M</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-500">Size:</span>
                      <span>{(model.size / 1024).toFixed(1)}GB</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-500">Accuracy:</span>
                      <span>{model.accuracy}%</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-500">Latency:</span>
                      <span>{formatDuration(model.latency)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-500">Cost:</span>
                      <span>${model.cost}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-500">Usage:</span>
                      <span>{model.usageCount.toLocaleString()}</span>
                    </div>
                  </div>
                  
                  <div className="flex flex-wrap gap-1 mt-3">
                    {model.tags.slice(0, 3).map(tag => (
                      <Badge key={tag} variant="secondary" className="text-xs">
                        {tag}
                      </Badge>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Job Details Modal */}
      {showJobDetails && selectedJob && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-xl font-semibold">Job Details</h2>
              <Button variant="outline" onClick={() => setShowJobDetails(false)}>
                ×
              </Button>
            </div>
            
            <div className="space-y-4">
              <div>
                <h3 className="font-medium mb-2">Job Information</h3>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-gray-600">Job ID:</span>
                    <span className="font-mono text-xs">{selectedJob.id}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-600">Model:</span>
                    <span>{selectedJob.modelName}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-600">Status:</span>
                    <Badge className={getStatusColor(selectedJob.status)}>
                      {selectedJob.status}
                    </Badge>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-600">Mode:</span>
                    <span>{selectedJob.mode}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-600">Started:</span>
                    <span>{formatTimeAgo(selectedJob.startedAt)}</span>
                  </div>
                  {selectedJob.completedAt && (
                    <div className="flex justify-between">
                      <span className="text-gray-600">Completed:</span>
                      <span>{formatTimeAgo(selectedJob.completedAt)}</span>
                    </div>
                  )}
                  {selectedJob.duration && (
                    <div className="flex justify-between">
                      <span className="text-gray-600">Duration:</span>
                      <span>{formatDuration(selectedJob.duration)}</span>
                    </div>
                  )}
                  {selectedJob.cost && (
                    <div className="flex justify-between">
                      <span className="text-gray-600">Cost:</span>
                      <span>${selectedJob.cost}</span>
                    </div>
                  )}
                </div>
              </div>
              
              <div>
                <h3 className="font-medium mb-2">Input</h3>
                <div className="bg-gray-50 rounded p-3 text-sm">
                  {selectedJob.input}
                </div>
              </div>
              
              {selectedJob.output && (
                <div>
                  <h3 className="font-medium mb-2">Output</h3>
                  <div className="bg-green-50 rounded p-3 text-sm">
                    {selectedJob.output}
                  </div>
                </div>
              )}
              
              {selectedJob.error && (
                <div>
                  <h3 className="font-medium mb-2">Error</h3>
                  <div className="bg-red-50 rounded p-3 text-sm text-red-800">
                    {selectedJob.error}
                  </div>
                </div>
              )}
            </div>
            
            <div className="flex justify-end gap-3 mt-6">
              <Button variant="outline" onClick={() => setShowJobDetails(false)}>
                Close
              </Button>
              {selectedJob.output && (
                <Button>
                  <Copy className="h-4 w-4 mr-2" />
                  Copy Output
                </Button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}