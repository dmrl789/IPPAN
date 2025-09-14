# Quick IPPAN Multi-Node Connectivity Test
# This script performs a basic connectivity test between the two servers

# Server configuration
$SERVER1_IP = "188.245.97.41"    # Nuremberg
$SERVER2_IP = "135.181.145.174"  # Helsinki
$IPPAN_PORT = 8080
$API_PORT = 3000

function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Write-Header {
    param([string]$Message)
    Write-Host "[HEADER] $Message" -ForegroundColor Blue
}

function Test-TCPConnection {
    param(
        [string]$ComputerName,
        [int]$Port,
        [int]$TimeoutMs = 10000
    )
    
    try {
        $tcpClient = New-Object System.Net.Sockets.TcpClient
        $connect = $tcpClient.BeginConnect($ComputerName, $Port, $null, $null)
        $wait = $connect.AsyncWaitHandle.WaitOne($TimeoutMs, $false)
        
        if ($wait) {
            $tcpClient.EndConnect($connect)
            $tcpClient.Close()
            return $true
        } else {
            $tcpClient.Close()
            return $false
        }
    } catch {
        return $false
    }
}

function Test-HTTPEndpoint {
    param(
        [string]$Url,
        [int]$TimeoutSec = 10
    )
    
    try {
        $response = Invoke-WebRequest -Uri $Url -TimeoutSec $TimeoutSec -UseBasicParsing
        return $response.StatusCode -eq 200
    } catch {
        return $false
    }
}

Write-Header "🚀 IPPAN Multi-Node Quick Connectivity Test"
Write-Host "Server 1 (Nuremberg): $SERVER1_IP"
Write-Host "Server 2 (Helsinki): $SERVER2_IP"
Write-Host "================================================"

# Test Server 1
Write-Header "Testing Server 1 (Nuremberg)"
Write-Status "Testing P2P port (8080)..."
if (Test-TCPConnection -ComputerName $SERVER1_IP -Port $IPPAN_PORT) {
    Write-Status "✅ Server 1 P2P port is open"
} else {
    Write-Error "❌ Server 1 P2P port is not accessible"
}

Write-Status "Testing API port (3000)..."
if (Test-TCPConnection -ComputerName $SERVER1_IP -Port $API_PORT) {
    Write-Status "✅ Server 1 API port is open"
} else {
    Write-Error "❌ Server 1 API port is not accessible"
}

Write-Status "Testing API endpoint..."
if (Test-HTTPEndpoint -Url "http://$SERVER1_IP`:$API_PORT/health") {
    Write-Status "✅ Server 1 API is responding"
} else {
    Write-Error "❌ Server 1 API is not responding"
}

Write-Host ""

# Test Server 2
Write-Header "Testing Server 2 (Helsinki)"
Write-Status "Testing P2P port (8080)..."
if (Test-TCPConnection -ComputerName $SERVER2_IP -Port $IPPAN_PORT) {
    Write-Status "✅ Server 2 P2P port is open"
} else {
    Write-Error "❌ Server 2 P2P port is not accessible"
}

Write-Status "Testing API port (3000)..."
if (Test-TCPConnection -ComputerName $SERVER2_IP -Port $API_PORT) {
    Write-Status "✅ Server 2 API port is open"
} else {
    Write-Error "❌ Server 2 API port is not accessible"
}

Write-Status "Testing API endpoint..."
if (Test-HTTPEndpoint -Url "http://$SERVER2_IP`:$API_PORT/health") {
    Write-Status "✅ Server 2 API is responding"
} else {
    Write-Error "❌ Server 2 API is not responding"
}

Write-Host ""

# Test inter-node connectivity
Write-Header "Testing Inter-Node Connectivity"
Write-Status "Testing Server 1 → Server 2 P2P..."
if (Test-TCPConnection -ComputerName $SERVER2_IP -Port $IPPAN_PORT) {
    Write-Status "✅ Server 1 can reach Server 2 P2P"
} else {
    Write-Error "❌ Server 1 cannot reach Server 2 P2P"
}

Write-Status "Testing Server 2 → Server 1 P2P..."
if (Test-TCPConnection -ComputerName $SERVER1_IP -Port $IPPAN_PORT) {
    Write-Status "✅ Server 2 can reach Server 1 P2P"
} else {
    Write-Error "❌ Server 2 cannot reach Server 1 P2P"
}

Write-Host ""
Write-Header "Test Complete!"
Write-Status "If all tests pass, the servers are ready for multi-node deployment"
Write-Status "If any tests fail, check firewall settings and service status"
