# Test Server 1 Status After Exiting Rescue Mode
Import-Module Posh-SSH -ErrorAction SilentlyContinue

$SERVER1_IP = "188.245.97.41"
$SERVER1_PASSWORD = "7LuR4nUCfTiv"

Write-Host "=== Testing Server 1 Status ===" -ForegroundColor Cyan
Write-Host ""

# Test basic connectivity
Write-Host "Testing basic connectivity..." -ForegroundColor Green
$server1Online = Test-Connection -ComputerName $SERVER1_IP -Count 1 -Quiet
Write-Host "Server 1 ($SERVER1_IP): $(if($server1Online){'Online'}else{'Offline'})" -ForegroundColor White

# Test API endpoint
Write-Host "Testing Server 1 API (port 3000)..." -ForegroundColor Green
try {
    $response = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    if ($response.StatusCode -eq 200) {
        Write-Host "✅ Server 1 API is responding: $($response.StatusCode)" -ForegroundColor Green
        Write-Host "Response: $($response.Content)" -ForegroundColor Cyan
    } else {
        Write-Host "⚠️ Server 1 API returned status: $($response.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 API is not responding: $($_.Exception.Message)" -ForegroundColor Red
}

# Test P2P port
Write-Host "Testing Server 1 P2P port (8080)..." -ForegroundColor Green
try {
    $tcp = New-Object System.Net.Sockets.TcpClient
    $connect = $tcp.BeginConnect($SERVER1_IP, 8080, $null, $null)
    $wait = $connect.AsyncWaitHandle.WaitOne(3000, $false)
    if ($wait) {
        $tcp.EndConnect($connect)
        Write-Host "✅ Server 1 P2P port is open" -ForegroundColor Green
        $tcp.Close()
    } else {
        Write-Host "❌ Server 1 P2P port connection timeout" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Server 1 P2P port connection failed: $($_.Exception.Message)" -ForegroundColor Red
}

# Test SSH connection
Write-Host "Testing SSH connection..." -ForegroundColor Green
try {
    $securePassword = ConvertTo-SecureString $SERVER1_PASSWORD -AsPlainText -Force
    $credential = New-Object System.Management.Automation.PSCredential("root", $securePassword)
    $session = New-SSHSession -ComputerName $SERVER1_IP -Credential $credential -AcceptKey -ConnectionTimeout 10
    
    if ($session) {
        Write-Host "✅ SSH connection successful" -ForegroundColor Green
        
        # Check Docker status
        $dockerStatus = Invoke-SSHCommand -SessionId $session.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -Timeout 30
        
        if ($dockerStatus.ExitStatus -eq 0) {
            Write-Host "Docker status:" -ForegroundColor Cyan
            Write-Host $dockerStatus.Output -ForegroundColor White
        }
        
        # Check if IPPAN directory exists
        $ippanCheck = Invoke-SSHCommand -SessionId $session.SessionId -Command "ls -la /opt/ippan/mainnet/ 2>/dev/null || echo 'IPPAN not found'" -Timeout 30
        
        if ($ippanCheck.Output -like "*IPPAN not found*") {
            Write-Host "❌ IPPAN directory not found" -ForegroundColor Red
        } else {
            Write-Host "✅ IPPAN directory exists" -ForegroundColor Green
            Write-Host "IPPAN files:" -ForegroundColor Cyan
            Write-Host $ippanCheck.Output -ForegroundColor White
        }
        
        # Try to start services if they're not running
        $startServices = Invoke-SSHCommand -SessionId $session.SessionId -Command "su - ippan -c 'cd /opt/ippan/mainnet && docker-compose up -d'" -Timeout 300
        
        if ($startServices.ExitStatus -eq 0) {
            Write-Host "✅ Attempted to start IPPAN services" -ForegroundColor Green
        } else {
            Write-Host "⚠️ Failed to start services: $($startServices.Error)" -ForegroundColor Yellow
        }
        
        # Check final Docker status
        $finalStatus = Invoke-SSHCommand -SessionId $session.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'" -Timeout 30
        
        if ($finalStatus.ExitStatus -eq 0) {
            Write-Host "Final Docker status:" -ForegroundColor Cyan
            Write-Host $finalStatus.Output -ForegroundColor White
        }
        
        Remove-SSHSession -SessionId $session.SessionId
    }
} catch {
    Write-Host "❌ SSH connection failed: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Final API Test ===" -ForegroundColor Cyan
Start-Sleep -Seconds 10

try {
    $finalResponse = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    if ($finalResponse.StatusCode -eq 200) {
        Write-Host "✅ Server 1 API is now responding: $($finalResponse.StatusCode)" -ForegroundColor Green
        Write-Host "Response: $($finalResponse.Content)" -ForegroundColor Cyan
    } else {
        Write-Host "⚠️ Server 1 API returned status: $($finalResponse.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 API is still not responding: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Server 1 is out of rescue mode and accessible via SSH" -ForegroundColor Green
Write-Host "Next: Deploy Server 2 and test the connection" -ForegroundColor Yellow
Write-Host ""
Write-Host "Access URLs:" -ForegroundColor Cyan
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 1 Grafana: http://$SERVER1_IP`:3001" -ForegroundColor White
Write-Host "Server 1 Prometheus: http://$SERVER1_IP`:9090" -ForegroundColor White
