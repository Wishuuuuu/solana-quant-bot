#!/bin/bash

# Solana Quant Bot - Auto-Start Script
# This script automatically starts all components in the correct order

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
LOG_DIR="logs"
PID_DIR="pids"
MAX_RETRIES=3
RETRY_DELAY=5

# Create necessary directories
mkdir -p "$LOG_DIR"
mkdir -p "$PID_DIR"

# Function to check if a process is running
is_running() {
    local pid_file="$PID_DIR/$1.pid"
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if ps -p "$pid" > /dev/null 2>&1; then
            return 0
        else
            rm -f "$pid_file"
            return 1
        fi
    fi
    return 1
}

# Function to start a component
start_component() {
    local name="$1"
    local command="$2"
    local pid_file="$PID_DIR/$name.pid"
    local log_file="$LOG_DIR/$name.log"

    if is_running "$name"; then
        echo -e "${YELLOW}$name is already running${NC}"
        return 0
    fi

    echo -e "${BLUE}Starting $name...${NC}"
    
    # Start the component in background
    nohup bash -c "$command" > "$log_file" 2>&1 &
    local pid=$!
    
    # Save PID
    echo $pid > "$pid_file"
    
    # Wait and check if it's still running
    sleep 2
    if ps -p "$pid" > /dev/null 2>&1; then
        echo -e "${GREEN}$name started successfully (PID: $pid)${NC}"
        return 0
    else
        echo -e "${RED}$name failed to start${NC}"
        rm -f "$pid_file"
        return 1
    fi
}

# Function to stop a component
stop_component() {
    local name="$1"
    local pid_file="$PID_DIR/$name.pid"

    if is_running "$name"; then
        local pid=$(cat "$pid_file")
        echo -e "${YELLOW}Stopping $name (PID: $pid)...${NC}"
        kill "$pid"
        
        # Wait for graceful shutdown
        local count=0
        while ps -p "$pid" > /dev/null 2>&1 && [ $count -lt 10 ]; do
            sleep 1
            count=$((count + 1))
        done
        
        # Force kill if still running
        if ps -p "$pid" > /dev/null 2>&1; then
            echo -e "${RED}Force killing $name${NC}"
            kill -9 "$pid"
        fi
        
        rm -f "$pid_file"
        echo -e "${GREEN}$name stopped${NC}"
    else
        echo -e "${YELLOW}$name is not running${NC}"
    fi
}

# Function to check component health
check_health() {
    local name="$1"
    local pid_file="$PID_DIR/$name.pid"

    if is_running "$name"; then
        echo -e "${GREEN}✓ $name is running${NC}"
        return 0
    else
        echo -e "${RED}✗ $name is not running${NC}"
        return 1
    fi
}

# Start infrastructure services
echo -e "${BLUE}=== Starting Infrastructure Services ===${NC}"

# Check if Redis is running
if ! redis-cli ping > /dev/null 2>&1; then
    echo -e "${YELLOW}Redis is not running, starting...${NC}"
    redis-server --daemonize yes --appendonly yes
    sleep 2
fi

# Check if PostgreSQL is running
if ! pg_isready > /dev/null 2>&1; then
    echo -e "${YELLOW}PostgreSQL is not running, please start it manually${NC}"
    exit 1
fi

# Start core components in order
echo -e "${BLUE}=== Starting Core Components ===${NC}"

# 1. Smart Money Radar
start_component "smart-money-radar" "cd smart-money-radar && cargo run --release"

# 2. AI Sentiment Engine
start_component "ai-sentiment-engine" "cd ai-sentiment-engine && python main.py"

# 3. Price Oracle
start_component "price-oracle" "cd price-oracle && cargo run --release"

# 4. Orchestrator
start_component "orchestrator" "cd orchestrator && cargo run --release"

# 5. Execution Engine
start_component "execution-engine" "cd execution-engine && cargo run --release"

# 6. Risk Manager
start_component "risk-manager" "cd risk-manager && cargo run --release"

# 7. Dashboard API
start_component "dashboard-api" "cd dashboard-api && cargo run --release"

# 8. Health Checker
start_component "health-checker" "cd health-checker && cargo run --release"

# 9. React Dashboard
start_component "dashboard" "cd dashboard && npm start"

# Start optional components
echo -e "${BLUE}=== Starting Autonomous Components ===${NC}"

# Notification Service
if [ -n "$TELEGRAM_BOT_TOKEN" ] || [ -n "$DISCORD_WEBHOOK_URL" ]; then
    start_component "notifications" "cd notifications && cargo run --release"
fi

# Auto-Discovery System
start_component "auto-discovery" "cd auto-discovery && cargo run --release"

# Auto-Optimizer
start_component "auto-optimizer" "cd auto-optimizer && cargo run --release"

# Auto-Balance Manager
start_component "auto-balance" "cd auto-balance && cargo run --release"

# Task Scheduler
start_component "scheduler" "cd scheduler && cargo run --release"

# Auto-Reporter
start_component "auto-reporter" "cd auto-reporter && cargo run --release"

# Wait for all components to stabilize
echo -e "${BLUE}=== Waiting for components to stabilize ===${NC}"
sleep 10

# Health check
echo -e "${BLUE}=== Component Health Check ===${NC}"
check_health "smart-money-radar"
check_health "ai-sentiment-engine"
check_health "price-oracle"
check_health "orchestrator"
check_health "execution-engine"
check_health "risk-manager"
check_health "dashboard-api"
check_health "health-checker"
check_health "dashboard"

echo -e "${GREEN}=== All components started successfully ===${NC}"
echo -e "${BLUE}Dashboard available at: http://localhost:3000${NC}"
echo -e "${BLUE}API available at: http://localhost:3000/api${NC}"
echo -e "${YELLOW}Use './stop.sh' to stop all components${NC}"
