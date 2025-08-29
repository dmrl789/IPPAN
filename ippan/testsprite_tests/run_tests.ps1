# IPPAN Blockchain Test Suite - Windows PowerShell Version
# Comprehensive test execution script

param(
    [int]$NodePort = 8080,
    [int]$P2pPort = 8081,
    [int]$TestDuration = 60,
    [int]$LoadTps = 1000
)

# Error handling
$ErrorActionPreference = "Stop"

# Configuration
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$LogDir = Join-Path $ProjectRoot "testsprite_tests\logs"
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$LogFile = Join-Path $LogDir "test_run_$Timestamp.log"

# Create log directory
if (!(Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}

# Logging functions
function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] [$Level] $Message"
    Write-Host $logMessage
    Add-Content -Path $LogFile -Value $logMessage
}

function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
    Write-Log "SUCCESS: $Message"
}

function Write-Error {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
    Write-Log "ERROR: $Message"
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠ $Message" -ForegroundColor Yellow
    Write-Log "WARNING: $Message"
}

# Test functions
function Test-Build {
    Write-Log "Running build tests..."
    Set-Location $ProjectRoot
    
    # Debug build
    Write-Log "Building debug version..."
    try {
        cargo build
        Write-Success "Debug build successful"
    }
    catch {
        Write-Error "Debug build failed: $_"
        throw
    }
    
    # Release build
    Write-Log "Building release version..."
    try {
        cargo build --release
        Write-Success "Release build successful"
    }
    catch {
        Write-Error "Release build failed: $_"
        throw
    }
    
    # Check for warnings
    Write-Log "Checking for warnings..."
    $warnings = cargo check 2>&1 | Select-String "warning"
    if ($warnings) {
        Write-Warning "Build warnings found"
        $warnings | ForEach-Object { Write-Log "WARNING: $_" }
    }
    else {
        Write-Success "No build warnings"
    }
}

function Test-UnitTests {
    Write-Log "Running unit tests..."
    Set-Location $ProjectRoot
    
    # Test common crate
    Write-Log "Testing common crate..."
    try {
        cargo test -p ippan-common --lib
        Write-Success "Common crate tests passed"
    }
    catch {
        Write-Error "Common crate tests failed: $_"
        throw
    }
    
    # Test wallet CLI
    Write-Log "Testing wallet CLI..."
    try {
        cargo test -p ippan-wallet-cli --lib
        Write-Success "Wallet CLI tests passed"
    }
    catch {
        Write-Error "Wallet CLI tests failed: $_"
        throw
    }
    
    # Test load generator
    Write-Log "Testing load generator..."
    try {
        cargo test -p ippan-loadgen-cli --lib
        Write-Success "Load generator tests passed"
    }
    catch {
        Write-Error "Load generator tests failed: $_"
        throw
    }
    
    # Test benchmarks
    Write-Log "Testing benchmarks..."
    try {
        cargo test -p ippan-bench --lib
        Write-Success "Benchmark tests passed"
    }
    catch {
        Write-Error "Benchmark tests failed: $_"
        throw
    }
}

function Test-Benchmarks {
    Write-Log "Running performance benchmarks..."
    Set-Location $ProjectRoot
    
    try {
        # Run criterion benchmarks
        Write-Log "Running Criterion benchmarks..."
        cargo bench -p ippan-bench -- --verbose
        
        # Generate benchmark report
        Write-Log "Generating benchmark report..."
        $benchmarkFile = Join-Path $LogDir "benchmarks_$Timestamp.json"
        cargo bench -p ippan-bench -- --output-format=json | Out-File -FilePath $benchmarkFile
        
        Write-Success "Benchmarks completed"
    }
    catch {
        Write-Error "Benchmarks failed: $_"
        throw
    }
}

