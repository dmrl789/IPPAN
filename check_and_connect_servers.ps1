# IPPAN Server Status Check and Connection Script
# This script checks the status of server1 and server2 and establishes connections

# Colors for output
$Red = "Red"
$Green = "Green"
$Yellow = "Yellow"
$Blue = "Blue"
$Cyan = "Cyan"

# Server configuration
$SERVER1_IP = "188.245.97.41"    # Nuremberg (Node 1)
$SERVER2_IP = "135.181.145.174"  # Helsinki (Node 2)
$IPPAN_USER = "ippan"

# Function to print colored output
function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor $Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor $Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor $Red
}

function Write-Header {
    param([string]$Message)
    Write-Host "[HEADER] $Message" -ForegroundColor $Blue
}

function Write-Section {
    param([string]$Message)
    Write-Host "`n=== $Message ===" -ForegroundColor $Cyan
}

# Function to test basic connectivity
function Test-ServerConnectivity {
    param([string]$ServerIP, [string]$ServerName)
    
    Write-Header "🔍 Testing $ServerName Connectivity ($ServerIP)"
    
    # Test ping
    Write-Status "Testing ping connectivity..."
    $pingResult = Test-Connection -ComputerName $ServerIP -Count 2 -Quiet
    if ($pingResult) {
        Write-Status "✅ $ServerName is reachable via ping"
    } else {
        Write-Error "❌ $ServerName is not reachable via ping"
        return $false
    }
    
    # Test SSH connectivity
    Write-Status "Testing SSH connectivity..."
    try {
        $sshTest = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -o BatchMode=yes $IPPAN_USER@$ServerIP "echo 'SSH connection successful'" 2>$null
        if ($sshTest -eq "SSH connection successful") {
            Write-Status "✅ $ServerName SSH connection successful"
        } else {
            Write-Warning "⚠️  $ServerName SSH connection failed or requires authentication"
        }
    } catch {
        Write-Warning "⚠️  $ServerName SSH connection failed: $_"
    }
    
    return $true
}

# Function to check server services
function Test-ServerServices {
    param([string]$ServerIP, [string]$ServerName)
    
    Write-Header "🔧 Checking $ServerName Services"
    
    # Test API endpoint
    Write-Status "Testing API endpoint (port 3000)..."
    try {
        $apiResponse = Invoke-WebRequest -Uri "http://$ServerIP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
        if ($apiResponse.StatusCode -eq 200) {
            Write-Status "✅ $ServerName API is responding"
            Write-Status "API Response: $($apiResponse.Content)"
        } else {
            Write-Warning "⚠️  $ServerName API returned status: $($apiResponse.StatusCode)"
        }
    } catch {
        Write-Warning "⚠️  $ServerName API is not responding: $_"
    }
    
    # Test P2P port
    Write-Status "Testing P2P port (8080)..."
    try {
        $tcpClient = New-Object System.Net.Sockets.TcpClient
        $connect = $tcpClient.BeginConnect($ServerIP, 8080, $null, $null)
        $wait = $connect.AsyncWaitHandle.WaitOne(5000, $false)
        if ($wait) {
            $tcpClient.EndConnect($connect)
            Write-Status "✅ $ServerName P2P port (8080) is open"
            $tcpClient.Close()
        } else {
            Write-Warning "⚠️  $ServerName P2P port (8080) connection timeout"
        }
    } catch {
        Write-Warning "⚠️  $ServerName P2P port (8080) connection failed: $_"
    }
    
    # Test Prometheus metrics port
    Write-Status "Testing Prometheus metrics port (9090)..."
    try {
        $metricsResponse = Invoke-WebRequest -Uri "http://$ServerIP`:9090/metrics" -TimeoutSec 10 -UseBasicParsing 2>$null
        if ($metricsResponse.StatusCode -eq 200) {
            Write-Status "✅ $ServerName Prometheus metrics are available"
        } else {
            Write-Warning "⚠️  $ServerName Prometheus metrics returned status: $($metricsResponse.StatusCode)"
        }
    } catch {
        Write-Warning "⚠️  $ServerName Prometheus metrics are not available: $_"
    }
    
    # Test Grafana dashboard port
    Write-Status "Testing Grafana dashboard port (3001)..."
    try {
        $grafanaResponse = Invoke-WebRequest -Uri "http://$ServerIP`:3001" -TimeoutSec 10 -UseBasicParsing 2>$null
        if ($grafanaResponse.StatusCode -eq 200) {
            Write-Status "✅ $ServerName Grafana dashboard is accessible"
        } else {
            Write-Warning "⚠️  $ServerName Grafana dashboard returned status: $($grafanaResponse.StatusCode)"
        }
    } catch {
        Write-Warning "⚠️  $ServerName Grafana dashboard is not accessible: $_"
    }
}

