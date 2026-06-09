@echo off
setlocal
set PYTHON_EXE=C:\Users\wangn\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe
if not exist "%PYTHON_EXE%" (
  echo Python runtime not found: %PYTHON_EXE%
  exit /b 1
)
"%PYTHON_EXE%" "%~dp0prepare_tauri_build.py" %*
