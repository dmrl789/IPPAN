import * as React from "react";
import { useEffect, useState } from "react";
import { Button, Card, Input, Label, Badge, Switch, Select, SelectTrigger, SelectValue, SelectContent, SelectItem, Sheet, SheetContent, SheetHeader, SheetTitle, SheetFooter, Textarea } from "../components/UI";
import {
  Upload, FileSignature, Search, BarChart3, Database, Link as LinkIcon, 
  Play, Pause, Download, Star, Eye, EyeOff, Settings, Zap, 
  TrendingUp, Users, Clock, CheckCircle, AlertCircle, 
  Brain, Cpu, HardDrive, Network, Activity, Filter,
  Plus, Edit, Trash2, Copy, ExternalLink, Info
} from "lucide-react";

// =================== Types ===================
type Arch = "Transformer" | "CNN" | "RNN/LSTM" | "ResNet" | "BERT" | "GPT" | "CLIP" | "Other";
type Modality = "text" | "image" | "audio" | "multimodal" | "tabular" | "video" | "code";
type Visibility = "private" | "public" | "commercial";
type Status = "active" | "inactive" | "training" | "deployed" | "archived";
type License = "MIT" | "Apache-2.0" | "GPL-3.0" | "BSD-3-Clause" | "Custom" | "Commercial";

type Model = {
  id: string;
  owner: string;
  name: string;
  description?: string;
  arch: Arch;
  modality: Modality;
  license: License;
  visibility: Visibility;
  status: Status;
  paramsM: number;
  totalSizeBytes: number;
  tags: string[];
  createdAt: string;
  updatedAt: string;
  latestVersion: string;
  downloadCount: number;
  rating: number;
  inferenceCount: number;
  lastInference?: string;
  benchmarks?: Benchmark[];
  deployment?: Deployment;
};

type Benchmark = {
  name: string;
  score: number;
  metric: string;
  dataset: string;
};

type Deployment = {
  status: "deployed" | "pending" | "failed";
  endpoint?: string;
  replicas: number;
  lastDeployed?: string;
};

type InferenceRequest = {
  id: string;
  modelId: string;
  input: string;
  output?: string;
  status: "pending" | "processing" | "completed" | "failed";
  timestamp: string;
  duration?: number;
  cost?: number;
};

// =================== Local storage ===================
const LS_ADDR = "ippan.wallet.address";
function loadAddress(){ return localStorage.getItem(LS_ADDR) || null; }

// =================== Mock APIs ===================
function nowMinus(h:number){ return new Date(Date.now()-h*3600e3).toISOString(); }

