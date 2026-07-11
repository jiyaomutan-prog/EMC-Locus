@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0seed-measurement-engineering-demo.ps1" %*
exit /b %ERRORLEVEL%
