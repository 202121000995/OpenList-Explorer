@echo off
setlocal
cd /d "%~dp0\.."
where py >nul 2>nul
if %ERRORLEVEL%==0 (
  py scripts\prepare_aria2_bundle.py %*
  exit /b %ERRORLEVEL%
)
where python >nul 2>nul
if %ERRORLEVEL%==0 (
  python scripts\prepare_aria2_bundle.py %*
  exit /b %ERRORLEVEL%
)
set "BUNDLED_PY=%USERPROFILE%\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe"
if exist "%BUNDLED_PY%" (
  "%BUNDLED_PY%" scripts\prepare_aria2_bundle.py %*
  exit /b %ERRORLEVEL%
)
echo Python was not found. Install Python or run from Codex Desktop.
exit /b 1