async function apiListModels(): Promise<Model[]> {
  const models: Model[] = [
    {
      id: "model_001",
      name: "GPT-4 Alternative",
      description: "Large language model for text generation and understanding",
      owner: loadAddress() || "iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X",
      arch: "GPT",
      modality: "text",
      license: "Apache-2.0",
      visibility: "public",
      status: "deployed",
      paramsM: 7000,
      totalSizeBytes: 28*1024**3,
      tags: ["gpt", "causal-lm", "nlp"],
      createdAt: nowMinus(120),
      updatedAt: nowMinus(2),
      latestVersion: "v1.1",
      downloadCount: 15420,
      rating: 4.8,
      inferenceCount: 125000,
      lastInference: nowMinus(0.5),
      benchmarks: [
        { name: "MMLU", score: 85.2, metric: "accuracy", dataset: "MMLU" },
        { name: "HellaSwag", score: 92.1, metric: "accuracy", dataset: "HellaSwag" }
      ],
      deployment: {
        status: "deployed",
        endpoint: "https://api.ippan.network/inference/model_001",
        replicas: 5,
        lastDeployed: nowMinus(24)
      }
    },
    {
      id: "model_002",
      name: "Vision Transformer",
      description: "State-of-the-art image classification model",
      owner: "i1B2zP2eP6QGefi3DMPTfTL6SLmv8DivfNb",
      arch: "Transformer",
      modality: "image",
      license: "MIT",
      visibility: "public",
      status: "active",
      paramsM: 86,
      totalSizeBytes: 340*1024**2,
      tags: ["vision", "classification", "transformer"],
      createdAt: nowMinus(72),
      updatedAt: nowMinus(12),
      latestVersion: "v2.0",
      downloadCount: 8930,
      rating: 4.6,
      inferenceCount: 45000,
      lastInference: nowMinus(1.5),
      benchmarks: [
        { name: "ImageNet", score: 94.3, metric: "top-1 accuracy", dataset: "ImageNet" },
        { name: "CIFAR-100", score: 89.7, metric: "accuracy", dataset: "CIFAR-100" }
      ]
    },
    {
      id: "model_003",
      name: "CodeGen-16B",
      description: "Code generation model for multiple programming languages",
      owner: "i1C3zP3eP7QGefi4DMPTfTL7SLmv9DivfNc",
      arch: "GPT",
      modality: "code",
      license: "Apache-2.0",
      visibility: "commercial",
      status: "training",
      paramsM: 16000,
      totalSizeBytes: 64*1024**3,
      tags: ["code", "generation", "programming"],
      createdAt: nowMinus(48),
      updatedAt: nowMinus(0.5),
      latestVersion: "v0.9",
      downloadCount: 2340,
      rating: 4.4,
      inferenceCount: 12000,
      lastInference: nowMinus(3),
      benchmarks: [
        { name: "HumanEval", score: 78.5, metric: "pass@1", dataset: "HumanEval" }
      ]
    },
    {
      id: "model_004",
      name: "Whisper-Large",
      description: "Automatic speech recognition and translation",
      owner: loadAddress() || "iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X",
      arch: "Transformer",
      modality: "audio",
      license: "MIT",
      visibility: "public",
      status: "deployed",
      paramsM: 1550,
      totalSizeBytes: 6.2*1024**3,
      tags: ["speech", "recognition", "translation"],
      createdAt: nowMinus(96),
      updatedAt: nowMinus(6),
      latestVersion: "v1.0",
      downloadCount: 18750,
      rating: 4.9,
      inferenceCount: 89000,
      lastInference: nowMinus(0.2),
      deployment: {
        status: "deployed",
        endpoint: "https://api.ippan.network/inference/model_004",
        replicas: 3,
        lastDeployed: nowMinus(48)
      }
    }
  ];
  return new Promise(r=>setTimeout(()=>r(models),160));
}

async function apiRegisterModel(m: Omit<Model,"createdAt"|"updatedAt"|"latestVersion"|"downloadCount"|"rating"|"inferenceCount">): Promise<{ok:boolean; model:Model}> {
  const model: Model = {
    ...m,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    latestVersion: "v1.0",
    downloadCount: 0,
    rating: 0,
    inferenceCount: 0
  };
  return new Promise(r=>setTimeout(()=>r({ ok:true, model }),300));
}

async function apiDeployModel(modelId: string): Promise<{ok:boolean; endpoint?: string}> {
  return new Promise(r=>setTimeout(()=>r({ 
    ok: true, 
    endpoint: `https://api.ippan.network/inference/${modelId}` 
  }),2000));
}

async function apiRunInference(modelId: string, input: string): Promise<{ok:boolean; output?: string; cost?: number}> {
  return new Promise(r=>setTimeout(()=>r({ 
    ok: true, 
    output: `Generated response for: ${input}`,
    cost: 0.001
  }),1500));
}

// =================== Utils ===================
function bytesToSize(n: number) {
  if (n < 1024) return `${n} B`;
  if (n < 1024**2) return `${(n/1024).toFixed(1)} KB`;
  if (n < 1024**3) return `${(n/1024**2).toFixed(1)} MB`;
  return `${(n/1024**3).toFixed(2)} GB`;
}

