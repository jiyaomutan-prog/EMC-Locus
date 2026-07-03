@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0seed-equipment-demo.ps1" %*
exit /b %ERRORLEVEL%
