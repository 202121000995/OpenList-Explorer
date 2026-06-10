@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0\.."
where py >nul 2>nul
if %ERRORLEVEL%==0 (
  py scripts\check_responsive_layout.py
  exit /b !ERRORLEVEL!
)
where python >nul 2>nul
if %ERRORLEVEL%==0 (
  python scripts\check_responsive_layout.py
  exit /b !ERRORLEVEL!
)
set "BUNDLED_PY=%USERPROFILE%\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe"
if exist "%BUNDLED_PY%" (
  "%BUNDLED_PY%" scripts\check_responsive_layout.py
  exit /b !ERRORLEVEL!
)
echo Python was not found. Install Python or run from Codex Desktop.
exit /b 1
