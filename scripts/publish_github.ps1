$ErrorActionPreference = "Stop"

$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$PackageJson = Get-Content -Raw -Encoding UTF8 (Join-Path $ProjectRoot "package.json") | ConvertFrom-Json
$Version = $PackageJson.version
$Installer = Join-Path $ProjectRoot "src-tauri\target\release\bundle\nsis\OpenList Explorer_$($Version)_x64-setup.exe"
$Publisher = Join-Path $ProjectRoot "scripts\publish_github.py"
$P0Check = Join-Path $ProjectRoot "scripts\p0_release_check.py"
$EncodingCheck = Join-Path $ProjectRoot "scripts\check_encoding.cmd"
$ModalCheck = Join-Path $ProjectRoot "scripts\check_modal_layout.cmd"

Write-Host "Project:   $ProjectRoot"
Write-Host "Version:   $Version"
Write-Host "Installer: $Installer"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
  throw "GitHub CLI was not found. Install gh first: https://cli.github.com/"
}

if (-not (Test-Path $Publisher)) {
  throw "Publish script not found: $Publisher"
}
if (-not (Test-Path $P0Check)) {
  throw "P0 release check script not found: $P0Check"
}
if (-not (Test-Path $EncodingCheck)) {
  throw "Encoding check script not found: $EncodingCheck"
}
if (-not (Test-Path $ModalCheck)) {
  throw "Modal layout check script not found: $ModalCheck"
}

if (-not (Test-Path $Installer)) {
  throw "Installer not found. Build it first with: npm.cmd run tauri:build"
}

$Python = (Get-Command python -ErrorAction SilentlyContinue)
if (-not $Python) {
  $Python = (Get-Command py -ErrorAction SilentlyContinue)
}
$PythonExe = $null
if ($Python) {
  $PythonExe = $Python.Source
} else {
  $BundledPython = Join-Path $env:USERPROFILE ".cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe"
  if (Test-Path $BundledPython) {
    $PythonExe = $BundledPython
  }
}
if (-not $PythonExe) {
  throw "Python was not found. Install Python, add it to PATH, or run this from Codex Desktop where the bundled Python runtime exists."
}

Write-Host ""
Write-Host "> encoding check"
& $EncodingCheck
if ($LASTEXITCODE -ne 0) {
  throw "Encoding check failed. Fix the text encoding before publishing."
}

Write-Host ""
Write-Host "> modal screenshot check"
& $ModalCheck
if ($LASTEXITCODE -ne 0) {
  throw "Modal screenshot check failed. Fix the modal layout before publishing."
}

Write-Host ""
Write-Host "> P0 release check"
& $PythonExe $P0Check
if ($LASTEXITCODE -ne 0) {
  throw "P0 release check failed. Fix the release blockers before publishing."
}

Write-Host ""
Write-Host "> gh auth status"
gh auth status
if ($LASTEXITCODE -ne 0) {
  Write-Host ""
  Write-Host "GitHub CLI is not logged in. Starting browser login..."
  gh auth login --hostname github.com --web --git-protocol https
  $LoginExitCode = $LASTEXITCODE

  Write-Host ""
  Write-Host "> gh auth status"
  gh auth status
  if ($LASTEXITCODE -ne 0) {
    throw "GitHub CLI is still not logged in. gh auth login exit code: $LoginExitCode"
  }
  if ($LoginExitCode -ne 0) {
    Write-Host "gh auth login returned $LoginExitCode after authentication, usually because Git is not installed. Auth is OK, continuing."
  }
}

Write-Host ""
Write-Host "> publish source and installer"
& $PythonExe $Publisher
if ($LASTEXITCODE -ne 0) {
  throw "GitHub publish failed."
}

Write-Host ""
Write-Host "Publish complete."
