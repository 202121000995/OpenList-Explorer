@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0\.."

set "BUNDLED_NODE=%USERPROFILE%\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe"
set "BUNDLED_NODE_MODULES=%USERPROFILE%\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\node_modules"
set "BUNDLED_PNPM_MODULES=%USERPROFILE%\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\node_modules\.pnpm\node_modules"

if exist "%BUNDLED_NODE_MODULES%" (
  set "NODE_PATH=%BUNDLED_NODE_MODULES%;%BUNDLED_PNPM_MODULES%;%NODE_PATH%"
)

where node >nul 2>nul
if %ERRORLEVEL%==0 (
  node scripts\check_modal_layout.cjs
  exit /b !ERRORLEVEL!
)

if exist "%BUNDLED_NODE%" (
  "%BUNDLED_NODE%" scripts\check_modal_layout.cjs
  exit /b !ERRORLEVEL!
)

echo Node.js was not found. Install Node.js or run from Codex Desktop.
exit /b 1