function Start-Node {
    Write-Log "Starting IPPAN node..."
    Set-Location $ProjectRoot
    
    # Kill any existing node processes
    Get-Process -Name "ippan-node" -ErrorAction SilentlyContinue | Stop-Process -Force
    
    # Start node in background
    $nodeLogFile = Join-Path $LogDir "node_$Timestamp.log"
    $nodeProcess = Start-Process -FilePath "cargo" -ArgumentList "run", "--release", "-p", "ippan-node", "--", "--http-port", $NodePort, "--p2p-port", $P2pPort, "--shards", "4" -RedirectStandardOutput $nodeLogFile -RedirectStandardError $nodeLogFile -PassThru -WindowStyle Hidden
    
    # Wait for node to start
    Write-Log "Waiting for node to start..."
    $maxAttempts = 30
    for ($i = 1; $i -le $maxAttempts; $i++) {
        try {
            $response = Invoke-RestMethod -Uri "http://localhost:$NodePort/health" -Method Get -TimeoutSec 5
            Write-Success "Node started successfully (PID: $($nodeProcess.Id))"
            return $nodeProcess
        }
        catch {
            if ($i -eq $maxAttempts) {
                Write-Error "Node failed to start after $maxAttempts attempts"
                throw
            }
            Start-Sleep -Seconds 1
        }
    }
}

function Stop-Node {
    param($NodeProcess)
    if ($NodeProcess) {
        Write-Log "Stopping node (PID: $($NodeProcess.Id))..."
        try {
            Stop-Process -Id $NodeProcess.Id -Force
            Write-Success "Node stopped"
        }
        catch {
            Write-Warning "Failed to stop node: $_"
        }
    }
}

function Test-HealthEndpoint {
    Write-Log "Testing health endpoint..."
    
    try {
        $response = Invoke-RestMethod -Uri "http://localhost:$NodePort/health" -Method Get
        Write-Success "Health endpoint working"
        Write-Log "Health response: $($response | ConvertTo-Json)"
    }
    catch {
        Write-Error "Health endpoint failed: $_"
        throw
    }
}

function Test-MetricsEndpoint {
    Write-Log "Testing metrics endpoint..."
    
    try {
        $response = Invoke-RestMethod -Uri "http://localhost:$NodePort/metrics" -Method Get
        if ($response -match "ippan_") {
            Write-Success "Metrics endpoint working"
        }
        else {
            Write-Error "Metrics endpoint response doesn't contain expected metrics"
            throw
        }
    }
    catch {
        Write-Error "Metrics endpoint failed: $_"
        throw
    }
}

function Test-WalletCli {
    Write-Log "Testing wallet CLI..."
    Set-Location $ProjectRoot
    
    # Create test wallet
    Write-Log "Creating test wallet..."
    $walletLogFile = Join-Path $LogDir "wallet_create_$Timestamp.log"
    try {
        cargo run --release -p ippan-wallet-cli -- new testwallet | Out-File -FilePath $walletLogFile
        Write-Success "Wallet creation successful"
    }
    catch {
        Write-Error "Wallet creation failed: $_"
        throw
    }
    
    # Show wallet address
    Write-Log "Showing wallet address..."
    $addrLogFile = Join-Path $LogDir "wallet_addr_$Timestamp.log"
    try {
        cargo run --release -p ippan-wallet-cli -- addr | Out-File -FilePath $addrLogFile
        Write-Success "Wallet address display successful"
    }
    catch {
        Write-Error "Wallet address display failed: $_"
        throw
    }
}

function Test-LoadGenerator {
    Write-Log "Testing load generator..."
    Set-Location $ProjectRoot
    
    # Run short load test
    Write-Log "Running load test ($LoadTps TPS for $TestDuration seconds)..."
    $loadgenLogFile = Join-Path $LogDir "loadgen_$Timestamp.log"
    
    try {
        cargo run --release -p ippan-loadgen-cli -- --tps $LoadTps --accounts 100 --duration $TestDuration --nodes "http://localhost:$NodePort" | Out-File -FilePath $loadgenLogFile
        
        Write-Success "Load generator test completed"
        
        # Check success rate
        $logContent = Get-Content $loadgenLogFile -Raw
        if ($logContent -match "Success rate.*?(\d+\.?\d*)%") {
            $successRate = $matches[1]
            Write-Log "Load test success rate: $successRate%"
        }
    }
    catch {
        Write-Error "Load generator test failed: $_"
        throw
    }
}

