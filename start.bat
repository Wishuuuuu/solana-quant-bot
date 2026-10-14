@echo off
REM Solana Quant Bot - Auto-Start Script for Windows

setlocal enabledelayedexpansion

set LOG_DIR=logs
set PID_DIR=pids

REM Create directories
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
if not exist "%PID_DIR%" mkdir "%PID_DIR%"

echo === Starting Solana Quant Bot ===
echo.

REM Check Redis
echo Checking Redis...
redis-cli ping >nul 2>&1
if errorlevel 1 (
    echo Redis is not running. Please start Redis first.
    echo You can use: docker run -d -p 6379:6379 redis
    pause
    exit /b 1
)
echo Redis is running
echo.

REM Start components in separate windows
echo Starting Smart Money Radar...
start "Smart Money Radar" cmd /k "cd smart-money-radar && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting AI Sentiment Engine...
start "AI Sentiment Engine" cmd /k "cd ai-sentiment-engine && python main.py"

timeout /t 3 /nobreak >nul

echo Starting Price Oracle...
start "Price Oracle" cmd /k "cd price-oracle && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Orchestrator...
start "Orchestrator" cmd /k "cd orchestrator && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Execution Engine...
start "Execution Engine" cmd /k "cd execution-engine && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Risk Manager...
start "Risk Manager" cmd /k "cd risk-manager && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Dashboard API...
start "Dashboard API" cmd /k "cd dashboard-api && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Health Checker...
start "Health Checker" cmd /k "cd health-checker && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting React Dashboard...
start "Dashboard" cmd /k "cd dashboard && npm start"

timeout /t 3 /nobreak >nul

echo Starting Wallet Tracker...
start "Wallet Tracker" cmd /k "cd wallet-tracker && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Auto-Discovery...
start "Auto-Discovery" cmd /k "cd auto-discovery && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Auto-Optimizer...
start "Auto-Optimizer" cmd /k "cd auto-optimizer && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Auto-Balance...
start "Auto-Balance" cmd /k "cd auto-balance && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Scheduler...
start "Scheduler" cmd /k "cd scheduler && cargo run --release"

timeout /t 3 /nobreak >nul

echo Starting Auto-Reporter...
start "Auto-Reporter" cmd /k "cd auto-reporter && cargo run --release"

echo.
echo === All components started ===
echo.
echo Dashboard: http://localhost:3000
echo API: http://localhost:3000/api
echo.
echo Close this window to keep components running
echo Individual component windows can be closed to stop specific components
pause
