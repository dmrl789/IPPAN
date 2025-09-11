import { useState, useEffect } from 'react'
import { Card, Button, Badge, LoadingSpinner, Modal, Field, Input } from '../../components/UI'

interface Metric {
  name: string
  value: string
  change: number
  trend: 'up' | 'down' | 'stable'
  icon: string
  description: string
  unit?: string
}

interface ChartData {
  label: string
  value: number
  color: string
  percentage: number
}

interface TimeSeriesData {
  timestamp: string
  value: number
  label: string
}

interface PerformanceMetric {
  category: string
  metrics: {
    name: string
    value: string
    target: string
    status: 'excellent' | 'good' | 'warning' | 'critical'
    trend: number
  }[]
}

interface AIJobAnalytics {
  totalJobs: number
  completedJobs: number
  failedJobs: number
  averageExecutionTime: number
  totalRewards: number
  activeModels: number
  popularModelTypes: { type: string; count: number; percentage: number }[]
}

export default function AnalyticsPage() {
  const [isLoading, setIsLoading] = useState(true)
  const [timeRange, setTimeRange] = useState('24h')
  const [selectedMetric, setSelectedMetric] = useState<string | null>(null)
  const [isExportModalOpen, setIsExportModalOpen] = useState(false)
  const [exportFormat, setExportFormat] = useState('csv')
  const [exportDateRange, setExportDateRange] = useState('7d')
  const [viewMode, setViewMode] = useState<'overview' | 'detailed' | 'comparison'>('overview')

  useEffect(() => {
    // Simulate loading
    setTimeout(() => setIsLoading(false), 1000)
  }, [])

  const metrics: Metric[] = [
    {
      name: 'Total Transactions',
      value: '1,234,567',
      change: 12.5,
      trend: 'up',
      icon: '📊',
      description: 'Total transactions processed in the selected time period',
      unit: 'txns'
    },
    {
      name: 'Active Addresses',
      value: '45,678',
      change: 8.3,
      trend: 'up',
      icon: '👥',
      description: 'Unique addresses that sent or received transactions',
      unit: 'addresses'
    },
    {
      name: 'Network TPS',
      value: '15.2',
      change: -2.1,
      trend: 'down',
      icon: '⚡',
      description: 'Transactions per second across the network',
      unit: 'TPS'
    },
    {
      name: 'Average Block Time',
      value: '2.1s',
      change: 0.0,
      trend: 'stable',
      icon: '⏱️',
      description: 'Average time between block production',
      unit: 'seconds'
    },
    {
      name: 'Files Shared',
      value: '2.4 PB',
      change: 5.7,
      trend: 'up',
      icon: '📁',
      description: 'Total amount of storage space used for shared files',
      unit: 'PB'
    },
    {
      name: 'Staking Ratio',
      value: '67.5%',
      change: 2.3,
      trend: 'up',
      icon: '🏦',
      description: 'Percentage of total supply currently staked',
      unit: '%'
    },
    {
      name: 'AI Jobs Completed',
      value: '12,456',
      change: 18.9,
      trend: 'up',
      icon: '🤖',
      description: 'AI/ML jobs completed in the time period',
      unit: 'jobs'
    },
    {
      name: 'Average Gas Price',
      value: '0.001',
      change: -3.2,
      trend: 'down',
      icon: '⛽',
      description: 'Average gas price for transactions',
      unit: 'IPPAN'
    }
  ]

  const transactionData: ChartData[] = [
    { label: 'Transfers', value: 45, color: 'bg-blue-500', percentage: 45 },
    { label: 'Staking', value: 25, color: 'bg-green-500', percentage: 25 },
    { label: 'AI Jobs', value: 20, color: 'bg-purple-500', percentage: 20 },
    { label: 'Smart Contracts', value: 10, color: 'bg-orange-500', percentage: 10 }
  ]

  const performanceMetrics: PerformanceMetric[] = [
    {
      category: 'Network Performance',
      metrics: [
        { name: 'Block Production Rate', value: '15.2', target: '15.0', status: 'excellent', trend: 1.3 },
        { name: 'Finality Time', value: '2.1s', target: '2.5s', status: 'excellent', trend: -0.2 },
        { name: 'Network Uptime', value: '99.8%', target: '99.5%', status: 'excellent', trend: 0.1 },
        { name: 'Validator Participation', value: '100%', target: '95%', status: 'excellent', trend: 0.0 }
      ]
    },
    {
      category: 'Economic Health',
      metrics: [
        { name: 'Total Supply', value: '1B IPPAN', target: '1B IPPAN', status: 'good', trend: 0.0 },
        { name: 'Circulating Supply', value: '750M IPPAN', target: '800M IPPAN', status: 'good', trend: 2.1 },
        { name: 'Staked Amount', value: '125M IPPAN', target: '100M IPPAN', status: 'excellent', trend: 5.2 },
        { name: 'Market Cap', value: '$750M', target: '$500M', status: 'excellent', trend: 8.7 }
      ]
    }
  ]

  const aiJobAnalytics: AIJobAnalytics = {
    totalJobs: 12456,
    completedJobs: 11890,
    failedJobs: 566,
    averageExecutionTime: 45.2,
    totalRewards: 2500000,
    activeModels: 1234,
    popularModelTypes: [
      { type: 'Image Generation', count: 450, percentage: 36.2 },
      { type: 'Text Analysis', count: 320, percentage: 25.7 },
      { type: 'Data Processing', count: 280, percentage: 22.5 },
      { type: 'Model Training', count: 194, percentage: 15.6 }
    ]
  }

  const timeSeriesData: TimeSeriesData[] = [
    { timestamp: '00:00', value: 12.5, label: 'TPS' },
    { timestamp: '04:00', value: 8.2, label: 'TPS' },
    { timestamp: '08:00', value: 18.7, label: 'TPS' },
    { timestamp: '12:00', value: 22.1, label: 'TPS' },
    { timestamp: '16:00', value: 19.8, label: 'TPS' },
    { timestamp: '20:00', value: 15.3, label: 'TPS' }
  ]

  // Helper functions
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'excellent': return 'text-green-600 bg-green-100'
      case 'good': return 'text-blue-600 bg-blue-100'
      case 'warning': return 'text-yellow-600 bg-yellow-100'
      case 'critical': return 'text-red-600 bg-red-100'
      default: return 'text-gray-600 bg-gray-100'
    }
  }

  const getTrendIcon = (trend: string) => {
    switch (trend) {
      case 'up': return '↗️'
      case 'down': return '↘️'
      case 'stable': return '→'
      default: return '→'
    }
  }

  const getTrendColor = (trend: string) => {
    switch (trend) {
      case 'up': return 'text-green-600'
      case 'down': return 'text-red-600'
      case 'stable': return 'text-gray-600'
      default: return 'text-gray-600'
    }
  }

  const handleExport = () => {
    setIsExportModalOpen(true)
  }

  const handleMetricClick = (metricName: string) => {
    setSelectedMetric(metricName)
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
        <h1 className="text-2xl font-bold text-gray-900">Analytics Dashboard</h1>
        <div className="flex items-center space-x-4">
          <div className="flex gap-2">
            <Button
              variant={viewMode === 'overview' ? 'primary' : 'secondary'}
              onClick={() => setViewMode('overview')}
            >
              Overview
            </Button>
            <Button
              variant={viewMode === 'detailed' ? 'primary' : 'secondary'}
              onClick={() => setViewMode('detailed')}
            >
              Detailed
            </Button>
            <Button
              variant={viewMode === 'comparison' ? 'primary' : 'secondary'}
              onClick={() => setViewMode('comparison')}
            >
              Compare
            </Button>
          </div>
          <select
            value={timeRange}
            onChange={(e) => setTimeRange(e.target.value)}
            className="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="1h">Last Hour</option>
            <option value="24h">Last 24 Hours</option>
            <option value="7d">Last 7 Days</option>
            <option value="30d">Last 30 Days</option>
            <option value="90d">Last 90 Days</option>
          </select>
          <Badge variant="success">Live Data</Badge>
        </div>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {metrics.map((metric) => (
          <Card 
            key={metric.name} 
            title={metric.name}
            className="cursor-pointer hover:shadow-lg transition-shadow"
            onClick={() => handleMetricClick(metric.name)}
          >
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-2xl">{metric.icon}</span>
                <Badge variant={metric.trend === 'up' ? 'success' : metric.trend === 'down' ? 'error' : 'default'}>
                  {getTrendIcon(metric.trend)} {metric.change}%
                </Badge>
              </div>
              <div className="text-2xl font-bold text-gray-900">{metric.value}</div>
              <div className="text-sm text-gray-600">{metric.description}</div>
              {metric.unit && (
                <div className="text-xs text-gray-500">Unit: {metric.unit}</div>
              )}
            </div>
          </Card>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Transaction Volume Chart */}
        <Card title="Transaction Volume Over Time">
          <div className="h-64 bg-gradient-to-br from-blue-50 to-indigo-100 rounded-lg p-4">
            <div className="h-full flex flex-col">
              <div className="text-center mb-4">
                <div className="text-2xl mb-2">📈</div>
                <div className="text-sm font-medium text-gray-700">TPS Trend ({timeRange})</div>
              </div>
              
              {/* Simple line chart representation */}
              <div className="flex-1 flex items-end justify-between space-x-2">
                {timeSeriesData.map((point, index) => {
                  const height = (point.value / 25) * 100 // Normalize to max 25 TPS
                  return (
                    <div key={index} className="flex flex-col items-center flex-1">
                      <div 
                        className="w-full bg-blue-500 rounded-t transition-all duration-300 hover:bg-blue-600"
                        style={{ height: `${height}%` }}
                        title={`${point.timestamp}: ${point.value} TPS`}
                      ></div>
                      <div className="text-xs text-gray-600 mt-2">{point.timestamp}</div>
                    </div>
                  )
                })}
              </div>
              
              <div className="mt-4 text-center">
                <div className="text-sm text-gray-600">
                  Peak: {Math.max(...timeSeriesData.map(d => d.value))} TPS at {timeSeriesData.find(d => d.value === Math.max(...timeSeriesData.map(d => d.value)))?.timestamp}
                </div>
              </div>
            </div>
          </div>
        </Card>

        {/* Transaction Types */}
        <Card title="Transaction Distribution">
          <div className="space-y-4">
            <div className="text-center mb-4">
              <div className="text-2xl mb-2">🥧</div>
              <div className="text-sm font-medium text-gray-700">Transaction Types by Volume</div>
            </div>
            
            {transactionData.map((item) => (
              <div key={item.label} className="space-y-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <div className={`w-4 h-4 rounded ${item.color}`}></div>
                    <span className="font-medium text-sm">{item.label}</span>
                  </div>
                  <div className="flex items-center space-x-2">
                    <span className="text-sm font-medium">{item.percentage}%</span>
                    <span className="text-xs text-gray-500">({item.value}%)</span>
                  </div>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-3">
                  <div 
                    className={`h-3 rounded-full transition-all duration-500 ${item.color}`}
                    style={{ width: `${item.percentage}%` }}
                  ></div>
                </div>
              </div>
            ))}
            
            <div className="pt-4 border-t">
              <div className="text-xs text-gray-500 text-center">
                Total: {transactionData.reduce((sum, item) => sum + item.percentage, 0)}% of all transactions
              </div>
            </div>
          </div>
        </Card>
      </div>

      {/* Performance Metrics */}
      {performanceMetrics.map((category) => (
        <Card key={category.category} title={category.category}>
          <div className="space-y-4">
            {category.metrics.map((metric, index) => (
              <div key={index} className="p-4 bg-gray-50 rounded-lg">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm font-medium text-gray-700">{metric.name}</span>
                  <div className="flex items-center space-x-2">
                    <Badge variant={metric.status === 'excellent' ? 'success' : metric.status === 'good' ? 'blue' : metric.status === 'warning' ? 'warning' : 'error'}>
                      {metric.status}
                    </Badge>
                    <span className={`text-xs ${metric.trend > 0 ? 'text-green-600' : metric.trend < 0 ? 'text-red-600' : 'text-gray-600'}`}>
                      {metric.trend > 0 ? '↗' : metric.trend < 0 ? '↘' : '→'} {Math.abs(metric.trend)}%
                    </span>
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <div className="text-xl font-bold text-gray-900">{metric.value}</div>
                  <div className="text-sm text-gray-500">Target: {metric.target}</div>
                </div>
                <div className="mt-2 w-full bg-gray-200 rounded-full h-2">
                  <div 
                    className={`h-2 rounded-full transition-all duration-300 ${
                      metric.status === 'excellent' ? 'bg-green-500' : 
                      metric.status === 'good' ? 'bg-blue-500' : 
                      metric.status === 'warning' ? 'bg-yellow-500' : 'bg-red-500'
                    }`}
                    style={{ width: '85%' }} // Simplified progress
                  ></div>
                </div>
              </div>
            ))}
          </div>
        </Card>
      ))}

      {/* AI/ML Analytics */}
      <Card title="AI/ML Marketplace Analytics">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          <div className="space-y-4">
            <div className="text-center">
              <div className="text-3xl font-bold text-blue-600">{aiJobAnalytics.totalJobs.toLocaleString()}</div>
              <div className="text-sm text-gray-600">Total Jobs</div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-green-600">{aiJobAnalytics.completedJobs.toLocaleString()}</div>
              <div className="text-sm text-gray-600">Completed</div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-red-600">{aiJobAnalytics.failedJobs.toLocaleString()}</div>
              <div className="text-sm text-gray-600">Failed</div>
            </div>
          </div>
          
          <div className="space-y-4">
            <div className="text-center">
              <div className="text-3xl font-bold text-purple-600">{aiJobAnalytics.averageExecutionTime}s</div>
              <div className="text-sm text-gray-600">Avg Execution Time</div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-orange-600">{aiJobAnalytics.totalRewards.toLocaleString()}</div>
              <div className="text-sm text-gray-600">Total Rewards (IPPAN)</div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-indigo-600">{aiJobAnalytics.activeModels.toLocaleString()}</div>
              <div className="text-sm text-gray-600">Active Models</div>
            </div>
          </div>
          
          <div className="space-y-4">
            <div className="text-sm font-medium text-gray-700 mb-3">Popular Model Types</div>
            {aiJobAnalytics.popularModelTypes.map((model, index) => (
              <div key={index} className="space-y-2">
                <div className="flex justify-between items-center">
                  <span className="text-sm font-medium">{model.type}</span>
                  <span className="text-sm text-gray-600">{model.percentage}%</span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div 
                    className="bg-gradient-to-r from-blue-500 to-purple-500 h-2 rounded-full transition-all duration-300"
                    style={{ width: `${model.percentage}%` }}
                  ></div>
                </div>
                <div className="text-xs text-gray-500">{model.count} models</div>
              </div>
            ))}
          </div>
        </div>
      </Card>

      {/* Historical Data */}
      <Card title="Historical Performance Trends">
        <div className="h-96 bg-gradient-to-br from-green-50 to-blue-100 rounded-lg p-6">
          <div className="h-full flex flex-col">
            <div className="text-center mb-6">
              <div className="text-3xl mb-2">📊</div>
              <div className="text-lg font-semibold text-gray-700">Multi-Metric Performance Chart</div>
              <div className="text-sm text-gray-600">Network performance metrics over {timeRange}</div>
            </div>
            
            {/* Legend */}
            <div className="flex justify-center space-x-6 mb-4">
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-blue-500 rounded-full"></div>
                <span className="text-sm">TPS</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-green-500 rounded-full"></div>
                <span className="text-sm">Active Addresses</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-purple-500 rounded-full"></div>
                <span className="text-sm">Transaction Volume</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-3 h-3 bg-orange-500 rounded-full"></div>
                <span className="text-sm">AI Jobs</span>
              </div>
            </div>
            
            {/* Chart placeholder with enhanced styling */}
            <div className="flex-1 bg-white rounded-lg p-4 shadow-sm">
              <div className="h-full flex items-center justify-center">
                <div className="text-center">
                  <div className="text-4xl mb-4">📈</div>
                  <div className="text-lg font-semibold text-gray-700">Interactive Chart</div>
                  <div className="text-sm text-gray-600">Hover over data points for detailed metrics</div>
                  <div className="mt-4 text-xs text-gray-500">
                    Peak TPS: 22.1 | Avg Active Addresses: 45,678 | Total Volume: 1.2M IPPAN
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </Card>

      {/* Export Options */}
      <Card title="Data Export & Reports">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <Button variant="primary" onClick={handleExport}>
            📊 Export CSV
          </Button>
          <Button variant="secondary" onClick={handleExport}>
            📄 Export JSON
          </Button>
          <Button variant="secondary" onClick={handleExport}>
            📋 Generate Report
          </Button>
          <Button variant="secondary" onClick={handleExport}>
            🔗 Share Dashboard
          </Button>
        </div>
        
        <div className="mt-6 p-4 bg-gray-50 rounded-lg">
          <div className="text-sm font-medium text-gray-700 mb-2">Quick Stats Summary</div>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <div className="text-gray-600">Time Range</div>
              <div className="font-semibold">{timeRange}</div>
            </div>
            <div>
              <div className="text-gray-600">Data Points</div>
              <div className="font-semibold">{timeSeriesData.length}</div>
            </div>
            <div>
              <div className="text-gray-600">Last Updated</div>
              <div className="font-semibold">{new Date().toLocaleTimeString()}</div>
            </div>
            <div>
              <div className="text-gray-600">Refresh Rate</div>
              <div className="font-semibold">Real-time</div>
            </div>
          </div>
        </div>
      </Card>

      {/* Export Modal */}
      <Modal
        isOpen={isExportModalOpen}
        onClose={() => setIsExportModalOpen(false)}
        title="Export Analytics Data"
      >
        <div className="space-y-6">
          <Field label="Export Format">
            <select
              value={exportFormat}
              onChange={(e) => setExportFormat(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="csv">CSV (Excel Compatible)</option>
              <option value="json">JSON (API Format)</option>
              <option value="pdf">PDF Report</option>
              <option value="xlsx">Excel Workbook</option>
            </select>
          </Field>

          <Field label="Date Range">
            <select
              value={exportDateRange}
              onChange={(e) => setExportDateRange(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="1h">Last Hour</option>
              <option value="24h">Last 24 Hours</option>
              <option value="7d">Last 7 Days</option>
              <option value="30d">Last 30 Days</option>
              <option value="90d">Last 90 Days</option>
            </select>
          </Field>

          <Field label="Include Metrics">
            <div className="space-y-2">
              {metrics.map((metric) => (
                <label key={metric.name} className="flex items-center space-x-2">
                  <input type="checkbox" defaultChecked className="rounded" />
                  <span className="text-sm">{metric.name}</span>
                </label>
              ))}
            </div>
          </Field>

          <div className="flex justify-end space-x-2 pt-4 border-t">
            <Button variant="secondary" onClick={() => setIsExportModalOpen(false)}>
              Cancel
            </Button>
            <Button variant="primary">
              Export Data
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  )
}
