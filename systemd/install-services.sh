#!/bin/bash

# Install systemd services for Solana Quant Bot

set -e

SERVICE_DIR="/etc/systemd/system"
INSTALL_DIR="/opt/solana-quant-bot"
USER="solana"

echo "=== Installing Solana Quant Bot Systemd Services ==="

# Create user if doesn't exist
if ! id "$USER" &>/dev/null; then
    echo "Creating user $USER..."
    sudo useradd -r -s /bin/false $USER
fi

# Create install directory
sudo mkdir -p "$INSTALL_DIR"
sudo chown -R $USER:$USER "$INSTALL_DIR"

# Copy service files
echo "Installing service files..."
sudo cp systemd/*.service "$SERVICE_DIR/"

# Reload systemd
sudo systemctl daemon-reload

# Enable services
echo "Enabling services..."
sudo systemctl enable solana-quant-bot-smart-money-radar.service
sudo systemctl enable solana-quant-bot-ai-sentiment.service
sudo systemctl enable solana-quant-bot-execution.service
sudo systemctl enable solana-quant-bot-risk-manager.service
sudo systemctl enable solana-quant-bot-orchestrator.service
sudo systemctl enable solana-quant-bot-price-oracle.service
sudo systemctl enable solana-quant-bot-dashboard-api.service
sudo systemctl enable solana-quant-bot-health-checker.service
sudo systemctl enable solana-quant-bot-auto-discovery.service
sudo systemctl enable solana-quant-bot-auto-optimizer.service
sudo systemctl enable solana-quant-bot-auto-balance.service
sudo systemctl enable solana-quant-bot-scheduler.service
sudo systemctl enable solana-quant-bot-auto-reporter.service

echo "=== Services installed successfully ==="
echo ""
echo "Commands:"
echo "  Start all:    sudo systemctl start solana-quant-bot-*"
echo "  Stop all:     sudo systemctl stop solana-quant-bot-*"
echo "  Status:       sudo systemctl status solana-quant-bot-*"
echo "  View logs:    sudo journalctl -u solana-quant-bot-* -f"