function formatTimeAgo(timestamp: string) {
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

function getStatusColor(status: Status) {
  switch (status) {
    case 'deployed': return 'bg-green-100 text-green-800';
    case 'active': return 'bg-blue-100 text-blue-800';
    case 'training': return 'bg-yellow-100 text-yellow-800';
    case 'inactive': return 'bg-gray-100 text-gray-800';
    case 'archived': return 'bg-red-100 text-red-800';
    default: return 'bg-gray-100 text-gray-800';
  }
}

function getModalityIcon(modality: Modality) {
  switch (modality) {
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

// =================== Component ===================
export default function NeuralModels() {
  const [address] = useState<string | null>(loadAddress());

  // Data
  const [models, setModels] = useState<Model[]>([]);
  const [selectedModel, setSelectedModel] = useState<Model | null>(null);
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');

  // Register form
  const [owner, setOwner] = useState<string>(address || "");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [arch, setArch] = useState<Arch>("Transformer");
  const [modality, setModality] = useState<Modality>("text");
  const [license, setLicense] = useState<License>("Apache-2.0");
  const [visibility, setVisibility] = useState<Visibility>("public");
  const [status, setStatus] = useState<Status>("active");
  const [paramsM, setParamsM] = useState<number>(0);
  const [tags, setTags] = useState<string>("");
  const [weightsCid, setWeightsCid] = useState<string>("");
  const [weightsHashHex, setWeightsHashHex] = useState<string>("");
  const [modelIdHex, setModelIdHex] = useState<string>("");

  // Catalog filters
  const [q, setQ] = useState("");
  const [fArch, setFArch] = useState<"all"|Arch>("all");
  const [fMod, setFMod] = useState<"all"|Modality>("all");
  const [fVis, setFVis] = useState<"all"|Visibility>("all");
  const [fStatus, setFStatus] = useState<"all"|Status>("all");

  // Inference
  const [inferenceInput, setInferenceInput] = useState("");
  const [inferenceOutput, setInferenceOutput] = useState("");
  const [isRunningInference, setIsRunningInference] = useState(false);

  // Deployment
  const [isDeploying, setIsDeploying] = useState<string | null>(null);

  // Modals
  const [showRegisterModal, setShowRegisterModal] = useState(false);
  const [showInferenceModal, setShowInferenceModal] = useState(false);
  const [showModelDetails, setShowModelDetails] = useState(false);

  // Init
  useEffect(()=>{
    apiListModels().then(setModels);
  },[]);

  // ============ Helpers ============
  function filteredModels(){
    let r = models;
    if (q) {
      const qq = q.toLowerCase();
      r = r.filter(m => 
        m.name.toLowerCase().includes(qq) || 
        m.id.toLowerCase().includes(qq) || 
        m.description?.toLowerCase().includes(qq) ||
        m.tags.join(",").toLowerCase().includes(qq)
      );
    }
    if (fArch!=="all") r = r.filter(m=>m.arch===fArch);
    if (fMod!=="all") r = r.filter(m=>m.modality===fMod);
    if (fVis!=="all") r = r.filter(m=>m.visibility===fVis);
    if (fStatus!=="all") r = r.filter(m=>m.status===fStatus);
    return r;
  }

  // ============ Actions ============
  async function registerModel(){
    if (!owner || !name || !weightsHashHex || !modelIdHex) {
      alert("Owner, Name, and Hashes are required.");
      return;
    }
    const m = {
      id: modelIdHex,
      owner,
      name,
      description,
      arch, modality, license, visibility, status,
      paramsM,
      totalSizeBytes: 0,
      tags: tags.split(",").map(s=>s.trim()).filter(Boolean)
    };
    const r = await apiRegisterModel(m);
    if (r.ok) {
      setModels(prev=>[r.model, ...prev]);
      alert("✅ Model registered");
      setShowRegisterModal(false);
      // Reset form
      setName(""); setDescription(""); setWeightsCid(""); 
      setWeightsHashHex(""); setModelIdHex(""); setTags("");
    }
  }

  async function deployModel(modelId: string) {
    setIsDeploying(modelId);
    const result = await apiDeployModel(modelId);
    if (result.ok) {
      setModels(prev => prev.map(m => 
        m.id === modelId 
          ? { 
              ...m, 
              status: 'deployed' as Status,
              deployment: {
                status: 'deployed' as const,
                endpoint: result.endpoint,
                replicas: 1,
                lastDeployed: new Date().toISOString()
              }
            }
          : m
      ));
      alert("✅ Model deployed successfully!");
    } else {
      alert("❌ Deployment failed");
    }
    setIsDeploying(null);
  }

  async function runInference() {
    if (!selectedModel || !inferenceInput) return;
    
    setIsRunningInference(true);
    const result = await apiRunInference(selectedModel.id, inferenceInput);
    if (result.ok) {
      setInferenceOutput(result.output || "");
      // Update model inference count
      setModels(prev => prev.map(m => 
        m.id === selectedModel.id 
          ? { ...m, inferenceCount: m.inferenceCount + 1, lastInference: new Date().toISOString() }
          : m
      ));
    } else {
      setInferenceOutput("❌ Inference failed");
    }
    setIsRunningInference(false);
  }

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <header className="flex justify-between items-start">
        <div className="space-y-1">
          <h1 className="text-3xl font-bold text-gray-900">Neural Models</h1>
          <p className="text-gray-600">Register, deploy, and manage AI models on the IPPAN Neural Network</p>
        </div>
        <div className="flex items-center gap-3">
          <Button 
            onClick={() => setViewMode(viewMode === 'grid' ? 'list' : 'grid')}
            variant="outline"
            size="sm"
          >
            {viewMode === 'grid' ? 'List View' : 'Grid View'}
          </Button>
          <Button onClick={() => setShowRegisterModal(true)}>
            <Plus className="h-4 w-4 mr-2" />
            Register Model
          </Button>
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
              <p className="text-sm font-medium text-gray-600">Deployed</p>
              <p className="text-2xl font-bold text-green-600">{models.filter(m => m.status === 'deployed').length}</p>
            </div>
            <Zap className="h-8 w-8 text-green-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Size</p>
              <p className="text-2xl font-bold text-gray-900">{bytesToSize(models.reduce((sum, m) => sum + m.totalSizeBytes, 0))}</p>
            </div>
            <HardDrive className="h-8 w-8 text-purple-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Inferences</p>
              <p className="text-2xl font-bold text-orange-600">{models.reduce((sum, m) => sum + m.inferenceCount, 0).toLocaleString()}</p>
            </div>
            <Activity className="h-8 w-8 text-orange-600" />
          </div>
        </Card>
      </div>

      {/* Filters and Search */}
      <Card>
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex items-center gap-2">
            <Search className="h-4 w-4 text-gray-400" />
            <Input 
              placeholder="Search models..." 
              value={q} 
              onChange={e => setQ(e.target.value)}
              className="w-64"
            />
          </div>
          <Select value={fArch} onValueChange={(v) => setFArch(v as any)}>
            <SelectTrigger className="w-40">
              <SelectValue placeholder="Architecture" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Architectures</SelectItem>
              <SelectItem value="Transformer">Transformer</SelectItem>
              <SelectItem value="GPT">GPT</SelectItem>
              <SelectItem value="CNN">CNN</SelectItem>
              <SelectItem value="ResNet">ResNet</SelectItem>
              <SelectItem value="BERT">BERT</SelectItem>
              <SelectItem value="CLIP">CLIP</SelectItem>
              <SelectItem value="RNN/LSTM">RNN/LSTM</SelectItem>
              <SelectItem value="Other">Other</SelectItem>
            </SelectContent>
          </Select>
          <Select value={fMod} onValueChange={(v) => setFMod(v as any)}>
            <SelectTrigger className="w-40">
              <SelectValue placeholder="Modality" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Modalities</SelectItem>
              <SelectItem value="text">Text</SelectItem>
              <SelectItem value="image">Image</SelectItem>
              <SelectItem value="audio">Audio</SelectItem>
              <SelectItem value="video">Video</SelectItem>
              <SelectItem value="code">Code</SelectItem>
              <SelectItem value="multimodal">Multimodal</SelectItem>
              <SelectItem value="tabular">Tabular</SelectItem>
            </SelectContent>
          </Select>
          <Select value={fStatus} onValueChange={(v) => setFStatus(v as any)}>
            <SelectTrigger className="w-40">
              <SelectValue placeholder="Status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Status</SelectItem>
              <SelectItem value="deployed">Deployed</SelectItem>
              <SelectItem value="active">Active</SelectItem>
              <SelectItem value="training">Training</SelectItem>
              <SelectItem value="inactive">Inactive</SelectItem>
              <SelectItem value="archived">Archived</SelectItem>
            </SelectContent>
          </Select>
          <Select value={fVis} onValueChange={(v) => setFVis(v as any)}>
            <SelectTrigger className="w-40">
              <SelectValue placeholder="Visibility" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Visibility</SelectItem>
              <SelectItem value="public">Public</SelectItem>
              <SelectItem value="private">Private</SelectItem>
              <SelectItem value="commercial">Commercial</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </Card>

      {/* Models Grid/List */}
      <Card>
        <div className="space-y-4">
          {viewMode === 'grid' ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              {filteredModels().map(model => (
                <div key={model.id} className="border rounded-lg p-4 hover:shadow-md transition-shadow">
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-2">
                      <span className="text-2xl">{getModalityIcon(model.modality)}</span>
                      <div>
                        <h3 className="font-semibold text-gray-900">{model.name}</h3>
                        <p className="text-sm text-gray-600">{model.arch}</p>
                      </div>
                    </div>
                    <Badge className={getStatusColor(model.status)}>
                      {model.status}
                    </Badge>
                  </div>
                  
                  {model.description && (
                    <p className="text-sm text-gray-600 mb-3 line-clamp-2">{model.description}</p>
                  )}
                  
                  <div className="space-y-2 mb-4">
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Parameters:</span>
                      <span className="font-medium">{model.paramsM}M</span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Size:</span>
                      <span className="font-medium">{bytesToSize(model.totalSizeBytes)}</span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Downloads:</span>
                      <span className="font-medium">{model.downloadCount.toLocaleString()}</span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-gray-500">Rating:</span>
                      <div className="flex items-center gap-1">
                        <Star className="h-3 w-3 fill-yellow-400 text-yellow-400" />
                        <span className="font-medium">{model.rating}</span>
                      </div>
                    </div>
                  </div>
                  
                  <div className="flex flex-wrap gap-1 mb-4">
                    {model.tags.slice(0, 3).map(tag => (
                      <Badge key={tag} variant="secondary" className="text-xs">
                        {tag}
                      </Badge>
                    ))}
                    {model.tags.length > 3 && (
                      <Badge variant="secondary" className="text-xs">
                        +{model.tags.length - 3}
                      </Badge>
                    )}
                  </div>
                  
                  <div className="flex gap-2">
                    <Button 
                      size="sm" 
                      variant="outline"
                      onClick={() => {
                        setSelectedModel(model);
                        setShowModelDetails(true);
                      }}
                    >
                      <Eye className="h-3 w-3 mr-1" />
                      Details
                    </Button>
                    {model.status !== 'deployed' && (
                      <Button 
                        size="sm"
                        onClick={() => deployModel(model.id)}
                        disabled={isDeploying === model.id}
                      >
                        {isDeploying === model.id ? (
                          <Activity className="h-3 w-3 mr-1 animate-spin" />
                        ) : (
                          <Zap className="h-3 w-3 mr-1" />
                        )}
                        Deploy
                      </Button>
                    )}
                    {model.status === 'deployed' && (
                      <Button 
                        size="sm"
                        onClick={() => {
                          setSelectedModel(model);
                          setShowInferenceModal(true);
                        }}
                      >
                        <Play className="h-3 w-3 mr-1" />
                        Test
                      </Button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="space-y-3">
              {filteredModels().map(model => (
                <div key={model.id} className="border rounded-lg p-4 hover:bg-gray-50">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-4">
                      <span className="text-2xl">{getModalityIcon(model.modality)}</span>
                      <div>
                        <div className="flex items-center gap-2">
                          <h3 className="font-semibold text-gray-900">{model.name}</h3>
                          <Badge className={getStatusColor(model.status)}>
                            {model.status}
                          </Badge>
                        </div>
                        <p className="text-sm text-gray-600">{model.arch} • {model.modality} • {model.license}</p>
                        <p className="text-xs text-gray-500">
                          {model.paramsM}M params • {bytesToSize(model.totalSizeBytes)} • 
                          {model.downloadCount.toLocaleString()} downloads • 
                          <Star className="h-3 w-3 inline fill-yellow-400 text-yellow-400 mx-1" />
                          {model.rating}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Button 
                        size="sm" 
                        variant="outline"
                        onClick={() => {
                          setSelectedModel(model);
                          setShowModelDetails(true);
                        }}
                      >
                        <Eye className="h-3 w-3 mr-1" />
                        Details
                      </Button>
                      {model.status !== 'deployed' && (
                        <Button 
                          size="sm"
                          onClick={() => deployModel(model.id)}
                          disabled={isDeploying === model.id}
                        >
                          {isDeploying === model.id ? (
                            <Activity className="h-3 w-3 mr-1 animate-spin" />
                          ) : (
                            <Zap className="h-3 w-3 mr-1" />
                          )}
                          Deploy
                        </Button>
                      )}
                      {model.status === 'deployed' && (
                        <Button 
                          size="sm"
                          onClick={() => {
                            setSelectedModel(model);
                            setShowInferenceModal(true);
                          }}
                        >
                          <Play className="h-3 w-3 mr-1" />
                          Test
                        </Button>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
          
          {filteredModels().length === 0 && (
            <div className="text-center py-12">
              <Brain className="h-12 w-12 text-gray-400 mx-auto mb-4" />
              <h3 className="text-lg font-medium text-gray-900 mb-2">No models found</h3>
              <p className="text-gray-600 mb-4">Try adjusting your search criteria or register a new model.</p>
              <Button onClick={() => setShowRegisterModal(true)}>
                <Plus className="h-4 w-4 mr-2" />
                Register Model
              </Button>
            </div>
          )}
        </div>
      </Card>

      {/* Register Model Modal */}
      {showRegisterModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-xl font-semibold">Register New Model</h2>
              <Button variant="outline" onClick={() => setShowRegisterModal(false)}>
                ×
              </Button>
            </div>
            
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Model Name</Label>
                  <Input value={name} onChange={e => setName(e.target.value)} placeholder="e.g., AlphaTransformer-7B" />
                </div>
                <div>
                  <Label>Owner Address</Label>
                  <Input value={owner} onChange={e => setOwner(e.target.value)} placeholder="i1..." />
                </div>
              </div>
              
              <div>
                <Label>Description</Label>
                <Textarea value={description} onChange={e => setDescription(e.target.value)} placeholder="Describe your model..." />
              </div>
              
              <div className="grid grid-cols-3 gap-4">
                <div>
                  <Label>Architecture</Label>
                  <Select value={arch} onValueChange={(v) => setArch(v as Arch)}>
                    <SelectTrigger><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="Transformer">Transformer</SelectItem>
                      <SelectItem value="GPT">GPT</SelectItem>
                      <SelectItem value="CNN">CNN</SelectItem>
                      <SelectItem value="ResNet">ResNet</SelectItem>
                      <SelectItem value="BERT">BERT</SelectItem>
                      <SelectItem value="CLIP">CLIP</SelectItem>
                      <SelectItem value="RNN/LSTM">RNN/LSTM</SelectItem>
                      <SelectItem value="Other">Other</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label>Modality</Label>
                  <Select value={modality} onValueChange={(v) => setModality(v as Modality)}>
                    <SelectTrigger><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="text">Text</SelectItem>
                      <SelectItem value="image">Image</SelectItem>
                      <SelectItem value="audio">Audio</SelectItem>
                      <SelectItem value="video">Video</SelectItem>
                      <SelectItem value="code">Code</SelectItem>
                      <SelectItem value="multimodal">Multimodal</SelectItem>
                      <SelectItem value="tabular">Tabular</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label>License</Label>
                  <Select value={license} onValueChange={(v) => setLicense(v as License)}>
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
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label>Visibility</Label>
                  <Select value={visibility} onValueChange={(v) => setVisibility(v as Visibility)}>
                    <SelectTrigger><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="public">Public</SelectItem>
                      <SelectItem value="private">Private</SelectItem>
                      <SelectItem value="commercial">Commercial</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label>Parameters (Millions)</Label>
                  <Input type="number" value={paramsM || ""} onChange={e => setParamsM(Number(e.target.value) || 0)} placeholder="7000" />
                </div>
              </div>
              
              <div>
                <Label>Tags (comma separated)</Label>
                <Input value={tags} onChange={e => setTags(e.target.value)} placeholder="gpt,causal-lm,7b" />
              </div>
              
              <div className="space-y-3">
                <div>
                  <Label>Weights Hash (32-byte hex BLAKE3)</Label>
                  <Input value={weightsHashHex} onChange={e => setWeightsHashHex(e.target.value)} placeholder="0x...64 hex" />
                </div>
                <div>
                  <Label>Model ID (blake3(weights_hash))</Label>
                  <Input value={modelIdHex} onChange={e => setModelIdHex(e.target.value)} placeholder="0x...64" />
                </div>
                <div>
                  <Label>Weights CID (optional)</Label>
                  <Input value={weightsCid} onChange={e => setWeightsCid(e.target.value)} placeholder="bafy..." />
                </div>
              </div>
            </div>
            
            <div className="flex justify-end gap-3 mt-6">
              <Button variant="outline" onClick={() => setShowRegisterModal(false)}>
                Cancel
              </Button>
              <Button onClick={registerModel}>
                <FileSignature className="h-4 w-4 mr-2" />
                Register Model
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Model Details Modal */}
      {showModelDetails && selectedModel && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-4xl max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-6">
              <div className="flex items-center gap-3">
                <span className="text-3xl">{getModalityIcon(selectedModel.modality)}</span>
                <div>
                  <h2 className="text-xl font-semibold">{selectedModel.name}</h2>
                  <p className="text-gray-600">{selectedModel.arch} • {selectedModel.modality}</p>
                </div>
              </div>
              <Button variant="outline" onClick={() => setShowModelDetails(false)}>
                ×
              </Button>
            </div>
            
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="space-y-4">
                <div>
                  <h3 className="font-medium mb-2">Model Information</h3>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-600">Model ID:</span>
                      <span className="font-mono text-xs">{selectedModel.id}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Owner:</span>
                      <span className="font-mono text-xs">{selectedModel.owner}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Version:</span>
                      <span>{selectedModel.latestVersion}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">License:</span>
                      <span>{selectedModel.license}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Visibility:</span>
                      <Badge className={getStatusColor(selectedModel.visibility as any)}>
                        {selectedModel.visibility}
                      </Badge>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Status:</span>
                      <Badge className={getStatusColor(selectedModel.status)}>
                        {selectedModel.status}
                      </Badge>
                    </div>
                  </div>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Performance Metrics</h3>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-600">Parameters:</span>
                      <span>{selectedModel.paramsM}M</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Size:</span>
                      <span>{bytesToSize(selectedModel.totalSizeBytes)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Downloads:</span>
                      <span>{selectedModel.downloadCount.toLocaleString()}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Inferences:</span>
                      <span>{selectedModel.inferenceCount.toLocaleString()}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Rating:</span>
                      <div className="flex items-center gap-1">
                        <Star className="h-3 w-3 fill-yellow-400 text-yellow-400" />
                        <span>{selectedModel.rating}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              
              <div className="space-y-4">
                {selectedModel.description && (
                  <div>
                    <h3 className="font-medium mb-2">Description</h3>
                    <p className="text-sm text-gray-600">{selectedModel.description}</p>
                  </div>
                )}
                
                <div>
                  <h3 className="font-medium mb-2">Tags</h3>
                  <div className="flex flex-wrap gap-1">
                    {selectedModel.tags.map(tag => (
                      <Badge key={tag} variant="secondary">
                        {tag}
                      </Badge>
                    ))}
                  </div>
                </div>
                
                {selectedModel.benchmarks && selectedModel.benchmarks.length > 0 && (
                  <div>
                    <h3 className="font-medium mb-2">Benchmarks</h3>
                    <div className="space-y-2">
                      {selectedModel.benchmarks.map((benchmark, index) => (
                        <div key={index} className="flex justify-between text-sm">
                          <span className="text-gray-600">{benchmark.name}:</span>
                          <span>{benchmark.score} ({benchmark.metric})</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
                
                {selectedModel.deployment && (
                  <div>
                    <h3 className="font-medium mb-2">Deployment</h3>
                    <div className="space-y-2 text-sm">
                      <div className="flex justify-between">
                        <span className="text-gray-600">Status:</span>
                        <Badge className={getStatusColor(selectedModel.deployment.status as any)}>
                          {selectedModel.deployment.status}
                        </Badge>
                      </div>
                      {selectedModel.deployment.endpoint && (
                        <div className="flex justify-between">
                          <span className="text-gray-600">Endpoint:</span>
                          <span className="font-mono text-xs">{selectedModel.deployment.endpoint}</span>
                        </div>
                      )}
                      <div className="flex justify-between">
                        <span className="text-gray-600">Replicas:</span>
                        <span>{selectedModel.deployment.replicas}</span>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </div>
            
            <div className="flex justify-end gap-3 mt-6">
              <Button variant="outline" onClick={() => setShowModelDetails(false)}>
                Close
              </Button>
              {selectedModel.status !== 'deployed' && (
                <Button onClick={() => deployModel(selectedModel.id)}>
                  <Zap className="h-4 w-4 mr-2" />
                  Deploy Model
                </Button>
              )}
              {selectedModel.status === 'deployed' && (
                <Button onClick={() => {
                  setShowModelDetails(false);
                  setShowInferenceModal(true);
                }}>
                  <Play className="h-4 w-4 mr-2" />
                  Test Inference
                </Button>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Inference Modal */}
      {showInferenceModal && selectedModel && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-2xl">
            <div className="flex justify-between items-center mb-6">
              <div>
                <h2 className="text-xl font-semibold">Test Inference</h2>
                <p className="text-gray-600">{selectedModel.name}</p>
              </div>
              <Button variant="outline" onClick={() => setShowInferenceModal(false)}>
                ×
              </Button>
            </div>
            
            <div className="space-y-4">
              <div>
                <Label>Input</Label>
                <Textarea 
                  value={inferenceInput} 
                  onChange={e => setInferenceInput(e.target.value)}
                  placeholder="Enter your input here..."
                  rows={4}
                />
              </div>
              
              {inferenceOutput && (
                <div>
                  <Label>Output</Label>
                  <div className="border rounded-md p-3 bg-gray-50">
                    <p className="text-sm">{inferenceOutput}</p>
                  </div>
                </div>
              )}
            </div>
            
            <div className="flex justify-end gap-3 mt-6">
              <Button variant="outline" onClick={() => setShowInferenceModal(false)}>
                Close
              </Button>
              <Button 
                onClick={runInference}
                disabled={!inferenceInput || isRunningInference}
              >
                {isRunningInference ? (
                  <Activity className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Play className="h-4 w-4 mr-2" />
                )}
                {isRunningInference ? 'Running...' : 'Run Inference'}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
