# IPPAN Quick Test Runner
# Simple test execution script for Windows

Write-Host "IPPAN Blockchain - Quick Test Runner" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Green

# Configuration
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$LogDir = Join-Path $ProjectRoot "test_logs"

# Create log directory
if (!(Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}

$LogFile = Join-Path $LogDir "quick_test_$Timestamp.log"

function Write-TestLog {
    param([string]$Message, [string]$Status = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] [$Status] $Message"
    Write-Host $logMessage
    Add-Content -Path $LogFile -Value $logMessage
}

function Test-Step {
    param([string]$Name, [scriptblock]$Action)
    Write-TestLog "Testing: $Name"
    try {
        & $Action
        Write-TestLog "✓ $Name - PASSED" "SUCCESS"
        return $true
    }
    catch {
        Write-TestLog "✗ $Name - FAILED: $_" "ERROR"
        return $false
    }
}

# Test results tracking
$TestResults = @{}

Write-TestLog "Starting IPPAN quick tests..."
Write-TestLog "Project root: $ProjectRoot"
Write-TestLog "Log file: $LogFile"

# Change to project directory
Set-Location $ProjectRoot

# Test 1: Check if Rust is installed
$TestResults["Rust"] = Test-Step "Rust Installation" {
    $rustVersion = rustc --version
    if ($rustVersion -match "rustc") {
        Write-TestLog "Rust version: $rustVersion"
    } else {
        throw "Rust not found"
    }
}

# Test 2: Check if Cargo is working
$TestResults["Cargo"] = Test-Step "Cargo Check" {
    cargo --version | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo not working"
    }
}

# Test 3: Check project structure
$TestResults["Structure"] = Test-Step "Project Structure" {
    $requiredFiles = @(
        "Cargo.toml",
        "rust-toolchain.toml",
        "crates/common/Cargo.toml",
        "crates/node/Cargo.toml",
        "crates/wallet-cli/Cargo.toml",
        "crates/loadgen-cli/Cargo.toml",
        "crates/bench/Cargo.toml"
    )
    
    foreach ($file in $requiredFiles) {
        if (!(Test-Path $file)) {
            throw "Missing required file: $file"
        }
    }
    Write-TestLog "All required files present"
}

# Test 4: Check dependencies
$TestResults["Dependencies"] = Test-Step "Dependencies Check" {
    cargo check --quiet
    if ($LASTEXITCODE -ne 0) {
        throw "Dependency check failed"
    }
}

# Test 5: Build debug version
$TestResults["DebugBuild"] = Test-Step "Debug Build" {
    cargo build --quiet
    if ($LASTEXITCODE -ne 0) {
        throw "Debug build failed"
    }
}

# Test 6: Run unit tests for common crate
$TestResults["CommonTests"] = Test-Step "Common Crate Tests" {
    cargo test -p ippan-common --lib --quiet
    if ($LASTEXITCODE -ne 0) {
        throw "Common crate tests failed"
    }
}

# Test 7: Check if binaries can be built
$TestResults["Binaries"] = Test-Step "Binary Build Check" {
    # Try to build release binaries
    cargo build --release --bin ippan-wallet-cli --quiet
    if ($LASTEXITCODE -ne 0) {
        throw "Wallet CLI binary build failed"
    }
    
    cargo build --release --bin ippan-loadgen-cli --quiet
    if ($LASTEXITCODE -ne 0) {
        throw "Load generator binary build failed"
    }
}

# Test 8: Check documentation
$TestResults["Documentation"] = Test-Step "Documentation Check" {
    if (Test-Path "README.md") {
        $readmeSize = (Get-Item "README.md").Length
        if ($readmeSize -gt 1000) {
            Write-TestLog "README.md present and substantial ($readmeSize bytes)"
        } else {
            throw "README.md too small"
        }
    } else {
        throw "README.md missing"
    }
    
    if (Test-Path "docs/IPPAN_Minimal_PRD.md") {
        Write-TestLog "PRD document present"
    } else {
        throw "PRD document missing"
    }
}

# Test 9: Check test files
$TestResults["TestFiles"] = Test-Step "Test Files Check" {
    $testFiles = @(
        "testsprite_tests/test_plan.md",
        "testsprite_tests/run_tests.ps1",
        "docker-compose.yml",
        "Dockerfile"
    )
    
    foreach ($file in $testFiles) {
        if (!(Test-Path $file)) {
            throw "Missing test file: $file"
        }
    }
    Write-TestLog "All test files present"
}

# Generate summary
Write-TestLog ""
Write-TestLog "Test Summary:" "SUMMARY"
Write-TestLog "============" "SUMMARY"

$passed = 0
$failed = 0

foreach ($test in $TestResults.Keys) {
    if ($TestResults[$test]) {
        Write-TestLog "✓ $test - PASSED" "SUMMARY"
        $passed++
    } else {
        Write-TestLog "✗ $test - FAILED" "SUMMARY"
        $failed++
    }
}

Write-TestLog ""
Write-TestLog "Results: $passed passed, $failed failed" "SUMMARY"

if ($failed -eq 0) {
    Write-TestLog "🎉 All tests passed! IPPAN project is ready for development." "SUCCESS"
    Write-TestLog ""
    Write-TestLog "Next steps:" "INFO"
    Write-TestLog "1. Implement the remaining node modules (api, p2p, mempool, etc.)" "INFO"
    Write-TestLog "2. Run: cargo run --release -p ippan-node -- --http-port 8080 --p2p-port 8081" "INFO"
    Write-TestLog "3. Run: cargo run --release -p ippan-wallet-cli -- new mywallet" "INFO"
    Write-TestLog "4. Run: cargo run --release -p ippan-loadgen-cli -- --tps 1000 --duration 30" "INFO"
    Write-TestLog "5. For Docker: docker-compose up --profile load-test" "INFO"
} else {
    Write-TestLog "❌ Some tests failed. Please review the errors above." "ERROR"
    Write-TestLog "Check the log file for details: $LogFile" "INFO"
}

Write-TestLog ""
Write-TestLog "Test completed at $(Get-Date)" "INFO"
Write-TestLog "Log file: $LogFile" "INFO"
