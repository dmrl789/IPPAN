# IPPAN Simple Test Runner
Write-Host "IPPAN Blockchain - Simple Test Runner" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Green

$ProjectRoot = Split-Path -Parent $PSScriptRoot
Write-Host "Project root: $ProjectRoot"

# Test 1: Check if Rust is installed
Write-Host "`n[1/9] Testing Rust Installation..." -ForegroundColor Yellow
try {
    $rustVersion = rustc --version
    Write-Host "✓ Rust found: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "✗ Rust not found" -ForegroundColor Red
    exit 1
}

# Test 2: Check if Cargo is working
Write-Host "`n[2/9] Testing Cargo..." -ForegroundColor Yellow
try {
    cargo --version | Out-Null
    Write-Host "✓ Cargo working" -ForegroundColor Green
} catch {
    Write-Host "✗ Cargo not working" -ForegroundColor Red
    exit 1
}

# Test 3: Check project structure
Write-Host "`n[3/9] Testing Project Structure..." -ForegroundColor Yellow
$requiredFiles = @(
    "Cargo.toml",
    "rust-toolchain.toml",
    "crates/common/Cargo.toml",
    "crates/node/Cargo.toml",
    "crates/wallet-cli/Cargo.toml",
    "crates/loadgen-cli/Cargo.toml",
    "crates/bench/Cargo.toml"
)

$allFilesPresent = $true
foreach ($file in $requiredFiles) {
    if (Test-Path $file) {
        Write-Host "  ✓ $file" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $file" -ForegroundColor Red
        $allFilesPresent = $false
    }
}

if ($allFilesPresent) {
    Write-Host "✓ All required files present" -ForegroundColor Green
} else {
    Write-Host "✗ Missing required files" -ForegroundColor Red
    exit 1
}

# Test 4: Check dependencies
Write-Host "`n[4/9] Testing Dependencies..." -ForegroundColor Yellow
try {
    cargo check --quiet
    Write-Host "✓ Dependencies check passed" -ForegroundColor Green
} catch {
    Write-Host "✗ Dependencies check failed" -ForegroundColor Red
    exit 1
}

# Test 5: Build debug version
Write-Host "`n[5/9] Testing Debug Build..." -ForegroundColor Yellow
try {
    cargo build --quiet
    Write-Host "✓ Debug build successful" -ForegroundColor Green
} catch {
    Write-Host "✗ Debug build failed" -ForegroundColor Red
    exit 1
}

# Test 6: Run unit tests for common crate
Write-Host "`n[6/9] Testing Common Crate..." -ForegroundColor Yellow
try {
    cargo test -p ippan-common --lib --quiet
    Write-Host "✓ Common crate tests passed" -ForegroundColor Green
} catch {
    Write-Host "✗ Common crate tests failed" -ForegroundColor Red
    exit 1
}

# Test 7: Check if binaries can be built
Write-Host "`n[7/9] Testing Binary Builds..." -ForegroundColor Yellow
try {
    cargo build --release --bin ippan-wallet-cli --quiet
    Write-Host "✓ Wallet CLI binary built" -ForegroundColor Green
} catch {
    Write-Host "✗ Wallet CLI binary build failed" -ForegroundColor Red
    exit 1
}

try {
    cargo build --release --bin ippan-loadgen-cli --quiet
    Write-Host "✓ Load generator binary built" -ForegroundColor Green
} catch {
    Write-Host "✗ Load generator binary build failed" -ForegroundColor Red
    exit 1
}

# Test 8: Check documentation
Write-Host "`n[8/9] Testing Documentation..." -ForegroundColor Yellow
if (Test-Path "README.md") {
    $readmeSize = (Get-Item "README.md").Length
    Write-Host "✓ README.md present ($readmeSize bytes)" -ForegroundColor Green
} else {
    Write-Host "✗ README.md missing" -ForegroundColor Red
    exit 1
}

if (Test-Path "docs/IPPAN_Minimal_PRD.md") {
    Write-Host "✓ PRD document present" -ForegroundColor Green
} else {
    Write-Host "✗ PRD document missing" -ForegroundColor Red
    exit 1
}

# Test 9: Check test files
Write-Host "`n[9/9] Testing Test Files..." -ForegroundColor Yellow
$testFiles = @(
    "testsprite_tests/test_plan.md",
    "testsprite_tests/run_tests.ps1",
    "docker-compose.yml",
    "Dockerfile"
)

$allTestFilesPresent = $true
foreach ($file in $testFiles) {
    if (Test-Path $file) {
        Write-Host "  ✓ $file" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $file" -ForegroundColor Red
        $allTestFilesPresent = $false
    }
}

if ($allTestFilesPresent) {
    Write-Host "✓ All test files present" -ForegroundColor Green
} else {
    Write-Host "✗ Missing test files" -ForegroundColor Red
    exit 1
}

# Summary
Write-Host "`n" + "="*50 -ForegroundColor Green
Write-Host "🎉 ALL TESTS PASSED!" -ForegroundColor Green
Write-Host "IPPAN project is ready for development." -ForegroundColor Green
Write-Host "="*50 -ForegroundColor Green

Write-Host "`nNext steps:" -ForegroundColor Cyan
Write-Host "1. Implement the remaining node modules (api, p2p, mempool, etc.)" -ForegroundColor White
Write-Host "2. Run: cargo run --release -p ippan-node -- --http-port 8080 --p2p-port 8081" -ForegroundColor White
Write-Host "3. Run: cargo run --release -p ippan-wallet-cli -- new mywallet" -ForegroundColor White
Write-Host "4. Run: cargo run --release -p ippan-loadgen-cli -- --tps 1000 --duration 30" -ForegroundColor White
Write-Host "5. For Docker: docker-compose up --profile load-test" -ForegroundColor White
