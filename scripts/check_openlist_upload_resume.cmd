@echo off
setlocal
cd /d "%~dp0\.."

where py >nul 2>nul
if %ERRORLEVEL% EQU 0 (
  py scripts\check_openlist_upload_resume.py
  exit /b %ERRORLEVEL%
)

where python >nul 2>nul
if %ERRORLEVEL% EQU 0 (
  python scripts\check_openlist_upload_resume.py
  exit /b %ERRORLEVEL%
)

set "BUNDLED_PY=%USERPROFILE%\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe"
if exist "%BUNDLED_PY%" (
  "%BUNDLED_PY%" scripts\check_openlist_upload_resume.py
  exit /b %ERRORLEVEL%
)

echo Python was not found.
exit /b 1
