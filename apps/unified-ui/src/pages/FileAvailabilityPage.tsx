import { useState, useEffect } from 'react'
import { Card, Button, Input, Badge, Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '../components/UI'

// Types for network file availability
type FileStatus = 
  | "available" 
  | "replicating" 
  | "degraded" 
  | "unavailable"
  | "archived"

type NetworkFile = {
  id: string
  hash: string
  name: string
  size: number
  mimeType: string
  owner: string
  uploadedAt: string
  status: FileStatus
  replicationFactor: number
  availableReplicas: number
  storageNodes: string[]
  txHash: string
  blockHeight: number
  lastAccessed?: string
  downloadCount: number
  tags: string[]
  isPublic: boolean
}

type FileFilter = {
  status: FileStatus | "all"
  mimeType: string | "all"
  sizeRange: "all" | "small" | "medium" | "large"
  timeRange: "1h" | "24h" | "7d" | "30d" | "all"
  isPublic: boolean | "all"
}

export default function FileAvailabilityPage() {
  const [files, setFiles] = useState<NetworkFile[]>([])
  const [filteredFiles, setFilteredFiles] = useState<NetworkFile[]>([])
  const [searchQuery, setSearchQuery] = useState("")
  const [filters, setFilters] = useState<FileFilter>({
    status: "all",
    mimeType: "all",
    sizeRange: "all",
    timeRange: "24h",
    isPublic: "all"
  })
  const [loading, setLoading] = useState(true)
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid")
  
  // Pagination and performance
  const [currentPage, setCurrentPage] = useState(1)
  const [itemsPerPage] = useState(20)
  const [totalFiles, setTotalFiles] = useState(0)
  const [blockchainStatus, setBlockchainStatus] = useState<{
    connected: boolean
    lastBlock: number
    syncProgress: number
    nodeCount: number
  }>({
    connected: false,
    lastBlock: 0,
    syncProgress: 0,
    nodeCount: 0
  })
  
  // Modal and action states
  const [selectedFile, setSelectedFile] = useState<NetworkFile | null>(null)
  const [showDetailsModal, setShowDetailsModal] = useState(false)
  const [downloadingFiles, setDownloadingFiles] = useState<Set<string>>(new Set())
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({})

  // Generate hundreds of mock files with blockchain data
  const generateMockFiles = (count: number): NetworkFile[] => {
    const fileTypes = [
      { mime: "application/pdf", icon: "📄", names: ["research", "document", "report", "paper"] },
      { mime: "text/csv", icon: "📊", names: ["dataset", "data", "export", "analysis"] },
      { mime: "video/mp4", icon: "🎥", names: ["tutorial", "demo", "presentation", "recording"] },
      { mime: "image/jpeg", icon: "🖼️", names: ["photo", "image", "screenshot", "diagram"] },
      { mime: "audio/mp3", icon: "🎵", names: ["podcast", "music", "recording", "audio"] },
      { mime: "application/zip", icon: "📦", names: ["archive", "backup", "package", "bundle"] },
      { mime: "text/plain", icon: "📝", names: ["readme", "notes", "log", "config"] }
    ]
    
    const statuses: FileStatus[] = ["available", "replicating", "degraded", "unavailable", "archived"]
    const owners = [
      "i1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
      "i1B2zP2eP6QGefi3DMPTfTL6SLmv8DivfNb", 
      "i1C3zP3eP7QGefi4DMPTfTL7SLmv9DivfNc",
      "i1D4zP4eP8QGefi5DMPTfTL8SLmv0DivfNd",
      "i1E5zP5eP9QGefi6DMPTfTL9SLmv1DivfNe"
    ]
    const nodes = ["node_us_west", "node_eu_central", "node_asia_pacific", "node_us_east", "node_eu_west", "node_asia_south"]
    
    return Array.from({ length: count }, (_, i) => {
      const fileType = fileTypes[i % fileTypes.length]
      const status = statuses[i % statuses.length]
      const owner = owners[i % owners.length]
      const isPublic = Math.random() > 0.3
      const blockHeight = 12000 + i
      const txHash = `0x${(i * 123456789).toString(16).padStart(40, '0')}`
      const hash = `0x${(i * 987654321).toString(16).padStart(40, '0')}`
      
      return {
        id: `file_${String(i + 1).padStart(3, '0')}`,
        hash,
        name: `${fileType.names[i % fileType.names.length]}_${i + 1}.${fileType.mime.split('/')[1]}`,
        size: Math.floor(Math.random() * 100000000) + 1024, // 1KB to 100MB
        mimeType: fileType.mime,
        owner,
        uploadedAt: new Date(Date.now() - Math.random() * 7 * 24 * 60 * 60 * 1000).toISOString(),
        status,
        replicationFactor: Math.floor(Math.random() * 3) + 3, // 3-5 replicas
        availableReplicas: status === "unavailable" ? 0 : Math.floor(Math.random() * 3) + 1,
        storageNodes: status === "unavailable" ? [] : nodes.slice(0, Math.floor(Math.random() * 3) + 1),
        txHash,
        blockHeight,
        downloadCount: Math.floor(Math.random() * 1000),
        tags: isPublic ? ["public", "shared"] : ["private"],
        isPublic
      }
    })
  }

  // Load files and blockchain status
  useEffect(() => {
    const loadData = async () => {
      setLoading(true)
      
      // Simulate blockchain connection
      setBlockchainStatus({
        connected: true,
        lastBlock: 12500,
        syncProgress: 100,
        nodeCount: 47
      })
      
      // Generate 500 mock files
      const mockFiles = generateMockFiles(500)
      setFiles(mockFiles)
      setTotalFiles(mockFiles.length)
      setLoading(false)
    }
    
    loadData()
    
    // Simulate real-time blockchain updates
    const interval = setInterval(() => {
      setBlockchainStatus(prev => ({
        ...prev,
        lastBlock: prev.lastBlock + 1,
        nodeCount: prev.nodeCount + Math.floor(Math.random() * 3) - 1
      }))
    }, 5000)
    
    return () => clearInterval(interval)
  }, [])

  // Filter and search logic
  useEffect(() => {
    let filtered = files

    // Filter by status
    if (filters.status !== "all") {
      filtered = filtered.filter(file => file.status === filters.status)
    }

    // Filter by MIME type
    if (filters.mimeType !== "all") {
      filtered = filtered.filter(file => file.mimeType.includes(filters.mimeType))
    }

    // Filter by size range
    if (filters.sizeRange !== "all") {
      const sizeRanges = {
        "small": [0, 1024 * 1024], // 0-1MB
        "medium": [1024 * 1024, 10 * 1024 * 1024], // 1-10MB
        "large": [10 * 1024 * 1024, Infinity] // 10MB+
      }
      const [min, max] = sizeRanges[filters.sizeRange]
      filtered = filtered.filter(file => file.size >= min && file.size < max)
    }

    // Filter by time range
    if (filters.timeRange !== "all") {
      const now = Date.now()
      const timeRanges = {
        "1h": 1000 * 60 * 60,
        "24h": 1000 * 60 * 60 * 24,
        "7d": 1000 * 60 * 60 * 24 * 7,
        "30d": 1000 * 60 * 60 * 24 * 30
      }
      const cutoff = now - timeRanges[filters.timeRange]
      filtered = filtered.filter(file => new Date(file.uploadedAt).getTime() > cutoff)
    }

    // Filter by public/private
    if (filters.isPublic !== "all") {
      filtered = filtered.filter(file => file.isPublic === filters.isPublic)
    }

    // Search filter
    if (searchQuery) {
      filtered = filtered.filter(file => 
        file.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        file.hash.toLowerCase().includes(searchQuery.toLowerCase()) ||
        file.owner.toLowerCase().includes(searchQuery.toLowerCase()) ||
        file.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase())) ||
        file.txHash.toLowerCase().includes(searchQuery.toLowerCase())
      )
    }

    setFilteredFiles(filtered)
    setCurrentPage(1) // Reset to first page when filters change
  }, [files, filters, searchQuery])

  // Pagination logic
  const totalPages = Math.ceil(filteredFiles.length / itemsPerPage)
  const startIndex = (currentPage - 1) * itemsPerPage
  const endIndex = startIndex + itemsPerPage
  const currentFiles = filteredFiles.slice(startIndex, endIndex)

  const getStatusColor = (status: FileStatus) => {
    switch (status) {
      case "available": return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
      case "replicating": return "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200"
      case "degraded": return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200"
      case "unavailable": return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
      case "archived": return "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200"
      default: return "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200"
    }
  }

  const getFileIcon = (mimeType: string) => {
    if (mimeType.startsWith("image/")) return "🖼️"
    if (mimeType.startsWith("video/")) return "🎥"
    if (mimeType.startsWith("audio/")) return "🎵"
    if (mimeType.includes("pdf")) return "📄"
    if (mimeType.includes("text/")) return "📝"
    if (mimeType.includes("application/zip") || mimeType.includes("gzip")) return "📦"
    if (mimeType.includes("document")) return "📄"
    return "📁"
  }

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  }

  // Download handler
  const handleDownload = async (file: NetworkFile) => {
    if (downloadingFiles.has(file.id)) return
    
    setDownloadingFiles(prev => new Set(prev).add(file.id))
    setDownloadProgress(prev => ({ ...prev, [file.id]: 0 }))
    
    try {
      // Simulate download progress
      for (let progress = 0; progress <= 100; progress += 10) {
        await new Promise(resolve => setTimeout(resolve, 200))
        setDownloadProgress(prev => ({ ...prev, [file.id]: progress }))
      }
      
      // Call API to initiate download
      const response = await fetch(`/api/v1/availability/files/${file.id}/download?requester=current_user`, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      })
      
      if (response.ok) {
        const data = await response.json()
        
        // Create download link
        const link = document.createElement('a')
        link.href = data.download_url || '#'
        link.download = file.name
        link.click()
        
        // Show success message
        alert(`Download initiated for ${file.name}`)
      } else {
        throw new Error('Download failed')
      }
    } catch (error) {
      console.error('Download error:', error)
      alert(`Failed to download ${file.name}. Please try again.`)
    } finally {
      setDownloadingFiles(prev => {
        const newSet = new Set(prev)
        newSet.delete(file.id)
        return newSet
      })
      setDownloadProgress(prev => {
        const newProgress = { ...prev }
        delete newProgress[file.id]
        return newProgress
      })
    }
  }

  // Details handler
  const handleShowDetails = (file: NetworkFile) => {
    setSelectedFile(file)
    setShowDetailsModal(true)
  }

  // Close modal
  const handleCloseModal = () => {
    setShowDetailsModal(false)
    setSelectedFile(null)
  }

  const formatTimeAgo = (timestamp: string) => {
    const now = Date.now()
    const time = new Date(timestamp).getTime()
    const diff = now - time
    
    if (diff < 60000) return "Just now"
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`
    return `${Math.floor(diff / 86400000)}d ago`
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <div className="flex items-center space-x-3">
            <h1 className="text-2xl font-bold">Network File Availability</h1>
            <div className="flex items-center space-x-2">
              <div className={`w-3 h-3 rounded-full ${blockchainStatus.connected ? 'bg-green-500' : 'bg-red-500'}`}></div>
              <span className="text-sm text-gray-600 dark:text-gray-400">
                {blockchainStatus.connected ? 'Blockchain Connected' : 'Blockchain Disconnected'}
              </span>
            </div>
          </div>
          <p className="text-gray-600 dark:text-gray-400 mt-1">
            Discover and monitor {totalFiles.toLocaleString()} files shared across the IPPAN network from on-chain storage transactions
          </p>
          <div className="flex items-center space-x-4 text-sm text-gray-500 dark:text-gray-400 mt-2">
            <span>Block #{blockchainStatus.lastBlock.toLocaleString()}</span>
            <span>{blockchainStatus.nodeCount} nodes</span>
            <span>{blockchainStatus.syncProgress}% synced</span>
          </div>
        </div>
        <div className="flex space-x-2">
          <Button 
            variant={viewMode === "grid" ? "default" : "outline"}
            onClick={() => setViewMode("grid")}
          >
            ⊞ Grid
          </Button>
          <Button 
            variant={viewMode === "list" ? "default" : "outline"}
            onClick={() => setViewMode("list")}
          >
            ☰ List
          </Button>
          <Button onClick={() => window.location.reload()}>
            🔄 Refresh
          </Button>
        </div>
      </div>

      {/* Search and Filters */}
      <Card className="p-4">
        <div className="grid grid-cols-1 md:grid-cols-6 gap-4">
          {/* Search */}
          <div className="md:col-span-2">
            <Input
              placeholder="Search files, owners, tags, or transaction hashes..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>

          {/* Status Filter */}
          <Select value={filters.status} onValueChange={(value) => 
            setFilters(prev => ({ ...prev, status: value as FileStatus | "all" }))
          }>
            <SelectTrigger>
              <SelectValue placeholder="Status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Status</SelectItem>
              <SelectItem value="available">Available</SelectItem>
              <SelectItem value="replicating">Replicating</SelectItem>
              <SelectItem value="degraded">Degraded</SelectItem>
              <SelectItem value="unavailable">Unavailable</SelectItem>
              <SelectItem value="archived">Archived</SelectItem>
            </SelectContent>
          </Select>

          {/* File Type Filter */}
          <Select value={filters.mimeType} onValueChange={(value) =>
            setFilters(prev => ({ ...prev, mimeType: value }))
          }>
            <SelectTrigger>
              <SelectValue placeholder="File Type" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Types</SelectItem>
              <SelectItem value="image">Images</SelectItem>
              <SelectItem value="video">Videos</SelectItem>
              <SelectItem value="audio">Audio</SelectItem>
              <SelectItem value="pdf">PDFs</SelectItem>
              <SelectItem value="text">Text Files</SelectItem>
              <SelectItem value="application">Applications</SelectItem>
            </SelectContent>
          </Select>

          {/* Size Range Filter */}
          <Select value={filters.sizeRange} onValueChange={(value) =>
            setFilters(prev => ({ ...prev, sizeRange: value as any }))
          }>
            <SelectTrigger>
              <SelectValue placeholder="Size" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Sizes</SelectItem>
              <SelectItem value="small">Small (&lt;1MB)</SelectItem>
              <SelectItem value="medium">Medium (1-10MB)</SelectItem>
              <SelectItem value="large">Large (&gt;10MB)</SelectItem>
            </SelectContent>
          </Select>

          {/* Public/Private Filter */}
          <Select value={filters.isPublic.toString()} onValueChange={(value) =>
            setFilters(prev => ({ ...prev, isPublic: value === "all" ? "all" : value === "true" }))
          }>
            <SelectTrigger>
              <SelectValue placeholder="Visibility" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Files</SelectItem>
              <SelectItem value="true">Public Only</SelectItem>
              <SelectItem value="false">Private Only</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </Card>

      {/* Files List */}
      <div className="space-y-3">
        {loading ? (
          <div className="text-center py-8">Loading network files...</div>
        ) : filteredFiles.length === 0 ? (
          <Card className="p-8 text-center">
            <p className="text-gray-500">No files found matching your criteria</p>
          </Card>
        ) : viewMode === "grid" ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {currentFiles.map((file) => (
              <Card key={file.id} className="p-4">
                <div className="flex items-start space-x-3 mb-3">
                  <span className="text-2xl">{getFileIcon(file.mimeType)}</span>
                  <div className="flex-1 min-w-0">
                    <h3 className="font-medium truncate" title={file.name}>{file.name}</h3>
                    <p className="text-sm text-gray-500 dark:text-gray-400">{formatFileSize(file.size)}</p>
                  </div>
                </div>
                
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <Badge className={getStatusColor(file.status)}>
                      {file.status}
                    </Badge>
                    <Badge variant="outline">
                      {file.availableReplicas}/{file.replicationFactor} replicas
                    </Badge>
                  </div>
                  
                  <div className="text-xs text-gray-500 dark:text-gray-400 space-y-1">
                    <div>Owner: {file.owner.slice(0, 8)}...{file.owner.slice(-8)}</div>
                    <div>Uploaded: {formatTimeAgo(file.uploadedAt)}</div>
                    <div>Downloads: {file.downloadCount}</div>
                    <div className="flex items-center space-x-1">
                      <span>🔗</span>
                      <span>Block #{file.blockHeight}</span>
                    </div>
                    <div className="flex items-center space-x-1">
                      <span>📋</span>
                      <span className="font-mono text-xs">{file.txHash.slice(0, 8)}...{file.txHash.slice(-8)}</span>
                    </div>
                  </div>
                  
                  {file.tags.length > 0 && (
                    <div className="flex flex-wrap gap-1">
                      {file.tags.slice(0, 3).map((tag, index) => (
                        <Badge key={index} variant="secondary" className="text-xs">
                          {tag}
                        </Badge>
                      ))}
                      {file.tags.length > 3 && (
                        <Badge variant="secondary" className="text-xs">
                          +{file.tags.length - 3}
                        </Badge>
                      )}
                    </div>
                  )}
                  
                  <div className="flex space-x-2 pt-2">
                    <Button 
                      size="sm" 
                      variant="outline" 
                      className="flex-1"
                      onClick={() => handleDownload(file)}
                      disabled={downloadingFiles.has(file.id) || file.status === "unavailable"}
                    >
                      {downloadingFiles.has(file.id) ? (
                        <div className="flex items-center space-x-2">
                          <div className="w-4 h-4 border-2 border-gray-300 border-t-blue-600 rounded-full animate-spin"></div>
                          <span>{downloadProgress[file.id] || 0}%</span>
                        </div>
                      ) : (
                        "Download"
                      )}
                    </Button>
                    <Button 
                      size="sm" 
                      variant="outline"
                      onClick={() => handleShowDetails(file)}
                    >
                      Details
                    </Button>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        ) : (
          <div className="space-y-3">
            {currentFiles.map((file) => (
              <Card key={file.id} className="p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <span className="text-2xl">{getFileIcon(file.mimeType)}</span>
                    <div className="flex-1">
                      <div className="flex items-center space-x-2 mb-1">
                        <h3 className="font-medium">{file.name}</h3>
                        <Badge className={getStatusColor(file.status)}>
                          {file.status}
                        </Badge>
                        <Badge variant="outline">
                          {file.availableReplicas}/{file.replicationFactor}
                        </Badge>
                        {file.isPublic ? (
                          <Badge variant="secondary">Public</Badge>
                        ) : (
                          <Badge variant="outline">Private</Badge>
                        )}
                      </div>
                      <div className="flex items-center space-x-4 text-sm text-gray-500 dark:text-gray-400">
                        <span>{formatFileSize(file.size)}</span>
                        <span>Owner: {file.owner.slice(0, 8)}...{file.owner.slice(-8)}</span>
                        <span>Uploaded: {formatTimeAgo(file.uploadedAt)}</span>
                        <span>Downloads: {file.downloadCount}</span>
                        <span className="flex items-center space-x-1">
                          <span>🔗</span>
                          <span>Block #{file.blockHeight}</span>
                        </span>
                        <span className="flex items-center space-x-1">
                          <span>📋</span>
                          <span className="font-mono text-xs">{file.txHash.slice(0, 8)}...{file.txHash.slice(-8)}</span>
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="flex space-x-2">
                    <Button 
                      size="sm" 
                      variant="outline"
                      onClick={() => handleDownload(file)}
                      disabled={downloadingFiles.has(file.id) || file.status === "unavailable"}
                    >
                      {downloadingFiles.has(file.id) ? (
                        <div className="flex items-center space-x-2">
                          <div className="w-4 h-4 border-2 border-gray-300 border-t-blue-600 rounded-full animate-spin"></div>
                          <span>{downloadProgress[file.id] || 0}%</span>
                        </div>
                      ) : (
                        "Download"
                      )}
                    </Button>
                    <Button 
                      size="sm" 
                      variant="outline"
                      onClick={() => handleShowDetails(file)}
                    >
                      Details
                    </Button>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <Card className="p-4">
          <div className="flex items-center justify-between">
            <div className="text-sm text-gray-500 dark:text-gray-400">
              Showing {startIndex + 1}-{Math.min(endIndex, filteredFiles.length)} of {filteredFiles.length} files
            </div>
            <div className="flex items-center space-x-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setCurrentPage(prev => Math.max(1, prev - 1))}
                disabled={currentPage === 1}
              >
                ← Previous
              </Button>
              <div className="flex items-center space-x-1">
                {Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
                  const pageNum = currentPage <= 3 ? i + 1 : currentPage - 2 + i
                  if (pageNum > totalPages) return null
                  return (
                    <Button
                      key={pageNum}
                      variant={currentPage === pageNum ? "default" : "outline"}
                      size="sm"
                      onClick={() => setCurrentPage(pageNum)}
                    >
                      {pageNum}
                    </Button>
                  )
                })}
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setCurrentPage(prev => Math.min(totalPages, prev + 1))}
                disabled={currentPage === totalPages}
              >
                Next →
              </Button>
            </div>
          </div>
        </Card>
      )}

      {/* Summary Stats */}
      <Card className="p-4">
        <h3 className="font-medium mb-3">Network File Statistics</h3>
        <div className="grid grid-cols-2 md:grid-cols-5 gap-4 text-sm">
          <div>
            <div className="text-gray-500 dark:text-gray-400">Total Files</div>
            <div className="font-medium">{filteredFiles.length}</div>
          </div>
          <div>
            <div className="text-gray-500 dark:text-gray-400">Available</div>
            <div className="font-medium text-green-600">
              {filteredFiles.filter(f => f.status === "available").length}
            </div>
          </div>
          <div>
            <div className="text-gray-500 dark:text-gray-400">Degraded</div>
            <div className="font-medium text-yellow-600">
              {filteredFiles.filter(f => f.status === "degraded").length}
            </div>
          </div>
          <div>
            <div className="text-gray-500 dark:text-gray-400">Unavailable</div>
            <div className="font-medium text-red-600">
              {filteredFiles.filter(f => f.status === "unavailable").length}
            </div>
          </div>
          <div>
            <div className="text-gray-500 dark:text-gray-400">Public Files</div>
            <div className="font-medium text-blue-600">
              {filteredFiles.filter(f => f.isPublic).length}
            </div>
          </div>
        </div>
      </Card>

      {/* File Details Modal */}
      {showDetailsModal && selectedFile && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
            <div className="flex justify-between items-center mb-4">
              <h2 className="text-xl font-bold">File Details</h2>
              <Button variant="outline" onClick={handleCloseModal}>
                ✕
              </Button>
            </div>
            
            <div className="space-y-4">
              {/* File Header */}
              <div className="flex items-start space-x-4">
                <span className="text-4xl">{getFileIcon(selectedFile.mimeType)}</span>
                <div className="flex-1">
                  <h3 className="text-lg font-medium">{selectedFile.name}</h3>
                  <p className="text-gray-600 dark:text-gray-400">{formatFileSize(selectedFile.size)}</p>
                  <div className="flex items-center space-x-2 mt-2">
                    <Badge className={getStatusColor(selectedFile.status)}>
                      {selectedFile.status}
                    </Badge>
                    <Badge variant="outline">
                      {selectedFile.availableReplicas}/{selectedFile.replicationFactor} replicas
                    </Badge>
                    {selectedFile.isPublic ? (
                      <Badge variant="secondary">Public</Badge>
                    ) : (
                      <Badge variant="outline">Private</Badge>
                    )}
                  </div>
                </div>
              </div>

              {/* Blockchain Information */}
              <div className="border-t pt-4">
                <h4 className="font-medium mb-3">🔗 Blockchain Information</h4>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">Transaction Hash:</span>
                    <div className="font-mono text-xs bg-gray-100 dark:bg-gray-700 p-2 rounded mt-1">
                      {selectedFile.txHash}
                    </div>
                  </div>
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">Block Height:</span>
                    <div className="font-mono">#{selectedFile.blockHeight.toLocaleString()}</div>
                  </div>
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">File Hash:</span>
                    <div className="font-mono text-xs bg-gray-100 dark:bg-gray-700 p-2 rounded mt-1">
                      {selectedFile.hash}
                    </div>
                  </div>
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">Uploaded:</span>
                    <div>{new Date(selectedFile.uploadedAt).toLocaleString()}</div>
                  </div>
                </div>
              </div>

              {/* File Information */}
              <div className="border-t pt-4">
                <h4 className="font-medium mb-3">📁 File Information</h4>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">MIME Type:</span>
                    <div>{selectedFile.mimeType}</div>
                  </div>
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">Downloads:</span>
                    <div>{selectedFile.downloadCount.toLocaleString()}</div>
                  </div>
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">Owner:</span>
                    <div className="font-mono text-xs">{selectedFile.owner}</div>
                  </div>
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">Last Accessed:</span>
                    <div>{selectedFile.lastAccessed ? new Date(selectedFile.lastAccessed).toLocaleString() : 'Never'}</div>
                  </div>
                </div>
              </div>

              {/* Storage Nodes */}
              <div className="border-t pt-4">
                <h4 className="font-medium mb-3">🖥️ Storage Nodes</h4>
                <div className="space-y-2">
                  {selectedFile.storageNodes.length > 0 ? (
                    selectedFile.storageNodes.map((node, index) => (
                      <div key={index} className="flex items-center space-x-2 text-sm">
                        <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                        <span className="font-mono">{node}</span>
                      </div>
                    ))
                  ) : (
                    <div className="text-gray-500 dark:text-gray-400 text-sm">No storage nodes available</div>
                  )}
                </div>
              </div>

              {/* Tags */}
              {selectedFile.tags.length > 0 && (
                <div className="border-t pt-4">
                  <h4 className="font-medium mb-3">🏷️ Tags</h4>
                  <div className="flex flex-wrap gap-2">
                    {selectedFile.tags.map((tag, index) => (
                      <Badge key={index} variant="secondary">
                        {tag}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}

              {/* Actions */}
              <div className="border-t pt-4 flex space-x-3">
                <Button 
                  onClick={() => handleDownload(selectedFile)}
                  disabled={downloadingFiles.has(selectedFile.id) || selectedFile.status === "unavailable"}
                  className="flex-1"
                >
                  {downloadingFiles.has(selectedFile.id) ? (
                    <div className="flex items-center space-x-2">
                      <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
                      <span>Downloading... {downloadProgress[selectedFile.id] || 0}%</span>
                    </div>
                  ) : (
                    "Download File"
                  )}
                </Button>
                <Button variant="outline" onClick={handleCloseModal}>
                  Close
                </Button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
