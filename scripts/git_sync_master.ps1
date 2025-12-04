# scripts/git_sync_master.ps1

$ErrorActionPreference = "Stop"

function Info($t){ Write-Host "[git] $t" -ForegroundColor Cyan }
function Warn($t){ Write-Host "[git WARN] $t" -ForegroundColor Yellow }
function Fail($t){ throw $t }

# Go to repo root
$root = (git rev-parse --show-toplevel).Trim()
if (-not $root) { Fail "Not inside a git repository." }
Set-Location $root

# Ensure master checked out (rule: master only)
git checkout master | Out-Null

# Choose remote branch ref (origin/master preferred; fallback origin/main if needed)
$remoteRef = "origin/master"
$refOk = $false
try { git show-ref --verify --quiet "refs/remotes/origin/master"; if ($LASTEXITCODE -eq 0) { $refOk = $true } } catch {}

if (-not $refOk) {
  try { git show-ref --verify --quiet "refs/remotes/origin/main"; if ($LASTEXITCODE -eq 0) { $remoteRef = "origin/main"; $refOk = $true } } catch {}
}

if (-not $refOk) { Fail "Could not find origin/master or origin/main." }

# Stash uncommitted work automatically (safe default)
$stashed = $false
if ((git status --porcelain).Trim().Length -gt 0) {
  Info "Working tree dirty -> stashing changes"
  git stash push -u -m "auto-stash before syncing master" | Out-Null
  $stashed = $true
}

Info "Fetching origin..."
git fetch origin | Out-Null

# Compute divergence: left=commits on remote not local, right=commits on local not remote
$counts = (git rev-list --left-right --count "$remoteRef...master").Trim().Split()

[int]$behind = $counts[0]
[int]$ahead  = $counts[1]
Info "Divergence vs $remoteRef : behind=$behind ahead=$ahead"

if ($behind -gt 0 -and $ahead -gt 0) {
  Info "Branches diverged -> rebasing master onto $remoteRef"
  # This is safe: it rewrites only local commits and results in a fast-forward push.
  git rebase $remoteRef
  if ($LASTEXITCODE -ne 0) {
    Warn "Rebase hit conflicts. Resolve them, then run: git rebase --continue"
    Warn "If you want to abort: git rebase --abort"
    throw "Rebase conflicts require manual resolution (cannot safely auto-resolve)."
  }
}
elseif ($behind -gt 0 -and $ahead -eq 0) {
  Info "Behind only -> fast-forward merge from $remoteRef"
  git merge --ff-only $remoteRef | Out-Null
}
elseif ($behind -eq 0 -and $ahead -gt 0) {
  Info "Ahead only -> will push after stash pop"
}
else {
  Info "Already up to date."
}

# Restore stashed changes if any (best effort)
if ($stashed) {
  Info "Popping stash (best effort)"
  git stash pop | Out-Null
  if ($LASTEXITCODE -ne 0) {
    Warn "Stash pop had conflicts. Resolve them manually, then continue."
    throw "Stash conflicts require manual resolution."
  }
}

# Push if we are ahead of remote
$counts2 = (git rev-list --left-right --count "$remoteRef...master").Trim().Split()
[int]$behind2 = $counts2[0]
[int]$ahead2  = $counts2[1]
Info "After sync: behind=$behind2 ahead=$ahead2"
if ($behind2 -ne 0) { throw "Unexpected: still behind remote after sync." }
if ($ahead2 -gt 0) {
  Info "Pushing master to origin..."
  git push origin master
  if ($LASTEXITCODE -ne 0) { throw "Push failed." }
  Info "Push OK."
} else {
  Info "No push needed."
}

Info "Master sync complete."

