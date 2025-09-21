import React, { useState, useEffect } from 'react';
import { Card, Button, Input, Badge, Select, SelectTrigger, SelectValue, SelectContent, SelectItem, Label, Textarea } from '../components/UI';
import {
  Gavel, Trophy, Clock, TrendingUp, Users, DollarSign, Eye, Edit, Trash2, 
  Plus, Filter, Grid, List, Star, CheckCircle, AlertCircle, Brain, Cpu, 
  Network, Activity, Zap, Shield, Link, Copy, ExternalLink, Info, Settings,
  BarChart3, FileText, Image, Music, Video, Code, Globe, Upload, RefreshCw,
  Save, Share2, Bookmark, History, Timer, Target, Award, Crown, Medal,
  TrendingDown, ArrowUp, ArrowDown, Minus, X, Check, AlertTriangle
} from 'lucide-react';

// =================== Types ===================
type AuctionStatus = 'active' | 'ended' | 'upcoming' | 'cancelled';
type BidStatus = 'active' | 'winning' | 'outbid' | 'cancelled' | 'won' | 'lost';
type AuctionType = 'model' | 'dataset' | 'compute' | 'storage' | 'service';

type Auction = {
  id: string;
  title: string;
  description: string;
  type: AuctionType;
  status: AuctionStatus;
  seller: string;
  startingPrice: number;
  currentPrice: number;
  reservePrice?: number;
  buyoutPrice?: number;
  endTime: string;
  startTime: string;
  bidCount: number;
  bidderCount: number;
  winner?: string;
  winningBid?: number;
  tags: string[];
  metadata?: Record<string, any>;
  imageUrl?: string;
};

type Bid = {
  id: string;
  auctionId: string;
  bidder: string;
  amount: number;
  timestamp: string;
  status: BidStatus;
  isWinning: boolean;
  autoBid?: boolean;
  maxBid?: number;
  transactionHash?: string;
};

type Winner = {
  id: string;
  auctionId: string;
  auctionTitle: string;
  winner: string;
  winningBid: number;
  endTime: string;
  claimed: boolean;
  transactionHash?: string;
  deliveryStatus: 'pending' | 'delivered' | 'disputed';
};

// =================== Mock Data ===================
const generateMockAuctions = (): Auction[] => [
  {
    id: 'auction_001',
    title: 'GPT-4o Model License',
    description: 'Exclusive license for GPT-4o model deployment on IPPAN network',
    type: 'model',
    status: 'active',
    seller: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
    startingPrice: 1000,
    currentPrice: 2500,
    reservePrice: 2000,
    buyoutPrice: 5000,
    endTime: new Date(Date.now() + 2 * 24 * 60 * 60 * 1000).toISOString(),
    startTime: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000).toISOString(),
    bidCount: 15,
    bidderCount: 8,
    tags: ['ai', 'language-model', 'gpt', 'exclusive'],
    metadata: {
      modelSize: '175B parameters',
      accuracy: '95.8%',
      latency: '1.2s'
    }
  },
  {
    id: 'auction_002',
    title: 'ImageNet-21K Dataset',
    description: 'Large-scale image dataset with 21,841 classes and 14 million images',
    type: 'dataset',
    status: 'active',
    seller: 'i1B2zP2eP6QGefi3DMPTfTL6SLmv8DivfNb',
    startingPrice: 500,
    currentPrice: 1200,
    reservePrice: 800,
    buyoutPrice: 2000,
    endTime: new Date(Date.now() + 1 * 24 * 60 * 60 * 1000).toISOString(),
    startTime: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString(),
    bidCount: 23,
    bidderCount: 12,
    tags: ['dataset', 'computer-vision', 'imagenet', 'large-scale'],
    metadata: {
      size: '150GB',
      records: '14M images',
      format: 'JPEG'
    }
  },
  {
    id: 'auction_003',
    title: 'High-Performance Compute Cluster',
    description: 'Dedicated GPU cluster for AI training and inference workloads',
    type: 'compute',
    status: 'active',
    seller: 'i1C3zP3eP7QGefi4DMPTfTL7SLmv9DivfNc',
    startingPrice: 2000,
    currentPrice: 3500,
    reservePrice: 3000,
    buyoutPrice: 8000,
    endTime: new Date(Date.now() + 3 * 24 * 60 * 60 * 1000).toISOString(),
    startTime: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000).toISOString(),
    bidCount: 8,
    bidderCount: 5,
    tags: ['compute', 'gpu', 'training', 'infrastructure'],
    metadata: {
      gpus: '8x A100 80GB',
      memory: '640GB VRAM',
      storage: '10TB NVMe'
    }
  },
  {
    id: 'auction_004',
    title: 'Stable Diffusion XL Model',
    description: 'High-quality image generation model for text-to-image synthesis',
    type: 'model',
    status: 'ended',
    seller: 'i1D4zP4eP8QGefi5DMPTfTL8SLmv0DivfNd',
    startingPrice: 800,
    currentPrice: 1800,
    reservePrice: 1000,
    endTime: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
    startTime: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000).toISOString(),
    bidCount: 31,
    bidderCount: 18,
    winner: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
    winningBid: 1800,
    tags: ['ai', 'generation', 'diffusion', 'image'],
    metadata: {
      modelSize: '3.5B parameters',
      resolution: '1024x1024',
      quality: 'High'
    }
  }
];

