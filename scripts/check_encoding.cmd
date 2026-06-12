@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0\.."

set "BUNDLED_NODE=%USERPROFILE%\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin\node.exe"

where node >nul 2>nul
if %ERRORLEVEL%==0 (
  node scripts\check_encoding.cjs
  exit /b !ERRORLEVEL!
)

if exist "%BUNDLED_NODE%" (
  "%BUNDLED_NODE%" scripts\check_encoding.cjs
  exit /b !ERRORLEVEL!
)

echo Node.js was not found. Install Node.js or run from Codex Desktop.
exit /b 1
