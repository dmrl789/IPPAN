# IPPAN Two Nodes Connection Demo PowerShell Script

Write-Host "IPPAN Two Nodes Connection Demo" -ForegroundColor Cyan
Write-Host "===============================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Choose which demo to run:" -ForegroundColor Yellow
Write-Host "1) Simple Two Nodes Demo (simplified implementation)"
Write-Host "2) Real IPPAN Nodes Demo (actual node implementation)"
Write-Host -NoNewline "Enter choice [1-2]: " -ForegroundColor Green
$choice = Read-Host

switch ($choice) {
    "1" {
        Write-Host ""
        Write-Host "Running Simple Two Nodes Demo..." -ForegroundColor Cyan
        Write-Host "--------------------------------" -ForegroundColor Cyan
        cargo run --example two_nodes_connect
    }
    "2" {
        Write-Host ""
        Write-Host "Running Real IPPAN Nodes Demo..." -ForegroundColor Cyan
        Write-Host "---------------------------------" -ForegroundColor Cyan
        Write-Host "Note: This will start actual IPPAN nodes on ports 8080/9001 and 8081/9002" -ForegroundColor Yellow
        cargo run --example real_nodes_connect
    }
    default {
        Write-Host "Invalid choice. Please run the script again and choose 1 or 2." -ForegroundColor Red
        exit 1
    }
}