const generateMockBids = (): Bid[] => [
  {
    id: 'bid_001',
    auctionId: 'auction_001',
    bidder: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
    amount: 2500,
    timestamp: new Date(Date.now() - 30 * 60 * 1000).toISOString(),
    status: 'winning',
    isWinning: true,
    autoBid: true,
    maxBid: 3000,
    transactionHash: '0xabc123...'
  },
  {
    id: 'bid_002',
    auctionId: 'auction_001',
    bidder: 'iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X',
    amount: 2400,
    timestamp: new Date(Date.now() - 45 * 60 * 1000).toISOString(),
    status: 'outbid',
    isWinning: false,
    transactionHash: '0xdef456...'
  },
  {
    id: 'bid_003',
    auctionId: 'auction_002',
    bidder: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
    amount: 1200,
    timestamp: new Date(Date.now() - 15 * 60 * 1000).toISOString(),
    status: 'winning',
    isWinning: true,
    transactionHash: '0xghi789...'
  }
];

const generateMockWinners = (): Winner[] => [
  {
    id: 'winner_001',
    auctionId: 'auction_004',
    auctionTitle: 'Stable Diffusion XL Model',
    winner: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
    winningBid: 1800,
    endTime: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
    claimed: true,
    transactionHash: '0xwinner123...',
    deliveryStatus: 'delivered'
  },
  {
    id: 'winner_002',
    auctionId: 'auction_005',
    auctionTitle: 'BERT-Base Model License',
    winner: 'iDLZ4d490pJHpsL2PDoXTDA8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0X',
    winningBid: 950,
    endTime: new Date(Date.now() - 1 * 24 * 60 * 60 * 1000).toISOString(),
    claimed: false,
    transactionHash: '0xwinner456...',
    deliveryStatus: 'pending'
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
  
  if (diffMs <= 0) return 'Ended';
  
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffDays = Math.floor(diffHours / 24);
  const remainingHours = diffHours % 24;
  const remainingMinutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));

  if (diffDays > 0) return `${diffDays}d ${remainingHours}h`;
  if (diffHours > 0) return `${diffHours}h ${remainingMinutes}m`;
  return `${remainingMinutes}m`;
}

function getStatusColor(status: AuctionStatus | BidStatus): string {
  switch (status) {
    case 'active':
    case 'winning':
    case 'won':
      return 'bg-green-100 text-green-800';
    case 'ended':
    case 'lost':
      return 'bg-gray-100 text-gray-800';
    case 'upcoming':
    case 'pending':
      return 'bg-blue-100 text-blue-800';
    case 'cancelled':
    case 'outbid':
      return 'bg-red-100 text-red-800';
    case 'delivered':
      return 'bg-green-100 text-green-800';
    case 'disputed':
      return 'bg-yellow-100 text-yellow-800';
    default:
      return 'bg-gray-100 text-gray-800';
  }
}