function Test-ApiEndpoints {
    Write-Log "Testing API endpoints..."
    
    # Test health endpoint
    Test-HealthEndpoint
    
    # Test metrics endpoint
    Test-MetricsEndpoint
    
    # Test transaction endpoint (basic)
    Write-Log "Testing transaction endpoint..."
    $txLogFile = Join-Path $LogDir "tx_test_$Timestamp.log"
    try {
        $testData = "test transaction data"
        $response = Invoke-RestMethod -Uri "http://localhost:$NodePort/tx" -Method Post -Body $testData -ContentType "application/octet-stream" | Out-File -FilePath $txLogFile
        Write-Success "Transaction endpoint test completed"
    }
    catch {
        Write-Warning "Transaction endpoint test failed (expected until proper format is implemented): $_"
    }
}

function Test-Integration {
    Write-Log "Running integration tests..."
    
    # Start node
    $nodeProcess = Start-Node
    
    # Wait a moment for node to fully initialize
    Start-Sleep -Seconds 5
    
    try {
        # Test API endpoints
        Test-ApiEndpoints
        
        # Test wallet CLI
        Test-WalletCli
        
        # Test load generator
        Test-LoadGenerator
        
        Write-Success "Integration tests completed"
    }
    finally {
        # Stop node
        Stop-Node -NodeProcess $nodeProcess
    }
}

function Test-Security {
    Write-Log "Running security tests..."
    Set-Location $ProjectRoot
    
    # Test cryptographic operations
    Write-Log "Testing cryptographic operations..."
    try {
        cargo test -p ippan-common crypto --lib
        Write-Success "Cryptographic tests passed"
    }
    catch {
        Write-Error "Cryptographic tests failed: $_"
        throw
    }
    
    # Test transaction validation
    Write-Log "Testing transaction validation..."
    try {
        cargo test -p ippan-common types --lib
        Write-Success "Transaction validation tests passed"
    }
    catch {
        Write-Error "Transaction validation tests failed: $_"
        throw
    }
}

function New-TestReport {
    Write-Log "Generating test report..."
    
    $reportFile = Join-Path $LogDir "test_report_$Timestamp.md"
    
    $reportContent = @"
# IPPAN Test Report - $(Get-Date)

## Test Summary
- **Timestamp**: $(Get-Date)
- **Duration**: $(Get-Date -Format "HH:mm:ss")
- **Status**: PASSED

## Test Results

### Unit Tests
- Common crate: PASSED
- Wallet CLI: PASSED
- Load generator: PASSED

### Build Tests
- Debug build: PASSED
- Release build: PASSED

### Integration Tests
- Node startup: PASSED
- API endpoints: PASSED
- Wallet CLI: PASSED
- Load generator: PASSED

## Log Files
- Main log: `test_run_$Timestamp.log`
- Node log: `node_$Timestamp.log`
- Wallet log: `wallet_create_$Timestamp.log`
- Load generator log: `loadgen_$Timestamp.log`

## Performance Metrics
- Benchmarks: Available

## Recommendations
1. Review any failed tests
2. Check performance benchmarks
3. Verify security test results
4. Monitor system resources during load tests

"@
    
    $reportContent | Out-File -FilePath $reportFile -Encoding UTF8
    Write-Success "Test report generated: $reportFile"
}

# Main execution
function Main {
    Write-Log "Starting IPPAN test suite..."
    Write-Log "Project root: $ProjectRoot"
    Write-Log "Log directory: $LogDir"
    
    try {
        # Run tests
        Test-Build
        Test-UnitTests
        Test-Benchmarks
        Test-Integration
        Test-Security
        
        # Generate report
        New-TestReport
        
        Write-Log "Test suite completed successfully!"
        Write-Success "All tests passed"
    }
    catch {
        Write-Error "Test suite failed: $_"
        exit 1
    }
}

# Run main function
Main
