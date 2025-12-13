param(
  [string]$Url = "http://178.156.219.107:8080/status",
  [int]$Seconds = 86400,
  [int]$Interval = 10
)

$ErrorActionPreference="Continue"
$end=(Get-Date).AddSeconds($Seconds)
$out="tmp/devnet/soak_status_$(Get-Date -Format yyyyMMdd_HHmmss).ndjson"
New-Item -Force -Path (Split-Path $out) | Out-Null

while((Get-Date) -lt $end){
  $ts=(Get-Date).ToUniversalTime().ToString("o")
  try { 
    $body=curl.exe -fsS $Url
    "{""ts"":""$ts"",""ok"":true,""body"":$body}" | Add-Content -Encoding utf8 $out
  }
  catch { 
    "{""ts"":""$ts"",""ok"":false,""err"":""$($_.Exception.Message.Replace('"',''''))""}" | Add-Content -Encoding utf8 $out
  }
  Start-Sleep -Seconds $Interval
}

Write-Host "SOAK DONE -> $out"

