param(
  [Parameter(Mandatory = $true)]
  [string]$ExtensionId,

  [ValidateSet("Chrome", "Edge", "Both")]
  [string]$Browser = "Both",

  [string]$ExePath = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($ExePath)) {
  $ExePath = Join-Path $repoRoot "src-tauri\target\release\study_guardian_native_host.exe"
}

if (-not (Test-Path $ExePath)) {
  Push-Location (Join-Path $repoRoot "src-tauri")
  try {
    cargo build --release --bin study_guardian_native_host
  } finally {
    Pop-Location
  }
}

if (-not (Test-Path $ExePath)) {
  throw "Native Messaging host exe not found: $ExePath"
}

$manifestPath = Join-Path $repoRoot "native-messaging-host.generated.json"
$manifest = [ordered]@{
  name = "com.local.study_guardian"
  description = "Study Guardian Native Messaging Host"
  path = (Resolve-Path $ExePath).Path
  type = "stdio"
  allowed_origins = @("chrome-extension://$ExtensionId/")
}

$manifest | ConvertTo-Json -Depth 5 | Set-Content -Path $manifestPath -Encoding UTF8

function Register-NativeHost {
  param([string]$RegistryPath)
  New-Item -Path $RegistryPath -Force | Out-Null
  & reg.exe add ($RegistryPath -replace "^HKCU:", "HKCU") /ve /d $manifestPath /f | Out-Null
}

if ($Browser -eq "Chrome" -or $Browser -eq "Both") {
  Register-NativeHost "HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.local.study_guardian"
}

if ($Browser -eq "Edge" -or $Browser -eq "Both") {
  Register-NativeHost "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\com.local.study_guardian"
}

Write-Host "Generated Native Messaging manifest: $manifestPath"
Write-Host "Registered browser: $Browser"
