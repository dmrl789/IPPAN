# Deploy IPPAN via Console Session
# This script will help you deploy by providing commands to copy-paste

Write-Host "=== IPPAN DEPLOYMENT VIA CONSOLE ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Since you're already connected to the server console," -ForegroundColor Green
Write-Host "I'll provide you with the exact commands to copy and paste." -ForegroundColor Green
Write-Host ""

# Read the deployment commands
$deployCommands = Get-Content "server1_deploy_commands.txt" -Raw

Write-Host "=== COPY THESE COMMANDS TO YOUR SERVER CONSOLE ===" -ForegroundColor Yellow
Write-Host ""
Write-Host $deployCommands -ForegroundColor White
Write-Host ""
Write-Host "=== END OF COMMANDS ===" -ForegroundColor Yellow
Write-Host ""

Write-Host "Instructions:" -ForegroundColor Cyan
Write-Host "1. Select ALL the text above (from '=== COPY THESE COMMANDS' to '=== END OF COMMANDS')" -ForegroundColor White
Write-Host "2. Copy it (Ctrl+C)" -ForegroundColor White
Write-Host "3. Paste it into your server console (Ctrl+V)" -ForegroundColor White
Write-Host "4. Press Enter to execute" -ForegroundColor White
Write-Host ""
Write-Host "The deployment will take 5-10 minutes to complete." -ForegroundColor Green
Write-Host ""

# Wait for user to complete deployment
Write-Host "Press any key when you've completed the deployment on Server 1..." -ForegroundColor Yellow
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

Write-Host ""
Write-Host "=== TESTING SERVER 1 ===" -ForegroundColor Cyan

# Test Server 1 API
try {
    $api1 = Invoke-WebRequest -Uri "http://188.245.97.41:3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
    if ($api1.StatusCode -eq 200) {
        Write-Host "✅ Server 1 API is responding: $($api1.StatusCode)" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 1 API returned status: $($api1.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 API is not responding yet" -ForegroundColor Red
}

# Test Server 1 Metrics
try {
    $metrics1 = Invoke-WebRequest -Uri "http://188.245.97.41:9090" -TimeoutSec 10 -UseBasicParsing 2>$null
    if ($metrics1.StatusCode -eq 200) {
        Write-Host "✅ Server 1 Metrics are responding: $($metrics1.StatusCode)" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Server 1 Metrics returned status: $($metrics1.StatusCode)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ Server 1 Metrics are not responding yet" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== SERVER 2 DEPLOYMENT ===" -ForegroundColor Cyan
Write-Host "Now let's deploy Server 2..." -ForegroundColor Green
Write-Host ""

# Read Server 2 deployment commands
$deployCommands2 = Get-Content "server2_deploy_commands.txt" -Raw

Write-Host "=== COPY THESE COMMANDS FOR SERVER 2 ===" -ForegroundColor Yellow
Write-Host ""
Write-Host $deployCommands2 -ForegroundColor White
Write-Host ""
Write-Host "=== END OF SERVER 2 COMMANDS ===" -ForegroundColor Yellow
Write-Host ""

Write-Host "Instructions for Server 2:" -ForegroundColor Cyan
Write-Host "1. Connect to Server 2 console (135.181.145.174)" -ForegroundColor White
Write-Host "2. Copy the commands above" -ForegroundColor White
Write-Host "3. Paste and execute them" -ForegroundColor White
Write-Host "4. Wait 5-10 minutes for completion" -ForegroundColor White
Write-Host ""

Write-Host "Press any key when you've completed the deployment on Server 2..." -ForegroundColor Yellow
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

Write-Host ""
Write-Host "=== TESTING BOTH SERVERS ===" -ForegroundColor Cyan

# Test both servers
$servers = @(
    @{IP="188.245.97.41"; Name="Server 1"},
    @{IP="135.181.145.174"; Name="Server 2"}
)

foreach ($server in $servers) {
    Write-Host "Testing $($server.Name) ($($server.IP))..." -ForegroundColor Green
    
    # Test API
    try {
        $api = Invoke-WebRequest -Uri "http://$($server.IP):3000/health" -TimeoutSec 10 -UseBasicParsing 2>$null
        if ($api.StatusCode -eq 200) {
            Write-Host "✅ $($server.Name) API: $($api.StatusCode)" -ForegroundColor Green
        } else {
            Write-Host "⚠️ $($server.Name) API: $($api.StatusCode)" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "❌ $($server.Name) API: Not responding" -ForegroundColor Red
    }
    
    # Test Metrics
    try {
        $metrics = Invoke-WebRequest -Uri "http://$($server.IP):9090" -TimeoutSec 10 -UseBasicParsing 2>$null
        if ($metrics.StatusCode -eq 200) {
            Write-Host "✅ $($server.Name) Metrics: $($metrics.StatusCode)" -ForegroundColor Green
        } else {
            Write-Host "⚠️ $($server.Name) Metrics: $($metrics.StatusCode)" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "❌ $($server.Name) Metrics: Not responding" -ForegroundColor Red
    }
    
    Write-Host ""
}

Write-Host "=== DEPLOYMENT COMPLETE ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "🎉 IPPAN Blockchain Network Status:" -ForegroundColor Green
Write-Host ""
Write-Host "Access URLs:" -ForegroundColor Cyan
Write-Host "- Server 1 API: http://188.245.97.41:3000" -ForegroundColor White
Write-Host "- Server 1 Metrics: http://188.245.97.41:9090" -ForegroundColor White
Write-Host "- Server 2 API: http://135.181.145.174:3000" -ForegroundColor White
Write-Host "- Server 2 Metrics: http://135.181.145.174:9090" -ForegroundColor White
Write-Host ""
Write-Host "P2P Network:" -ForegroundColor Cyan
Write-Host "- Node 1: 188.245.97.41:8080" -ForegroundColor White
Write-Host "- Node 2: 135.181.145.174:8080" -ForegroundColor White
Write-Host ""
Write-Host "Your IPPAN blockchain network is now operational!" -ForegroundColor Green
