@echo off
setlocal
cd /d "%~dp0"

:: 1. Ensure Vigilon backend is running on port 8745
netstat -ano | findstr ":8745 " | findstr "LISTENING" >nul
if %errorlevel% neq 0 (
    echo Starting Vigilon Observability Engine...
    start /min "" cargo run -p vigilon-api
    timeout /t 3 /nobreak >nul
)

:: 2. Launch native standalone desktop application window
set "EDGE=%ProgramFiles(x86)%\Microsoft\Edge\Application\msedge.exe"
if exist "%EDGE%" (
    start "" "%EDGE%" --app="http://127.0.0.1:8745" --window-size=1440,900 --user-data-dir="%LOCALAPPDATA%\Vigilon\app-profile"
) else (
    start "" "http://127.0.0.1:8745"
)
