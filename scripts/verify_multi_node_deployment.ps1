# IPPAN Multi-Node Deployment Verification Script (PowerShell)
# This script verifies server2 deployment and establishes connection to server1

# Server configuration
$SERVER1_IP = "188.245.97.41"    # Nuremberg
$SERVER2_IP = "135.181.145.174"  # Helsinki
$IPPAN_USER = "ippan"
$IPPAN_PORT = 8080
$API_PORT = 3000
$METRICS_PORT = 9090

# Function to print colored output
function Write-Status {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Write-Header {
    param([string]$Message)
    Write-Host "[HEADER] $Message" -ForegroundColor Blue
}

# Function to test network connectivity
function Test-Connectivity {
    param(
        [string]$TargetIP,
        [int]$Port,
        [string]$ServiceName
    )
    
    Write-Status "Testing connectivity to $ServiceName at $TargetIP`:$Port"
    
    try {
        $tcpClient = New-Object System.Net.Sockets.TcpClient
        $connect = $tcpClient.BeginConnect($TargetIP, $Port, $null, $null)
        $wait = $connect.AsyncWaitHandle.WaitOne(10000, $false)
        
        if ($wait) {
            $tcpClient.EndConnect($connect)
            $tcpClient.Close()
            Write-Status "✅ $ServiceName is reachable at $TargetIP`:$Port"
            return $true
        } else {
            $tcpClient.Close()
            Write-Error "❌ $ServiceName is not reachable at $TargetIP`:$Port"
            return $false
        }
    } catch {
        Write-Error "❌ $ServiceName is not reachable at $TargetIP`:$Port"
        return $false
    }
}

# Function to check service status via API
function Test-ServiceStatus {
    param(
        [string]$ServerIP,
        [string]$ServerName
    )
    
    Write-Status "Checking $ServerName service status..."
    
    try {
        $response = Invoke-WebRequest -Uri "http://$ServerIP`:$API_PORT/health" -TimeoutSec 10 -UseBasicParsing
        if ($response.StatusCode -eq 200) {
            Write-Status "✅ $ServerName API is responding"
            
            # Get node info
            try {
                $nodeInfo = Invoke-WebRequest -Uri "http://$ServerIP`:$API_PORT/api/v1/node/info" -TimeoutSec 10 -UseBasicParsing
                Write-Status "Node info: $($nodeInfo.Content)"
            } catch {
                Write-Warning "Could not retrieve node info"
            }
            
            # Get network peers
            try {
                $peers = Invoke-WebRequest -Uri "http://$ServerIP`:$API_PORT/api/v1/network/peers" -TimeoutSec 10 -UseBasicParsing
                Write-Status "Connected peers: $($peers.Content)"
            } catch {
                Write-Warning "Could not retrieve peer list"
            }
            
            return $true
        }
    } catch {
        Write-Error "❌ $ServerName API is not responding"
        return $false
    }
}

# Function to test consensus participation
function Test-Consensus {
    param(
        [string]$ServerIP,
        [string]$ServerName
    )
    
    Write-Status "Testing consensus participation on $ServerName..."
    
    try {
        $blockInfo = Invoke-WebRequest -Uri "http://$ServerIP`:$API_PORT/api/v1/blockchain/latest" -TimeoutSec 10 -UseBasicParsing
        if ($blockInfo.StatusCode -eq 200 -and $blockInfo.Content -ne "" -and $blockInfo.Content -ne "null") {
            Write-Status "✅ $ServerName is participating in consensus"
            Write-Status "Latest block info: $($blockInfo.Content)"
            return $true
        } else {
            Write-Warning "⚠️  $ServerName consensus status unclear"
            return $false
        }
    } catch {
        Write-Warning "⚠️  $ServerName consensus status unclear"
        return $false
    }
}

# Function to check monitoring endpoints
function Test-Monitoring {
    param(
        [string]$ServerIP,
        [string]$ServerName
    )
    
    Write-Status "Checking monitoring endpoints on $ServerName..."
    
    # Check Prometheus metrics
    try {
        $response = Invoke-WebRequest -Uri "http://$ServerIP`:$METRICS_PORT/metrics" -TimeoutSec 10 -UseBasicParsing
        if ($response.StatusCode -eq 200) {
            Write-Status "✅ Prometheus metrics available on $ServerName"
        }
    } catch {
        Write-Warning "⚠️  Prometheus metrics not available on $ServerName"
    }
    
    # Check Grafana (if accessible)
    try {
        $response = Invoke-WebRequest -Uri "http://$ServerIP`:3001" -TimeoutSec 10 -UseBasicParsing
        if ($response.StatusCode -eq 200) {
            Write-Status "✅ Grafana dashboard accessible on $ServerName"
        }
    } catch {
        Write-Warning "⚠️  Grafana dashboard not accessible on $ServerName"
    }
}

# Main verification function
function Main {
    Write-Header "🚀 IPPAN Multi-Node Deployment Verification"
    Write-Host "Server 1 (Nuremberg): $SERVER1_IP"
    Write-Host "Server 2 (Helsinki): $SERVER2_IP"
    Write-Host "================================================"
    
    $server1_ok = $true
    $server2_ok = $true
    
    # Verify Server 1
    Write-Header "🔍 Verifying Server 1 (Nuremberg) - $SERVER1_IP"
    
    if (-not (Test-Connectivity -TargetIP $SERVER1_IP -Port $IPPAN_PORT -ServiceName "IPPAN P2P")) {
        $server1_ok = $false
    }
    
    if (-not (Test-Connectivity -TargetIP $SERVER1_IP -Port $API_PORT -ServiceName "IPPAN API")) {
        $server1_ok = $false
    }
    
    if (-not (Test-ServiceStatus -ServerIP $SERVER1_IP -ServerName "Server 1")) {
        $server1_ok = $false
    }
    
    Test-Monitoring -ServerIP $SERVER1_IP -ServerName "Server 1"
    
    Write-Host ""
    
    # Verify Server 2
    Write-Header "🔍 Verifying Server 2 (Helsinki) - $SERVER2_IP"
    
    if (-not (Test-Connectivity -TargetIP $SERVER2_IP -Port $IPPAN_PORT -ServiceName "IPPAN P2P")) {
        $server2_ok = $false
    }
    
    if (-not (Test-Connectivity -TargetIP $SERVER2_IP -Port $API_PORT -ServiceName "IPPAN API")) {
        $server2_ok = $false
    }
    
    if (-not (Test-ServiceStatus -ServerIP $SERVER2_IP -ServerName "Server 2")) {
        $server2_ok = $false
    }
    
    Test-Monitoring -ServerIP $SERVER2_IP -ServerName "Server 2"
    
    Write-Host ""
    
    # Test inter-node connectivity
    Write-Header "🔗 Testing Inter-Node Connectivity"
    
    Write-Status "Testing Server 1 → Server 2 connectivity"
    if (Test-Connectivity -TargetIP $SERVER2_IP -Port $IPPAN_PORT -ServiceName "Server 2 from Server 1") {
        Write-Status "✅ Server 1 can reach Server 2"
    } else {
        Write-Error "❌ Server 1 cannot reach Server 2"
        $server1_ok = $false
    }
    
    Write-Status "Testing Server 2 → Server 1 connectivity"
    if (Test-Connectivity -TargetIP $SERVER1_IP -Port $IPPAN_PORT -ServiceName "Server 1 from Server 2") {
        Write-Status "✅ Server 2 can reach Server 1"
    } else {
        Write-Error "❌ Server 2 cannot reach Server 1"
        $server2_ok = $false
    }
    
    Write-Host ""
    
    # Test consensus participation
    Write-Header "⛓️  Testing Consensus Participation"
    
    Test-Consensus -ServerIP $SERVER1_IP -ServerName "Server 1"
    Test-Consensus -ServerIP $SERVER2_IP -ServerName "Server 2"
    
    Write-Host ""
    
    # Final status report
    Write-Header "📊 Final Status Report"
    
    if ($server1_ok) {
        Write-Status "✅ Server 1 (Nuremberg) is operational"
    } else {
        Write-Error "❌ Server 1 (Nuremberg) has issues"
    }
    
    if ($server2_ok) {
        Write-Status "✅ Server 2 (Helsinki) is operational"
    } else {
        Write-Error "❌ Server 2 (Helsinki) has issues"
    }
    
    if ($server1_ok -and $server2_ok) {
        Write-Status "🎉 Multi-node IPPAN network is operational!"
        Write-Status "Both servers are connected and participating in consensus"
        
        Write-Host ""
        Write-Status "Access URLs:"
        Write-Host "  Server 1 API: http://$SERVER1_IP`:$API_PORT"
        Write-Host "  Server 2 API: http://$SERVER2_IP`:$API_PORT"
        Write-Host "  Server 1 Grafana: http://$SERVER1_IP`:3001"
        Write-Host "  Server 2 Grafana: http://$SERVER2_IP`:3001"
        Write-Host "  Server 1 Prometheus: http://$SERVER1_IP`:$METRICS_PORT"
        Write-Host "  Server 2 Prometheus: http://$SERVER2_IP`:$METRICS_PORT"
        
    } else {
        Write-Error "⚠️  Multi-node network has issues that need to be resolved"
        Write-Status "Please check the error messages above and fix the issues"
    }
}

# Run main function
Main