# Function to check Docker containers on server
function Test-ServerDockerContainers {
    param([string]$ServerIP, [string]$ServerName)
    
    Write-Header "🐳 Checking $ServerName Docker Containers"
    
    try {
        $dockerPs = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$ServerIP "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" 2>$null
        if ($dockerPs) {
            Write-Status "✅ $ServerName Docker containers status:"
            Write-Host $dockerPs
        } else {
            Write-Warning "⚠️  $ServerName No IPPAN Docker containers found or SSH access failed"
        }
    } catch {
        Write-Warning "⚠️  $ServerName Failed to check Docker containers: $_"
    }
}

# Function to check network peer connections
function Test-NetworkPeerConnections {
    param([string]$ServerIP, [string]$ServerName)
    
    Write-Header "🔗 Checking $ServerName Network Peer Connections"
    
    # Check peer list via API
    Write-Status "Checking peer list via API..."
    try {
        $peersResponse = Invoke-WebRequest -Uri "http://$ServerIP`:3000/api/v1/network/peers" -TimeoutSec 10 -UseBasicParsing 2>$null
        if ($peersResponse.StatusCode -eq 200) {
            Write-Status "✅ $ServerName Peer list retrieved successfully"
            $peersData = $peersResponse.Content | ConvertFrom-Json
            Write-Status "Peer count: $($peersData.peers.Count)"
            foreach ($peer in $peersData.peers) {
                Write-Status "  - Peer: $($peer.address) (Status: $($peer.status))"
            }
        } else {
            Write-Warning "⚠️  $ServerName Failed to retrieve peer list: $($peersResponse.StatusCode)"
        }
    } catch {
        Write-Warning "⚠️  $ServerName Peer list API not available: $_"
    }
    
    # Check blockchain status
    Write-Status "Checking blockchain status..."
    try {
        $blockchainResponse = Invoke-WebRequest -Uri "http://$ServerIP`:3000/api/v1/blockchain/status" -TimeoutSec 10 -UseBasicParsing 2>$null
        if ($blockchainResponse.StatusCode -eq 200) {
            Write-Status "✅ $ServerName Blockchain status retrieved"
            $blockchainData = $blockchainResponse.Content | ConvertFrom-Json
            Write-Status "Block height: $($blockchainData.block_height)"
            Write-Status "Network status: $($blockchainData.network_status)"
        } else {
            Write-Warning "⚠️  $ServerName Failed to retrieve blockchain status: $($blockchainResponse.StatusCode)"
        }
    } catch {
        Write-Warning "⚠️  $ServerName Blockchain status API not available: $_"
    }
}

# Function to test inter-server connectivity
function Test-InterServerConnectivity {
    Write-Header "🔗 Testing Inter-Server Connectivity"
    
    # Test server1 to server2 connectivity
    Write-Status "Testing Server1 to Server2 connectivity..."
    try {
        $server1ToServer2 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER1_IP "timeout 10 bash -c '</dev/tcp/$SERVER2_IP/8080' && echo 'Server2 P2P reachable from Server1'" 2>$null
        if ($server1ToServer2 -eq "Server2 P2P reachable from Server1") {
            Write-Status "✅ Server1 can reach Server2 P2P port"
        } else {
            Write-Warning "⚠️  Server1 cannot reach Server2 P2P port"
        }
    } catch {
        Write-Warning "⚠️  Failed to test Server1 to Server2 connectivity: $_"
    }
    
    # Test server2 to server1 connectivity
    Write-Status "Testing Server2 to Server1 connectivity..."
    try {
        $server2ToServer1 = ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$SERVER2_IP "timeout 10 bash -c '</dev/tcp/$SERVER1_IP/8080' && echo 'Server1 P2P reachable from Server2'" 2>$null
        if ($server2ToServer1 -eq "Server1 P2P reachable from Server2") {
            Write-Status "✅ Server2 can reach Server1 P2P port"
        } else {
            Write-Warning "⚠️  Server2 cannot reach Server1 P2P port"
        }
    } catch {
        Write-Warning "⚠️  Failed to test Server2 to Server1 connectivity: $_"
    }
}

# Function to restart services if needed
function Restart-ServerServices {
    param([string]$ServerIP, [string]$ServerName)
    
    Write-Header "🔄 Restarting $ServerName Services"
    
    try {
        Write-Status "Stopping IPPAN services..."
        ssh -o ConnectTimeout=30 -o StrictHostKeyChecking=no $IPPAN_USER@$ServerIP "cd /opt/ippan/mainnet && docker-compose down" 2>$null
        
        Write-Status "Starting IPPAN services..."
        ssh -o ConnectTimeout=30 -o StrictHostKeyChecking=no $IPPAN_USER@$ServerIP "cd /opt/ippan/mainnet && docker-compose up -d" 2>$null
        
        Write-Status "Waiting for services to start..."
        Start-Sleep -Seconds 30
        
        Write-Status "Checking service status..."
        ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no $IPPAN_USER@$ServerIP "cd /opt/ippan/mainnet && docker-compose ps" 2>$null
        
        Write-Status "✅ $ServerName services restarted"
    } catch {
        Write-Error "❌ Failed to restart $ServerName services: $_"
    }
}

