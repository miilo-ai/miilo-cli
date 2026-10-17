$ErrorActionPreference = 'Stop'
$artifacts = @(Get-ChildItem -Path 'target/distrib' -Filter '*.msi')
if ($artifacts.Count -ne 1) { throw 'Expected exactly one Windows MSI candidate' }
$msi = $artifacts[0].FullName
$installRoot = Join-Path $env:ProgramFiles 'miilo-cli'
$log = Join-Path $env:RUNNER_TEMP 'miilo-msi-validation.log'
function Invoke-Msi([string]$operation) {
  $arguments = @($operation, "`"$msi`"", '/qn', '/norestart', '/l*v', "`"$log`"")
  $process = Start-Process -FilePath 'msiexec.exe' -ArgumentList $arguments -Wait -PassThru
  if ($process.ExitCode -notin @(0, 3010)) { throw "MSI operation failed ($($process.ExitCode)); inspect validation log" }
}
try {
  Invoke-Msi '/i'
  foreach ($name in @('miilo', 'miilo-local')) {
    $binary = Join-Path $installRoot "bin/$name.exe"
    if (-not (Test-Path $binary)) { throw "MSI did not install $name" }
    & $binary --version
    if ($LASTEXITCODE -ne 0) { throw "Installed $name failed its version check" }
  }
  if (-not (Test-Path (Join-Path $installRoot 'LICENSE'))) { throw 'MSI did not install LICENSE' }
} finally {
  Invoke-Msi '/x'
}
if (Test-Path (Join-Path $installRoot 'bin/miilo.exe')) { throw 'MSI uninstall left the executable behind' }
Write-Output 'Windows MSI install/version/license/removal passed.'
