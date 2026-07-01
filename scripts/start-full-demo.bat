@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0start-full-demo.ps1" %*
set "exitcode=%ERRORLEVEL%"
if not "%exitcode%"=="0" (
  echo.
  echo EMC Locus full demo launcher failed with exit code %exitcode%.
  echo Logs are under "%~dp0..\logs\launchers".
  pause
)
exit /b %exitcode%
