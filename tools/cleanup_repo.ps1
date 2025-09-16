# PowerShell cleanup script for IPPAN repository
param(
    [switch]$Apply
)

$Root = Split-Path -Parent $PSScriptRoot
$DryRun = -not $Apply

Write-Host "Running in $(if ($DryRun) { 'DRY-RUN' } else { 'APPLY' }) mode" -ForegroundColor Yellow

# Define patterns to remove
$RemovePatterns = @(
    "apps\wallet",
    "apps\console", 
    "apps\examples",
    "apps\gateway-api",
    "apps\node",
    "neuro-ui",
    "frontend",
    "deployment_package",
    "deployment_temp",
    "**\*demo*",
    "**\*example*", 
    "**\*mock*",
    "**\*playground*",
    "**\*storybook*",
    "**\*sample*",
    "**\*old*",
    "**\*legacy*",
    "**\*wip*",
    "**\*report*",
    "**\*slides*",
    "**\*pitch*",
    "**\*.exe",
    "**\*.pdb",
    "**\*.zip",
    "**\*.tar.gz",
    "**\node_modules",
    "**\target",
    "**\dist",
    "**\build",
    "**\.next",
    "**\.turbo",
    "**\coverage",
    "**\*.log",
    "**\Dockerfile.*",
    "**\docker-compose.dev.yml",
    "**\docker-compose.override.yml",
    "**\docker-compose.production.yml"
)

# Define patterns to keep
$KeepPatterns = @(
    "apps\unified-ui",
    "src",
    "crates", 
    "config",
    "scripts",
    "deployments",
    "benches",
    "tests",
    "tools",
    "docs",
    "migrations",
    "specs",
    "ci",
    "packages",
    "ippan",
    "keys",
    "data",
    "test_storage",
    "testsprite_templates",
    "testsprite_tests",
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    "LICENSE",
    ".gitignore",
    ".gitattributes"
)

$ItemsToRemove = @()

# Find items to remove
foreach ($pattern in $RemovePatterns) {
    $matches = Get-ChildItem -Path $Root -Recurse -Force | Where-Object { 
        $_.FullName -like "*$pattern*" -and $_.FullName -ne $Root
    }
    
    foreach ($match in $matches) {
        $shouldKeep = $false
        foreach ($keepPattern in $KeepPatterns) {
            if ($match.FullName -like "*$keepPattern*") {
                $shouldKeep = $true
                break
            }
        }
        
        if (-not $shouldKeep) {
            $ItemsToRemove += $match
        }
    }
}

# Remove duplicates and sort by depth (deepest first)
$ItemsToRemove = $ItemsToRemove | Sort-Object { $_.FullName.Split('\').Count } -Descending | Get-Unique

Write-Host "Found $($ItemsToRemove.Count) items to remove" -ForegroundColor Cyan

foreach ($item in $ItemsToRemove) {
    $relativePath = $item.FullName.Substring($Root.Length + 1)
    
    if ($item.Exists) {
        Write-Host "$(if ($DryRun) { 'would remove:' } else { 'removing:' }) $relativePath" -ForegroundColor $(if ($DryRun) { 'Yellow' } else { 'Red' })
        
        if (-not $DryRun) {
            try {
                if ($item.PSIsContainer) {
                    Remove-Item -Path $item.FullName -Recurse -Force -ErrorAction SilentlyContinue
                } else {
                    Remove-Item -Path $item.FullName -Force -ErrorAction SilentlyContinue
                }
            } catch {
                Write-Host "  Error removing $relativePath : $($_.Exception.Message)" -ForegroundColor Red
            }
        }
    }
}

if ($DryRun) {
    Write-Host "`nRun with -Apply to actually remove files" -ForegroundColor Yellow
} else {
    Write-Host "`nRemoved $($ItemsToRemove.Count) items" -ForegroundColor Green
}
