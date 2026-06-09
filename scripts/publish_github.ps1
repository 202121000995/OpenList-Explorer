$ErrorActionPreference = "Stop"

$ProjectRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$Installer = Join-Path $ProjectRoot "src-tauri\target\release\bundle\nsis\OpenList Explorer_0.1.3_x64-setup.exe"
$Publisher = Join-Path $ProjectRoot "scripts\publish_github.py"

Write-Host "Project:   $ProjectRoot"
Write-Host "Installer: $Installer"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
  throw "GitHub CLI was not found. Install gh first: https://cli.github.com/"
}

if (-not (Test-Path $Publisher)) {
  throw "Publish script not found: $Publisher"
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
