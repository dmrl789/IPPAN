<#
Clean redeploy orchestrator for devnet nodes.
Runs clean redeploy, optional soak logging, and optional daily schedule.
#>
param(
    [string]$Branch = "main",
    [string]$RepoUrl = "https://github.com/dmrl789/IPPAN",
    [switch]$RunSoak,
    [switch]$Schedule,
    [string]$User = "ippan-devnet",
    [string]$Node1 = "188.245.97.41",
    [string]$Node2 = "135.181.145.174",
    [string]$Node3 = "5.223.51.238",
    [string]$Node4 = "178.156.219.107"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$cleanScript = Join-Path $root "scripts/devnet_clean_redeploy.ps1"
if (-not (Test-Path $cleanScript)) { throw "Missing clean redeploy helper at $cleanScript" }

function Write-Step([string]$msg) {
    Write-Host "==> $msg" -ForegroundColor Cyan
}

function Start-SoakLogging([string]$url, [string]$outFile) {
    Write-Step "Starting soak logging to $outFile (10s interval for 24h)"
    $dir = Split-Path -Parent $outFile
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $scriptBlock = {
        param($Url, $FilePath, $DurationSeconds)
        $end = (Get-Date).AddSeconds($DurationSeconds)
        while ((Get-Date) -lt $end) {
            $ts = Get-Date -Format "o"
            try {
                $resp = Invoke-RestMethod -Uri $Url -TimeoutSec 8
                $line = @{ timestamp = $ts; ok = $true; status = $resp } | ConvertTo-Json -Depth 16 -Compress
            } catch {
                $line = @{ timestamp = $ts; ok = $false; error = $_.Exception.Message } | ConvertTo-Json -Depth 6 -Compress
            }
            Add-Content -Path $FilePath -Value $line
            Start-Sleep -Seconds 10
        }
    }
    Start-Job -ScriptBlock $scriptBlock -ArgumentList $url, $outFile, 86400 | Out-Null
}

function Register-DailySchedule([string]$scriptPath, [string]$branchArg) {
    $utcTarget = [datetime]::SpecifyKind((Get-Date).Date.AddHours(4), 'Utc')
    $localStart = $utcTarget.ToLocalTime().ToString("HH:mm")
    $taskName = "IppanDevnetRedeployDaily"
    $action = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`" -Branch $branchArg -RunSoak"
    Write-Step "Creating scheduled task '$taskName' daily at 04:00 UTC (local $localStart)"
    schtasks /Create /F /SC DAILY /ST $localStart /TN $taskName /TR "`"$action`"" /RU SYSTEM | Out-Host
}

Write-Step "Running clean redeploy on all nodes from branch '$Branch'"
& $cleanScript -User $User -Node1 $Node1 -Node2 $Node2 -Node3 $Node3 -Node4 $Node4 -RepoUrl $RepoUrl -Branch $Branch

if ($RunSoak) {
    $ts = Get-Date -Format "yyyyMMdd_HHmmss"
    $outFile = Join-Path $root ("tmp/devnet/soak_status_{0}.ndjson" -f $ts)
    Start-SoakLogging -url "http://$Node4:8080/status" -outFile $outFile
}

if ($Schedule) {
    Register-DailySchedule -scriptPath (Join-Path $root "devnet_redeploy.ps1") -branchArg $Branch
}

