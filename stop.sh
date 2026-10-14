#!/bin/bash

# Solana Quant Bot - Stop Script
# This script stops all running components

set -e

PID_DIR="pids"

# Function to stop a component
stop_component() {
    local name="$1"
    local pid_file="$PID_DIR/$name.pid"

    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if ps -p "$pid" > /dev/null 2>&1; then
            echo "Stopping $name (PID: $pid)..."
            kill "$pid"
            
            # Wait for graceful shutdown
            local count=0
            while ps -p "$pid" > /dev/null 2>&1 && [ $count -lt 10 ]; do
                sleep 1
                count=$((count + 1))
            done
            
            # Force kill if still running
            if ps -p "$pid" > /dev/null 2>&1; then
                echo "Force killing $name"
                kill -9 "$pid"
            fi
            
            rm -f "$pid_file"
            echo "$name stopped"
        else
            echo "$name is not running"
            rm -f "$pid_file"
        fi
    else
        echo "$name is not running"
    fi
}

echo "=== Stopping All Components ==="

# Stop all components
stop_component "smart-money-radar"
stop_component "ai-sentiment-engine"
stop_component "price-oracle"
stop_component "orchestrator"
stop_component "execution-engine"
stop_component "risk-manager"
stop_component "dashboard-api"
stop_component "health-checker"
stop_component "dashboard"
stop_component "notifications"
stop_component "auto-discovery"
stop_component "auto-optimizer"
stop_component "auto-balance"
stop_component "scheduler"
stop_component "auto-reporter"

echo "=== All components stopped ==="
