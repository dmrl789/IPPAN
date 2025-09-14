# SSH with password script
$processInfo = New-Object System.Diagnostics.ProcessStartInfo
$processInfo.FileName = "ssh"
$processInfo.Arguments = "-o ConnectTimeout=10 -o StrictHostKeyChecking=no root@135.181.145.174"
$processInfo.UseShellExecute = $false
$processInfo.RedirectStandardInput = $true
$processInfo.RedirectStandardOutput = $true
$processInfo.RedirectStandardError = $true

$process = New-Object System.Diagnostics.Process
$process.StartInfo = $processInfo
$process.Start()

# Send password and commands
$process.StandardInput.WriteLine("Pam3C4dcwUq4")
$process.StandardInput.WriteLine("bash -s")
$process.StandardInput.Write((Get-Content "basic_setup.sh" -Raw))
$process.StandardInput.WriteLine("exit")
$process.StandardInput.Close()

$output = $process.StandardOutput.ReadToEnd()
$error = $process.StandardError.ReadToEnd()
$process.WaitForExit()

Write-Host "Output: $output"
if ($error) { Write-Host "Error: $error" }
