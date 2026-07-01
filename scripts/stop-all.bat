@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0stop-all.ps1" %*
set "exitcode=%ERRORLEVEL%"
if not "%exitcode%"=="0" (
  echo.
  echo EMC Locus stop-all failed with exit code %exitcode%.
  echo Logs are under "%~dp0..\logs\launchers".
  pause
)
exit /b %exitcode%