# Function to create connection verification report
function New-ConnectionReport {
    Write-Header "📊 Connection Verification Report"
    
    $report = @"
IPPAN Multi-Node Connection Report
Generated: $(Get-Date)
=====================================

Server Configuration:
Server 1 (Nuremberg): $SERVER1_IP
Server 2 (Helsinki): $SERVER2_IP
User: $IPPAN_USER

Access URLs:
Server 1 API: http://$SERVER1_IP`:3000
Server 1 Grafana: http://$SERVER1_IP`:3001
Server 1 Prometheus: http://$SERVER1_IP`:9090
Server 2 API: http://$SERVER2_IP`:3000
Server 2 Grafana: http://$SERVER2_IP`:3001
Server 2 Prometheus: http://$SERVER2_IP`:9090

Network Ports:
P2P Network: 8080
API: 3000
Prometheus: 9090
Grafana: 3001

Next Steps:
1. Monitor logs: docker-compose logs -f
2. Check consensus participation via API endpoints
3. Verify blockchain synchronization
4. Test transaction processing
"@
    
    Write-Host $report
    $report | Out-File -FilePath "ippan_connection_report.txt" -Encoding UTF8
    Write-Status "Report saved to: ippan_connection_report.txt"
}

# Main execution
Write-Section "IPPAN Server Status Check and Connection Script"
Write-Header "🚀 Starting comprehensive server check and connection process"
Write-Host "Server 1 (Nuremberg): $SERVER1_IP"
Write-Host "Server 2 (Helsinki): $SERVER2_IP"
Write-Host "================================================"

# Check server connectivity
$server1Reachable = Test-ServerConnectivity -ServerIP $SERVER1_IP -ServerName "Server1"
$server2Reachable = Test-ServerConnectivity -ServerIP $SERVER2_IP -ServerName "Server2"

if (-not $server1Reachable -and -not $server2Reachable) {
    Write-Error "❌ Neither server is reachable. Please check network connectivity and server status."
    exit 1
}

# Check services on each server
if ($server1Reachable) {
    Test-ServerServices -ServerIP $SERVER1_IP -ServerName "Server1"
    Test-ServerDockerContainers -ServerIP $SERVER1_IP -ServerName "Server1"
    Test-NetworkPeerConnections -ServerIP $SERVER1_IP -ServerName "Server1"
}

if ($server2Reachable) {
    Test-ServerServices -ServerIP $SERVER2_IP -ServerName "Server2"
    Test-ServerDockerContainers -ServerIP $SERVER2_IP -ServerName "Server2"
    Test-NetworkPeerConnections -ServerIP $SERVER2_IP -ServerName "Server2"
}

# Test inter-server connectivity
if ($server1Reachable -and $server2Reachable) {
    Test-InterServerConnectivity
}

# Ask user if they want to restart services
Write-Section "Service Management"
$restartChoice = Read-Host "Do you want to restart services on both servers? (y/n)"
if ($restartChoice -eq "y" -or $restartChoice -eq "Y") {
    if ($server1Reachable) {
        Restart-ServerServices -ServerIP $SERVER1_IP -ServerName "Server1"
    }
    if ($server2Reachable) {
        Restart-ServerServices -ServerIP $SERVER2_IP -ServerName "Server2"
    }
    
    Write-Status "Waiting for services to stabilize..."
    Start-Sleep -Seconds 60
    
    # Re-check services after restart
    if ($server1Reachable) {
        Test-ServerServices -ServerIP $SERVER1_IP -ServerName "Server1"
    }
    if ($server2Reachable) {
        Test-ServerServices -ServerIP $SERVER2_IP -ServerName "Server2"
    }
}

# Generate connection report
New-ConnectionReport

Write-Section "Check Complete"
Write-Status "🎉 Server status check and connection verification complete!"
Write-Status "Review the report above and check the generated file: ippan_connection_report.txt"
Write-Status ""
Write-Status "If services are not running properly, you can:"
Write-Status "1. Check Docker logs: ssh $IPPAN_USER@<server_ip> 'cd /opt/ippan/mainnet && docker-compose logs'"
Write-Status "2. Restart services: ssh $IPPAN_USER@<server_ip> 'cd /opt/ippan/mainnet && docker-compose restart'"
Write-Status "3. Check system resources: ssh $IPPAN_USER@<server_ip> 'docker stats'"
