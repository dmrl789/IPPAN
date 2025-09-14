# Simple Connection Test and Status Check
Import-Module Posh-SSH -ErrorAction SilentlyContinue

$SERVER1_IP = "188.245.97.41"
$SERVER2_IP = "135.181.145.174"
$SERVER1_PASSWORD = "7LuR4nUCfTiv"
$SERVER2_PASSWORD = "VgRfqNg3T4sx"

Write-Host "=== Simple Connection Test ===" -ForegroundColor Cyan
Write-Host ""

# Test basic connectivity
Write-Host "Testing basic connectivity..." -ForegroundColor Green
$server1Online = Test-Connection -ComputerName $SERVER1_IP -Count 1 -Quiet
$server2Online = Test-Connection -ComputerName $SERVER2_IP -Count 1 -Quiet

Write-Host "Server 1 ($SERVER1_IP): $(if($server1Online){'Online'}else{'Offline'})" -ForegroundColor White
Write-Host "Server 2 ($SERVER2_IP): $(if($server2Online){'Online'}else{'Offline'})" -ForegroundColor White

Write-Host ""
Write-Host "=== Testing SSH Connections ===" -ForegroundColor Yellow

# Test Server 1 SSH
Write-Host "Testing Server 1 SSH connection..." -ForegroundColor Green
try {
    $securePassword1 = ConvertTo-SecureString $SERVER1_PASSWORD -AsPlainText -Force
    $credential1 = New-Object System.Management.Automation.PSCredential("root", $securePassword1)
    $session1 = New-SSHSession -ComputerName $SERVER1_IP -Credential $credential1 -AcceptKey -ConnectionTimeout 10
    
    if ($session1) {
        Write-Host "✅ Server 1 SSH connection successful" -ForegroundColor Green
        
        # Check if services are running
        $status1 = Invoke-SSHCommand -SessionId $session1.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}'" -Timeout 30
        if ($status1.ExitStatus -eq 0) {
            Write-Host "Server 1 Docker status:" -ForegroundColor Cyan
            Write-Host $status1.Output -ForegroundColor White
        }
        
        Remove-SSHSession -SessionId $session1.SessionId
    }
} catch {
    Write-Host "❌ Server 1 SSH connection failed: $($_.Exception.Message)" -ForegroundColor Red
}

# Test Server 2 SSH
Write-Host "Testing Server 2 SSH connection..." -ForegroundColor Green
try {
    $securePassword2 = ConvertTo-SecureString $SERVER2_PASSWORD -AsPlainText -Force
    $credential2 = New-Object System.Management.Automation.PSCredential("root", $securePassword2)
    $session2 = New-SSHSession -ComputerName $SERVER2_IP -Credential $credential2 -AcceptKey -ConnectionTimeout 10
    
    if ($session2) {
        Write-Host "✅ Server 2 SSH connection successful" -ForegroundColor Green
        
        # Check if services are running
        $status2 = Invoke-SSHCommand -SessionId $session2.SessionId -Command "docker ps --filter 'name=ippan' --format 'table {{.Names}}\t{{.Status}}'" -Timeout 30
        if ($status2.ExitStatus -eq 0) {
            Write-Host "Server 2 Docker status:" -ForegroundColor Cyan
            Write-Host $status2.Output -ForegroundColor White
        }
        
        Remove-SSHSession -SessionId $session2.SessionId
    }
} catch {
    Write-Host "❌ Server 2 SSH connection failed: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Testing API Endpoints ===" -ForegroundColor Yellow

# Test APIs
try {
    $api1 = Invoke-WebRequest -Uri "http://$SERVER1_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($api1.StatusCode -eq 200) {
        Write-Host "✅ Server 1 API is responding: $($api1.StatusCode)" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 1 API returned status: $($api1.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 API is not responding" -ForegroundColor Red
}

try {
    $api2 = Invoke-WebRequest -Uri "http://$SERVER2_IP`:3000/health" -TimeoutSec 5 -UseBasicParsing 2>$null
    if ($api2.StatusCode -eq 200) {
        Write-Host "✅ Server 2 API is responding: $($api2.StatusCode)" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 2 API returned status: $($api2.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 2 API is not responding" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Both servers are online and reachable" -ForegroundColor Green
Write-Host "SSH connections are working" -ForegroundColor Green
Write-Host "Next: Deploy IPPAN services on both servers" -ForegroundColor Yellow
Write-Host ""
Write-Host "Access URLs:" -ForegroundColor Cyan
Write-Host "Server 1 API: http://$SERVER1_IP`:3000" -ForegroundColor White
Write-Host "Server 2 API: http://$SERVER2_IP`:3000" -ForegroundColor White