function getTypeIcon(type: AuctionType): string {
  switch (type) {
    case 'model': return '🤖';
    case 'dataset': return '📊';
    case 'compute': return '⚡';
    case 'storage': return '💾';
    case 'service': return '🔧';
    default: return '🏷️';
  }
}

function formatPrice(amount: number): string {
  return `${amount.toLocaleString()} IPPAN`;
}

// =================== Component ===================
export default function BidsPage() {
  // Data
  const [auctions, setAuctions] = useState<Auction[]>([]);
  const [bids, setBids] = useState<Bid[]>([]);
  const [winners, setWinners] = useState<Winner[]>([]);
  const [selectedAuction, setSelectedAuction] = useState<Auction | null>(null);

  // Bid form
  const [bidAmount, setBidAmount] = useState('');
  const [autoBid, setAutoBid] = useState(false);
  const [maxBid, setMaxBid] = useState('');
  const [isPlacingBid, setIsPlacingBid] = useState(false);

  // UI state
  const [activeTab, setActiveTab] = useState<'auctions' | 'bids' | 'winners'>('auctions');
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const [showBidModal, setShowBidModal] = useState(false);
  const [showAuctionDetails, setShowAuctionDetails] = useState(false);
  const [selectedBid, setSelectedBid] = useState<Bid | null>(null);

  // Filters
  const [statusFilter, setStatusFilter] = useState<'all' | AuctionStatus>('all');
  const [typeFilter, setTypeFilter] = useState<'all' | AuctionType>('all');
  const [searchQuery, setSearchQuery] = useState('');

  // Initialize data
  useEffect(() => {
    setAuctions(generateMockAuctions());
    setBids(generateMockBids());
    setWinners(generateMockWinners());
  }, []);

  // Filter auctions
  const filteredAuctions = auctions.filter(auction => {
    const matchesSearch = auction.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         auction.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
                         auction.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesStatus = statusFilter === 'all' || auction.status === statusFilter;
    const matchesType = typeFilter === 'all' || auction.type === typeFilter;
    
    return matchesSearch && matchesStatus && matchesType;
  });

  // Place bid
  const handlePlaceBid = async () => {
    if (!selectedAuction || !bidAmount) return;
    
    setIsPlacingBid(true);
    
    // Simulate bid placement
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    const newBid: Bid = {
      id: `bid_${Date.now()}`,
      auctionId: selectedAuction.id,
      bidder: 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X',
      amount: parseFloat(bidAmount),
      timestamp: new Date().toISOString(),
      status: 'active',
      isWinning: parseFloat(bidAmount) > selectedAuction.currentPrice,
      autoBid: autoBid,
      maxBid: autoBid ? parseFloat(maxBid) : undefined,
      transactionHash: `0x${Math.random().toString(16).substr(2, 8)}...`
    };
    
    setBids(prev => [newBid, ...prev]);
    
    // Update auction current price
    setAuctions(prev => prev.map(auction => 
      auction.id === selectedAuction.id 
        ? { ...auction, currentPrice: parseFloat(bidAmount), bidCount: auction.bidCount + 1 }
        : auction
    ));
    
    setIsPlacingBid(false);
    setShowBidModal(false);
    setBidAmount('');
    setMaxBid('');
    setAutoBid(false);
  };

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <header className="flex justify-between items-start">
        <div className="space-y-1">
          <h1 className="text-3xl font-bold text-gray-900">Bids & Winners</h1>
          <p className="text-gray-600">Participate in auctions, place bids, and track your wins</p>
        </div>
        <div className="flex items-center gap-3">
          <Badge variant="success" className="flex items-center gap-1">
            <Gavel className="h-3 w-3" />
            Auctions Active
          </Badge>
        </div>
      </header>

      {/* Statistics Dashboard */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Active Auctions</p>
              <p className="text-2xl font-bold text-gray-900">
                {auctions.filter(a => a.status === 'active').length}
              </p>
            </div>
            <Gavel className="h-8 w-8 text-blue-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Bids</p>
              <p className="text-2xl font-bold text-green-600">
                {auctions.reduce((sum, a) => sum + a.bidCount, 0)}
              </p>
            </div>
            <TrendingUp className="h-8 w-8 text-green-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Value</p>
              <p className="text-2xl font-bold text-purple-600">
                {formatPrice(auctions.reduce((sum, a) => sum + a.currentPrice, 0))}
              </p>
            </div>
            <DollarSign className="h-8 w-8 text-purple-600" />
          </div>
        </Card>
        <Card>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Your Wins</p>
              <p className="text-2xl font-bold text-orange-600">
                {winners.filter(w => w.winner === 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X').length}
              </p>
            </div>
            <Trophy className="h-8 w-8 text-orange-600" />
          </div>
        </Card>
      </div>

      {/* Tabs */}
      <div className="flex border-b">
        <Button 
          variant={activeTab === 'auctions' ? 'secondary' : 'ghost'} 
          onClick={() => setActiveTab('auctions')}
        >
          <Gavel className="h-4 w-4 mr-2" />
          Auctions
        </Button>
        <Button 
          variant={activeTab === 'bids' ? 'secondary' : 'ghost'} 
          onClick={() => setActiveTab('bids')}
        >
          <TrendingUp className="h-4 w-4 mr-2" />
          My Bids
        </Button>
        <Button 
          variant={activeTab === 'winners' ? 'secondary' : 'ghost'} 
          onClick={() => setActiveTab('winners')}
        >
          <Trophy className="h-4 w-4 mr-2" />
          Winners
        </Button>
      </div>

      {/* Filters */}
      <Card>
        <div className="flex flex-wrap items-center gap-4">
          <div className="flex items-center gap-2">
            <Input 
              placeholder="Search auctions..." 
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
              <SelectItem value="active">Active</SelectItem>
              <SelectItem value="ended">Ended</SelectItem>
              <SelectItem value="upcoming">Upcoming</SelectItem>
              <SelectItem value="cancelled">Cancelled</SelectItem>
            </SelectContent>
          </Select>
          <Select value={typeFilter} onValueChange={(v) => setTypeFilter(v as any)}>
            <SelectTrigger className="w-40">
              <SelectValue placeholder="Type" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Types</SelectItem>
              <SelectItem value="model">Models</SelectItem>
              <SelectItem value="dataset">Datasets</SelectItem>
              <SelectItem value="compute">Compute</SelectItem>
              <SelectItem value="storage">Storage</SelectItem>
              <SelectItem value="service">Services</SelectItem>
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

      {/* Content */}
      {activeTab === 'auctions' && (
        <Card>
          <div className="space-y-4">
            {viewMode === 'grid' ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {filteredAuctions.map(auction => (
                  <div key={auction.id} className="border rounded-lg p-4 hover:shadow-md transition-shadow">
                    <div className="flex items-start justify-between mb-3">
                      <div className="flex items-center gap-2">
                        <span className="text-2xl">{getTypeIcon(auction.type)}</span>
                        <div>
                          <h3 className="font-semibold text-gray-900">{auction.title}</h3>
                          <p className="text-sm text-gray-600">{auction.type} • {auction.bidCount} bids</p>
                        </div>
                      </div>
                      <Badge className={getStatusColor(auction.status)}>
                        {auction.status}
                      </Badge>
                    </div>
                    
                    <p className="text-sm text-gray-600 mb-3 line-clamp-2">{auction.description}</p>
                    
                    <div className="space-y-2 mb-4">
                      <div className="flex justify-between text-sm">
                        <span className="text-gray-500">Current Price:</span>
                        <span className="font-medium text-green-600">{formatPrice(auction.currentPrice)}</span>
                      </div>
                      <div className="flex justify-between text-sm">
                        <span className="text-gray-500">Starting Price:</span>
                        <span className="font-medium">{formatPrice(auction.startingPrice)}</span>
                      </div>
                      {auction.buyoutPrice && (
                        <div className="flex justify-between text-sm">
                          <span className="text-gray-500">Buyout:</span>
                          <span className="font-medium text-blue-600">{formatPrice(auction.buyoutPrice)}</span>
                        </div>
                      )}
                      <div className="flex justify-between text-sm">
                        <span className="text-gray-500">Time Left:</span>
                        <span className="font-medium">{formatTimeRemaining(auction.endTime)}</span>
                      </div>
                    </div>
                    
                    <div className="flex flex-wrap gap-1 mb-4">
                      {auction.tags.slice(0, 3).map(tag => (
                        <Badge key={tag} variant="secondary" className="text-xs">
                          {tag}
                        </Badge>
                      ))}
                    </div>
                    
                    <div className="flex gap-2">
                      <Button 
                        size="sm" 
                        variant="outline"
                        onClick={() => {
                          setSelectedAuction(auction);
                          setShowAuctionDetails(true);
                        }}
                      >
                        <Eye className="h-3 w-3 mr-1" />
                        Details
                      </Button>
                      {auction.status === 'active' && (
                        <Button 
                          size="sm"
                          onClick={() => {
                            setSelectedAuction(auction);
                            setShowBidModal(true);
                          }}
                        >
                          <Gavel className="h-3 w-3 mr-1" />
                          Bid
                        </Button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="space-y-3">
                {filteredAuctions.map(auction => (
                  <div key={auction.id} className="border rounded-lg p-4 hover:bg-gray-50">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-4">
                        <span className="text-2xl">{getTypeIcon(auction.type)}</span>
                        <div>
                          <div className="flex items-center gap-2">
                            <h3 className="font-semibold text-gray-900">{auction.title}</h3>
                            <Badge className={getStatusColor(auction.status)}>
                              {auction.status}
                            </Badge>
                          </div>
                          <p className="text-sm text-gray-600">{auction.description}</p>
                          <p className="text-xs text-gray-500">
                            {auction.bidCount} bids • {auction.bidderCount} bidders • 
                            Ends in {formatTimeRemaining(auction.endTime)}
                          </p>
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="font-semibold text-green-600">{formatPrice(auction.currentPrice)}</div>
                        <div className="text-sm text-gray-500">Current Bid</div>
                        <div className="flex gap-2 mt-2">
                          <Button 
                            size="sm" 
                            variant="outline"
                            onClick={() => {
                              setSelectedAuction(auction);
                              setShowAuctionDetails(true);
                            }}
                          >
                            <Eye className="h-3 w-3 mr-1" />
                            Details
                          </Button>
                          {auction.status === 'active' && (
                            <Button 
                              size="sm"
                              onClick={() => {
                                setSelectedAuction(auction);
                                setShowBidModal(true);
                              }}
                            >
                              <Gavel className="h-3 w-3 mr-1" />
                              Bid
                            </Button>
                          )}
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
            
            {filteredAuctions.length === 0 && (
              <div className="text-center py-12">
                <Gavel className="h-12 w-12 text-gray-400 mx-auto mb-4" />
                <h3 className="text-lg font-medium text-gray-900 mb-2">No auctions found</h3>
                <p className="text-gray-600">Try adjusting your search criteria.</p>
              </div>
            )}
          </div>
        </Card>
      )}

      {activeTab === 'bids' && (
        <Card>
          <div className="space-y-4">
            <h3 className="text-lg font-semibold">Your Bids</h3>
            <div className="space-y-3">
              {bids.map(bid => {
                const auction = auctions.find(a => a.id === bid.auctionId);
                return (
                  <div key={bid.id} className="border rounded-lg p-4 hover:bg-gray-50">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-4">
                        <span className="text-2xl">{auction ? getTypeIcon(auction.type) : '🏷️'}</span>
                        <div>
                          <div className="flex items-center gap-2">
                            <h4 className="font-semibold text-gray-900">
                              {auction?.title || 'Unknown Auction'}
                            </h4>
                            <Badge className={getStatusColor(bid.status)}>
                              {bid.status}
                            </Badge>
                            {bid.autoBid && (
                              <Badge variant="secondary">Auto</Badge>
                            )}
                          </div>
                          <p className="text-sm text-gray-600">
                            Bid: {formatPrice(bid.amount)} • {formatTimeAgo(bid.timestamp)}
                          </p>
                          {bid.autoBid && bid.maxBid && (
                            <p className="text-xs text-gray-500">
                              Max bid: {formatPrice(bid.maxBid)}
                            </p>
                          )}
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="font-semibold text-green-600">{formatPrice(bid.amount)}</div>
                        <div className="text-sm text-gray-500">Your Bid</div>
                        {bid.isWinning && (
                          <Badge className="bg-green-100 text-green-800 mt-1">
                            <Crown className="h-3 w-3 mr-1" />
                            Winning
                          </Badge>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </Card>
      )}

      {activeTab === 'winners' && (
        <Card>
          <div className="space-y-4">
            <h3 className="text-lg font-semibold">Recent Winners</h3>
            <div className="space-y-3">
              {winners.map(winner => (
                <div key={winner.id} className="border rounded-lg p-4 bg-green-50 border-green-200">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-4">
                      <Trophy className="h-8 w-8 text-yellow-600" />
                      <div>
                        <div className="flex items-center gap-2">
                          <h4 className="font-semibold text-green-800">{winner.auctionTitle}</h4>
                          <Badge className={getStatusColor(winner.deliveryStatus)}>
                            {winner.deliveryStatus}
                          </Badge>
                        </div>
                        <p className="text-sm text-green-600">
                          Winner: {winner.winner === 'iCzfRuaeBuDyHYkzP2PO6zC9d454cfc419a4bcdf4c3d686cc34a0b64b7ed4X' ? 'You' : 'Other'}
                        </p>
                        <p className="text-xs text-green-500">
                          Ended {formatTimeAgo(winner.endTime)} • 
                          {winner.claimed ? ' Claimed' : ' Not claimed'}
                        </p>
                      </div>
                    </div>
                    <div className="text-right">
                      <div className="font-semibold text-green-600">{formatPrice(winner.winningBid)}</div>
                      <div className="text-sm text-green-500">Winning Bid</div>
                      <Badge className="bg-yellow-100 text-yellow-800 mt-1">
                        <Award className="h-3 w-3 mr-1" />
                        Winner
                      </Badge>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </Card>
      )}

      {/* Bid Modal */}
      {showBidModal && selectedAuction && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-xl font-semibold">Place Bid</h2>
              <Button variant="outline" onClick={() => setShowBidModal(false)}>
                ×
              </Button>
            </div>
            
            <div className="space-y-4">
              <div className="border rounded-lg p-4 bg-blue-50">
                <h3 className="font-semibold text-blue-800">{selectedAuction.title}</h3>
                <p className="text-sm text-blue-600">Current bid: {formatPrice(selectedAuction.currentPrice)}</p>
                {selectedAuction.buyoutPrice && (
                  <p className="text-sm text-blue-600">Buyout: {formatPrice(selectedAuction.buyoutPrice)}</p>
                )}
              </div>
              
              <div>
                <Label>Bid Amount (IPPAN)</Label>
                <Input 
                  type="number"
                  placeholder="0.00"
                  value={bidAmount}
                  onChange={e => setBidAmount(e.target.value)}
                  min={selectedAuction.currentPrice + 1}
                  className="mt-1"
                />
                <p className="text-xs text-gray-500 mt-1">
                  Minimum bid: {formatPrice(selectedAuction.currentPrice + 1)}
                </p>
              </div>
              
              <div className="flex items-center space-x-2">
                <input 
                  type="checkbox" 
                  id="autoBid"
                  checked={autoBid}
                  onChange={e => setAutoBid(e.target.checked)}
                />
                <Label htmlFor="autoBid">Enable auto-bidding</Label>
              </div>
              
              {autoBid && (
                <div>
                  <Label>Maximum Bid (IPPAN)</Label>
                  <Input 
                    type="number"
                    placeholder="0.00"
                    value={maxBid}
                    onChange={e => setMaxBid(e.target.value)}
                    min={parseFloat(bidAmount) || 0}
                    className="mt-1"
                  />
                </div>
              )}
            </div>
            
            <div className="flex justify-end gap-3 mt-6">
              <Button variant="outline" onClick={() => setShowBidModal(false)}>
                Cancel
              </Button>
              <Button 
                onClick={handlePlaceBid}
                disabled={!bidAmount || parseFloat(bidAmount) <= selectedAuction.currentPrice || isPlacingBid}
              >
                {isPlacingBid ? (
                  <Activity className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Gavel className="h-4 w-4 mr-2" />
                )}
                {isPlacingBid ? 'Placing Bid...' : 'Place Bid'}
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Auction Details Modal */}
      {showAuctionDetails && selectedAuction && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-2xl max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-6">
              <div className="flex items-center gap-3">
                <span className="text-3xl">{getTypeIcon(selectedAuction.type)}</span>
                <div>
                  <h2 className="text-xl font-semibold">{selectedAuction.title}</h2>
                  <p className="text-gray-600">{selectedAuction.type} • {selectedAuction.bidCount} bids</p>
                </div>
              </div>
              <Button variant="outline" onClick={() => setShowAuctionDetails(false)}>
                ×
              </Button>
            </div>
            
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="space-y-4">
                <div>
                  <h3 className="font-medium mb-2">Auction Information</h3>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-600">Auction ID:</span>
                      <span className="font-mono text-xs">{selectedAuction.id}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Seller:</span>
                      <span className="font-mono text-xs">{selectedAuction.seller}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Status:</span>
                      <Badge className={getStatusColor(selectedAuction.status)}>
                        {selectedAuction.status}
                      </Badge>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Started:</span>
                      <span>{formatTimeAgo(selectedAuction.startTime)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Ends:</span>
                      <span>{formatTimeRemaining(selectedAuction.endTime)}</span>
                    </div>
                  </div>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Pricing</h3>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-gray-600">Starting Price:</span>
                      <span>{formatPrice(selectedAuction.startingPrice)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-600">Current Price:</span>
                      <span className="font-semibold text-green-600">{formatPrice(selectedAuction.currentPrice)}</span>
                    </div>
                    {selectedAuction.reservePrice && (
                      <div className="flex justify-between">
                        <span className="text-gray-600">Reserve Price:</span>
                        <span>{formatPrice(selectedAuction.reservePrice)}</span>
                      </div>
                    )}
                    {selectedAuction.buyoutPrice && (
                      <div className="flex justify-between">
                        <span className="text-gray-600">Buyout Price:</span>
                        <span className="text-blue-600">{formatPrice(selectedAuction.buyoutPrice)}</span>
                      </div>
                    )}
                  </div>
                </div>
              </div>
              
              <div className="space-y-4">
                <div>
                  <h3 className="font-medium mb-2">Description</h3>
                  <p className="text-sm text-gray-600">{selectedAuction.description}</p>
                </div>
                
                <div>
                  <h3 className="font-medium mb-2">Tags</h3>
                  <div className="flex flex-wrap gap-1">
                    {selectedAuction.tags.map(tag => (
                      <Badge key={tag} variant="secondary">
                        {tag}
                      </Badge>
                    ))}
                  </div>
                </div>
                
                {selectedAuction.metadata && (
                  <div>
                    <h3 className="font-medium mb-2">Details</h3>
                    <div className="space-y-2 text-sm">
                      {Object.entries(selectedAuction.metadata).map(([key, value]) => (
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
              <Button variant="outline" onClick={() => setShowAuctionDetails(false)}>
                Close
              </Button>
              {selectedAuction.status === 'active' && (
                <Button onClick={() => {
                  setShowAuctionDetails(false);
                  setShowBidModal(true);
                }}>
                  <Gavel className="h-4 w-4 mr-2" />
                  Place Bid
                </Button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
