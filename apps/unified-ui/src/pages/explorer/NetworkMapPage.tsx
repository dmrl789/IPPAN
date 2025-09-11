import { useState, useEffect } from 'react'
import { Card, Button, Badge, LoadingSpinner, Modal, Field, Input } from '../../components/UI'

interface Node {
  id: string
  type: 'validator' | 'full_node' | 'light_node' | 'bootstrap' | 'relay'
  location: string
  country: string
  status: 'online' | 'offline' | 'syncing' | 'connecting'
  connections: number
  lastSeen: string
  version: string
  uptime: number
  latency?: number
  bandwidth?: number
  stake?: number
  reputation: number
  isp?: string
  coordinates?: { lat: number; lng: number }
  capabilities: string[]
  lastBlockHeight?: number
  syncProgress?: number
}

interface NetworkConnection {
  from: string
  to: string
  type: 'direct' | 'relay' | 'bootstrap'
  latency: number
  bandwidth: number
  quality: 'excellent' | 'good' | 'fair' | 'poor'
}

interface NetworkMetrics {
  totalNodes: number
  onlineNodes: number
  averageLatency: number
  networkThroughput: number
  consensusHealth: number
  geographicDistribution: Record<string, number>
  nodeTypeDistribution: Record<string, number>
}

export default function NetworkMapPage() {
  const [nodes, setNodes] = useState<Node[]>([])
  const [connections, setConnections] = useState<NetworkConnection[]>([])
  const [networkMetrics, setNetworkMetrics] = useState<NetworkMetrics | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [selectedNode, setSelectedNode] = useState<Node | null>(null)
  const [isNodeModalOpen, setIsNodeModalOpen] = useState(false)
  const [viewMode, setViewMode] = useState<'topology' | 'geographic' | 'performance'>('topology')
  const [filterType, setFilterType] = useState<string>('all')
  const [filterStatus, setFilterStatus] = useState<string>('all')

  useEffect(() => {
    const mockNodes: Node[] = [
      {
        id: '0xValidator1...',
        type: 'validator',
        location: 'New York, US',
        country: 'US',
        status: 'online',
        connections: 25,
        lastSeen: new Date().toISOString(),
        version: '1.2.0',
        uptime: 99.8,
        latency: 12,
        bandwidth: 1000,
        stake: 50000,
        reputation: 0.98,
        isp: 'AWS',
        coordinates: { lat: 40.7128, lng: -74.0060 },
        capabilities: ['consensus', 'validation', 'block_production'],
        lastBlockHeight: 123456,
        syncProgress: 100
      },
      {
        id: '0xValidator2...',
        type: 'validator',
        location: 'London, UK',
        country: 'UK',
        status: 'online',
        connections: 22,
        lastSeen: new Date(Date.now() - 300000).toISOString(),
        version: '1.2.0',
        uptime: 99.9,
        latency: 8,
        bandwidth: 800,
        stake: 45000,
        reputation: 0.99,
        isp: 'DigitalOcean',
        coordinates: { lat: 51.5074, lng: -0.1278 },
        capabilities: ['consensus', 'validation', 'block_production'],
        lastBlockHeight: 123456,
        syncProgress: 100
      },
      {
        id: '0xFullNode1...',
        type: 'full_node',
        location: 'Tokyo, JP',
        country: 'JP',
        status: 'online',
        connections: 18,
        lastSeen: new Date(Date.now() - 600000).toISOString(),
        version: '1.1.9',
        uptime: 98.5,
        latency: 45,
        bandwidth: 500,
        reputation: 0.95,
        isp: 'NTT',
        coordinates: { lat: 35.6762, lng: 139.6503 },
        capabilities: ['storage', 'relay', 'sync'],
        lastBlockHeight: 123455,
        syncProgress: 100
      },
      {
        id: '0xLightNode1...',
        type: 'light_node',
        location: 'Sydney, AU',
        country: 'AU',
        status: 'syncing',
        connections: 5,
        lastSeen: new Date(Date.now() - 1200000).toISOString(),
        version: '1.1.8',
        uptime: 95.2,
        latency: 120,
        bandwidth: 100,
        reputation: 0.85,
        isp: 'Telstra',
        coordinates: { lat: -33.8688, lng: 151.2093 },
        capabilities: ['sync', 'query'],
        lastBlockHeight: 123400,
        syncProgress: 85
      },
      {
        id: '0xBootstrap1...',
        type: 'bootstrap',
        location: 'Frankfurt, DE',
        country: 'DE',
        status: 'online',
        connections: 50,
        lastSeen: new Date(Date.now() - 60000).toISOString(),
        version: '1.2.0',
        uptime: 99.5,
        latency: 15,
        bandwidth: 2000,
        reputation: 0.97,
        isp: 'Hetzner',
        coordinates: { lat: 50.1109, lng: 8.6821 },
        capabilities: ['bootstrap', 'discovery', 'relay'],
        lastBlockHeight: 123456,
        syncProgress: 100
      },
      {
        id: '0xRelay1...',
        type: 'relay',
        location: 'Singapore, SG',
        country: 'SG',
        status: 'online',
        connections: 30,
        lastSeen: new Date(Date.now() - 180000).toISOString(),
        version: '1.1.9',
        uptime: 98.8,
        latency: 25,
        bandwidth: 1500,
        reputation: 0.94,
        isp: 'AWS',
        coordinates: { lat: 1.3521, lng: 103.8198 },
        capabilities: ['relay', 'routing', 'sync'],
        lastBlockHeight: 123456,
        syncProgress: 100
      }
    ]

    const mockConnections: NetworkConnection[] = [
      { from: '0xValidator1...', to: '0xValidator2...', type: 'direct', latency: 45, bandwidth: 100, quality: 'excellent' },
      { from: '0xValidator1...', to: '0xFullNode1...', type: 'direct', latency: 120, bandwidth: 50, quality: 'good' },
      { from: '0xValidator2...', to: '0xBootstrap1...', type: 'direct', latency: 25, bandwidth: 200, quality: 'excellent' },
      { from: '0xFullNode1...', to: '0xRelay1...', type: 'relay', latency: 80, bandwidth: 75, quality: 'good' },
      { from: '0xLightNode1...', to: '0xRelay1...', type: 'relay', latency: 150, bandwidth: 25, quality: 'fair' }
    ]

    const mockMetrics: NetworkMetrics = {
      totalNodes: mockNodes.length,
      onlineNodes: mockNodes.filter(n => n.status === 'online').length,
      averageLatency: 54,
      networkThroughput: 1250,
      consensusHealth: 98.5,
      geographicDistribution: {
        'North America': 35,
        'Europe': 30,
        'Asia Pacific': 25,
        'Other': 10
      },
      nodeTypeDistribution: {
        'validators': 2,
        'full_nodes': 1,
        'light_nodes': 1,
        'bootstrap': 1,
        'relay': 1
      }
    }

    setNodes(mockNodes)
    setConnections(mockConnections)
    setNetworkMetrics(mockMetrics)
    setIsLoading(false)
  }, [])

  // Helper functions
  const getNodeTypeIcon = (type: string) => {
    switch (type) {
      case 'validator': return '🛡️'
      case 'full_node': return '🖥️'
      case 'light_node': return '💡'
      case 'bootstrap': return '🚀'
      case 'relay': return '🔄'
      default: return '❓'
    }
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'online': return 'success'
      case 'syncing': return 'warning'
      case 'connecting': return 'blue'
      case 'offline': return 'error'
      default: return 'default'
    }
  }

  const getQualityColor = (quality: string) => {
    switch (quality) {
      case 'excellent': return 'text-green-600'
      case 'good': return 'text-blue-600'
      case 'fair': return 'text-yellow-600'
      case 'poor': return 'text-red-600'
      default: return 'text-gray-600'
    }
  }

  const filteredNodes = nodes.filter(node => {
    const typeMatch = filterType === 'all' || node.type === filterType
    const statusMatch = filterStatus === 'all' || node.status === filterStatus
    return typeMatch && statusMatch
  })

  const handleNodeClick = (node: Node) => {
    setSelectedNode(node)
    setIsNodeModalOpen(true)
  }

  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        <LoadingSpinner />
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold text-gray-900">Network Map</h1>
        <Badge variant="success">Live Network</Badge>
      </div>

      {/* Network Overview */}
      {networkMetrics && (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <Card title="Total Nodes">
            <div className="text-center">
              <div className="text-3xl font-bold text-blue-600">{networkMetrics.totalNodes}</div>
              <div className="text-sm text-gray-600">Active Nodes</div>
            </div>
          </Card>
          <Card title="Online Nodes">
            <div className="text-center">
              <div className="text-3xl font-bold text-green-600">{networkMetrics.onlineNodes}</div>
              <div className="text-sm text-gray-600">{((networkMetrics.onlineNodes / networkMetrics.totalNodes) * 100).toFixed(1)}% Online</div>
            </div>
          </Card>
          <Card title="Avg Latency">
            <div className="text-center">
              <div className="text-3xl font-bold text-purple-600">{networkMetrics.averageLatency}ms</div>
              <div className="text-sm text-gray-600">Network Latency</div>
            </div>
          </Card>
          <Card title="Consensus Health">
            <div className="text-center">
              <div className="text-3xl font-bold text-orange-600">{networkMetrics.consensusHealth}%</div>
              <div className="text-sm text-gray-600">Network Health</div>
            </div>
          </Card>
        </div>
      )}

      {/* Controls */}
      <div className="flex flex-wrap gap-4 items-center">
        <div className="flex gap-2">
          <Button
            variant={viewMode === 'topology' ? 'primary' : 'secondary'}
            onClick={() => setViewMode('topology')}
          >
            Topology View
          </Button>
          <Button
            variant={viewMode === 'geographic' ? 'primary' : 'secondary'}
            onClick={() => setViewMode('geographic')}
          >
            Geographic View
          </Button>
          <Button
            variant={viewMode === 'performance' ? 'primary' : 'secondary'}
            onClick={() => setViewMode('performance')}
          >
            Performance View
          </Button>
        </div>
        
        <div className="flex gap-2">
          <select
            value={filterType}
            onChange={(e) => setFilterType(e.target.value)}
            className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="all">All Types</option>
            <option value="validator">Validators</option>
            <option value="full_node">Full Nodes</option>
            <option value="light_node">Light Nodes</option>
            <option value="bootstrap">Bootstrap</option>
            <option value="relay">Relay</option>
          </select>
          
          <select
            value={filterStatus}
            onChange={(e) => setFilterStatus(e.target.value)}
            className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="all">All Status</option>
            <option value="online">Online</option>
            <option value="syncing">Syncing</option>
            <option value="connecting">Connecting</option>
            <option value="offline">Offline</option>
          </select>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2">
          <Card title={`Network ${viewMode === 'topology' ? 'Topology' : viewMode === 'geographic' ? 'Geographic' : 'Performance'} View`}>
            <div className="h-96 bg-gradient-to-br from-blue-50 to-indigo-100 rounded-lg relative overflow-hidden">
              {viewMode === 'topology' && (
                <div className="absolute inset-0 p-4">
                  <div className="text-center mb-4">
                    <div className="text-4xl mb-2">🕸️</div>
                    <div className="text-lg font-semibold text-gray-700">Network Topology</div>
                    <div className="text-sm text-gray-600">P2P connection graph</div>
                  </div>
                  
                  {/* Simple topology visualization */}
                  <div className="relative h-64">
                    {filteredNodes.map((node, index) => {
                      const angle = (index * 2 * Math.PI) / filteredNodes.length
                      const centerX = 200
                      const centerY = 100
                      const radius = 80
                      const x = centerX + radius * Math.cos(angle)
                      const y = centerY + radius * Math.sin(angle)
                      
                      return (
                        <div
                          key={node.id}
                          className="absolute cursor-pointer transform -translate-x-1/2 -translate-y-1/2"
                          style={{ left: x, top: y }}
                          onClick={() => handleNodeClick(node)}
                        >
                          <div className={`w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold ${
                            node.type === 'validator' ? 'bg-green-500 text-white' :
                            node.type === 'full_node' ? 'bg-blue-500 text-white' :
                            node.type === 'light_node' ? 'bg-yellow-500 text-white' :
                            node.type === 'bootstrap' ? 'bg-purple-500 text-white' :
                            'bg-gray-500 text-white'
                          }`}>
                            {getNodeTypeIcon(node.type)}
                          </div>
                          <div className="text-xs text-center mt-1 max-w-16 truncate">
                            {node.id.slice(0, 8)}...
                          </div>
                        </div>
                      )
                    })}
                    
                    {/* Connection lines */}
                    {connections.map((conn, index) => {
                      const fromNode = filteredNodes.find(n => n.id === conn.from)
                      const toNode = filteredNodes.find(n => n.id === conn.to)
                      if (!fromNode || !toNode) return null
                      
                      const fromIndex = filteredNodes.findIndex(n => n.id === conn.from)
                      const toIndex = filteredNodes.findIndex(n => n.id === conn.to)
                      const fromAngle = (fromIndex * 2 * Math.PI) / filteredNodes.length
                      const toAngle = (toIndex * 2 * Math.PI) / filteredNodes.length
                      const centerX = 200
                      const centerY = 100
                      const radius = 80
                      const fromX = centerX + radius * Math.cos(fromAngle)
                      const fromY = centerY + radius * Math.sin(fromAngle)
                      const toX = centerX + radius * Math.cos(toAngle)
                      const toY = centerY + radius * Math.sin(toAngle)
                      
                      return (
                        <svg
                          key={index}
                          className="absolute inset-0 pointer-events-none"
                          style={{ zIndex: 1 }}
                        >
                          <line
                            x1={fromX}
                            y1={fromY}
                            x2={toX}
                            y2={toY}
                            stroke={conn.quality === 'excellent' ? '#10b981' : conn.quality === 'good' ? '#3b82f6' : '#f59e0b'}
                            strokeWidth="2"
                            opacity="0.6"
                          />
                        </svg>
                      )
                    })}
                  </div>
                </div>
              )}
              
              {viewMode === 'geographic' && (
                <div className="absolute inset-0 p-4">
                  <div className="text-center mb-4">
                    <div className="text-4xl mb-2">🌍</div>
                    <div className="text-lg font-semibold text-gray-700">Geographic Distribution</div>
                    <div className="text-sm text-gray-600">Node locations worldwide</div>
                  </div>
                  
                  <div className="grid grid-cols-2 gap-4 h-64">
                    {Object.entries(networkMetrics?.geographicDistribution || {}).map(([region, percentage]) => (
                      <div key={region} className="bg-white rounded-lg p-4 shadow-sm">
                        <div className="text-sm font-medium text-gray-700">{region}</div>
                        <div className="text-2xl font-bold text-blue-600">{percentage}%</div>
                        <div className="w-full bg-gray-200 rounded-full h-2 mt-2">
                          <div 
                            className="bg-blue-600 h-2 rounded-full" 
                            style={{ width: `${percentage}%` }}
                          ></div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              
              {viewMode === 'performance' && (
                <div className="absolute inset-0 p-4">
                  <div className="text-center mb-4">
                    <div className="text-4xl mb-2">⚡</div>
                    <div className="text-lg font-semibold text-gray-700">Performance Metrics</div>
                    <div className="text-sm text-gray-600">Network performance indicators</div>
                  </div>
                  
                  <div className="grid grid-cols-2 gap-4 h-64">
                    <div className="bg-white rounded-lg p-4 shadow-sm">
                      <div className="text-sm font-medium text-gray-700">Average Latency</div>
                      <div className="text-2xl font-bold text-green-600">{networkMetrics?.averageLatency}ms</div>
                      <div className="text-xs text-gray-500 mt-1">Network-wide average</div>
                    </div>
                    <div className="bg-white rounded-lg p-4 shadow-sm">
                      <div className="text-sm font-medium text-gray-700">Throughput</div>
                      <div className="text-2xl font-bold text-blue-600">{networkMetrics?.networkThroughput} MB/s</div>
                      <div className="text-xs text-gray-500 mt-1">Data transfer rate</div>
                    </div>
                    <div className="bg-white rounded-lg p-4 shadow-sm">
                      <div className="text-sm font-medium text-gray-700">Consensus Health</div>
                      <div className="text-2xl font-bold text-purple-600">{networkMetrics?.consensusHealth}%</div>
                      <div className="text-xs text-gray-500 mt-1">Validator participation</div>
                    </div>
                    <div className="bg-white rounded-lg p-4 shadow-sm">
                      <div className="text-sm font-medium text-gray-700">Connection Quality</div>
                      <div className="text-2xl font-bold text-orange-600">
                        {connections.filter(c => c.quality === 'excellent' || c.quality === 'good').length}/{connections.length}
                      </div>
                      <div className="text-xs text-gray-500 mt-1">Good connections</div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </Card>
        </div>

        <div className="lg:col-span-1">
          <Card title="Node Legend">
            <div className="space-y-3">
              <div className="flex items-center space-x-2">
                <div className="w-4 h-4 bg-green-500 rounded-full"></div>
                <span className="text-sm">Validators ({nodes.filter(n => n.type === 'validator').length})</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-4 h-4 bg-blue-500 rounded-full"></div>
                <span className="text-sm">Full Nodes ({nodes.filter(n => n.type === 'full_node').length})</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-4 h-4 bg-yellow-500 rounded-full"></div>
                <span className="text-sm">Light Nodes ({nodes.filter(n => n.type === 'light_node').length})</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-4 h-4 bg-purple-500 rounded-full"></div>
                <span className="text-sm">Bootstrap ({nodes.filter(n => n.type === 'bootstrap').length})</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-4 h-4 bg-gray-500 rounded-full"></div>
                <span className="text-sm">Relay ({nodes.filter(n => n.type === 'relay').length})</span>
              </div>
            </div>
            
            <div className="mt-6 pt-4 border-t">
              <div className="text-sm font-medium text-gray-700 mb-3">Connection Quality</div>
              <div className="space-y-2">
                <div className="flex items-center space-x-2">
                  <div className="w-3 h-3 bg-green-500 rounded-full"></div>
                  <span className="text-xs">Excellent</span>
                </div>
                <div className="flex items-center space-x-2">
                  <div className="w-3 h-3 bg-blue-500 rounded-full"></div>
                  <span className="text-xs">Good</span>
                </div>
                <div className="flex items-center space-x-2">
                  <div className="w-3 h-3 bg-yellow-500 rounded-full"></div>
                  <span className="text-xs">Fair</span>
                </div>
                <div className="flex items-center space-x-2">
                  <div className="w-3 h-3 bg-red-500 rounded-full"></div>
                  <span className="text-xs">Poor</span>
                </div>
              </div>
            </div>
          </Card>
        </div>
      </div>

      <Card title={`Network Nodes (${filteredNodes.length} shown)`}>
        <div className="space-y-3">
          {filteredNodes.map((node) => (
            <div
              key={node.id}
              className={`p-4 border rounded-lg cursor-pointer transition-all hover:shadow-md ${
                selectedNode?.id === node.id ? 'border-blue-500 bg-blue-50' : 'border-gray-200'
              }`}
              onClick={() => handleNodeClick(node)}
            >
              <div className="flex justify-between items-start">
                <div className="flex-1">
                  <div className="flex items-center space-x-2 mb-2">
                    <span className="text-lg">{getNodeTypeIcon(node.type)}</span>
                    <h3 className="font-semibold text-sm">{node.id}</h3>
                    <Badge variant={node.type === 'validator' ? 'success' : node.type === 'full_node' ? 'blue' : node.type === 'bootstrap' ? 'purple' : 'warning'}>
                      {node.type.replace('_', ' ').toUpperCase()}
                    </Badge>
                    <Badge variant={getStatusColor(node.status)}>
                      {node.status}
                    </Badge>
                  </div>
                  <div className="text-sm text-gray-600 space-y-1">
                    <div className="flex justify-between">
                      <span>📍 {node.location}</span>
                      <span className="text-xs">{(node.reputation * 100).toFixed(1)}% reputation</span>
                    </div>
                    <div className="flex justify-between">
                      <span>🔗 {node.connections} connections</span>
                      <span className="text-xs">{node.latency}ms latency</span>
                    </div>
                    <div className="flex justify-between">
                      <span>📦 v{node.version}</span>
                      <span className="text-xs">{node.uptime}% uptime</span>
                    </div>
                    {node.syncProgress !== undefined && node.syncProgress < 100 && (
                      <div className="w-full bg-gray-200 rounded-full h-1.5 mt-2">
                        <div 
                          className="bg-blue-600 h-1.5 rounded-full transition-all duration-300" 
                          style={{ width: `${node.syncProgress}%` }}
                        ></div>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </Card>

      {/* Node Detail Modal */}
      {selectedNode && (
        <Modal
          isOpen={isNodeModalOpen}
          onClose={() => setIsNodeModalOpen(false)}
          title={`Node Details - ${selectedNode.id}`}
        >
          <div className="space-y-6">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <Field label="Node Type">
                  <div className="flex items-center space-x-2">
                    <span className="text-lg">{getNodeTypeIcon(selectedNode.type)}</span>
                    <Badge variant={selectedNode.type === 'validator' ? 'success' : selectedNode.type === 'full_node' ? 'blue' : selectedNode.type === 'bootstrap' ? 'purple' : 'warning'}>
                      {selectedNode.type.replace('_', ' ').toUpperCase()}
                    </Badge>
                  </div>
                </Field>
              </div>
              <div>
                <Field label="Status">
                  <Badge variant={getStatusColor(selectedNode.status)}>
                    {selectedNode.status}
                  </Badge>
                </Field>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <Field label="Location">
                <div className="text-sm">
                  <div>📍 {selectedNode.location}</div>
                  <div className="text-gray-500">{selectedNode.country}</div>
                </div>
              </Field>
              <Field label="ISP">
                <div className="text-sm">{selectedNode.isp || 'Unknown'}</div>
              </Field>
            </div>

            <div className="grid grid-cols-3 gap-4">
              <Field label="Connections">
                <div className="text-2xl font-bold text-blue-600">{selectedNode.connections}</div>
              </Field>
              <Field label="Latency">
                <div className="text-2xl font-bold text-green-600">{selectedNode.latency}ms</div>
              </Field>
              <Field label="Bandwidth">
                <div className="text-2xl font-bold text-purple-600">{selectedNode.bandwidth} Mbps</div>
              </Field>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <Field label="Uptime">
                <div className="text-2xl font-bold text-orange-600">{selectedNode.uptime}%</div>
              </Field>
              <Field label="Reputation">
                <div className="text-2xl font-bold text-indigo-600">{(selectedNode.reputation * 100).toFixed(1)}%</div>
              </Field>
            </div>

            {selectedNode.stake && (
              <Field label="Stake">
                <div className="text-2xl font-bold text-green-600">{selectedNode.stake.toLocaleString()} IPPAN</div>
              </Field>
            )}

            <Field label="Version">
              <div className="text-sm font-mono bg-gray-100 px-3 py-2 rounded">{selectedNode.version}</div>
            </Field>

            <Field label="Capabilities">
              <div className="flex flex-wrap gap-2">
                {selectedNode.capabilities.map((capability, index) => (
                  <Badge key={index} variant="blue">
                    {capability.replace('_', ' ')}
                  </Badge>
                ))}
              </div>
            </Field>

            {selectedNode.lastBlockHeight && (
              <Field label="Block Height">
                <div className="text-sm font-mono bg-gray-100 px-3 py-2 rounded">
                  {selectedNode.lastBlockHeight.toLocaleString()}
                </div>
              </Field>
            )}

            {selectedNode.syncProgress !== undefined && selectedNode.syncProgress < 100 && (
              <Field label="Sync Progress">
                <div className="space-y-2">
                  <div className="w-full bg-gray-200 rounded-full h-3">
                    <div 
                      className="bg-blue-600 h-3 rounded-full transition-all duration-300" 
                      style={{ width: `${selectedNode.syncProgress}%` }}
                    ></div>
                  </div>
                  <div className="text-sm text-center">{selectedNode.syncProgress}%</div>
                </div>
              </Field>
            )}

            <Field label="Last Seen">
              <div className="text-sm">
                {new Date(selectedNode.lastSeen).toLocaleString()}
              </div>
            </Field>

            <div className="flex justify-end space-x-2 pt-4 border-t">
              <Button variant="secondary" onClick={() => setIsNodeModalOpen(false)}>
                Close
              </Button>
              <Button variant="primary">
                Connect to Node
              </Button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  )
}
