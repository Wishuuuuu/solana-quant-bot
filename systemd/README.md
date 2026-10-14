# Systemd Services for Production Deployment

This directory contains systemd service files for running Solana Quant Bot as a system service on Linux.

## Installation

1. Run the installation script:
```bash
chmod +x install-services.sh
sudo ./install-services.sh
```

This will:
- Create a dedicated user `solana`
- Install all service files to `/etc/systemd/system/`
- Enable all services to start on boot

## Services

The following services are installed:

1. `solana-quant-bot-smart-money-radar.service` - Smart Money Radar
2. `solana-quant-bot-ai-sentiment.service` - AI Sentiment Engine
3. `solana-quant-bot-execution.service` - Execution Engine
4. `solana-quant-bot-risk-manager.service` - Risk Manager
5. `solana-quant-bot-orchestrator.service` - Orchestrator
6. `solana-quant-bot-price-oracle.service` - Price Oracle
7. `solana-quant-bot-dashboard-api.service` - Dashboard API
8. `solana-quant-bot-health-checker.service` - Health Checker
9. `solana-quant-bot-auto-discovery.service` - Auto-Discovery System
10. `solana-quant-bot-auto-optimizer.service` - Auto-Strategy Optimizer
11. `solana-quant-bot-auto-balance.service` - Auto-Balance Manager
12. `solana-quant-bot-scheduler.service` - Task Scheduler
13. `solana-quant-bot-auto-reporter.service` - Auto-Reporter

## Management Commands

### Start all services:
```bash
sudo systemctl start solana-quant-bot-*
```

### Stop all services:
```bash
sudo systemctl stop solana-quant-bot-*
```

### Restart all services:
```bash
sudo systemctl restart solana-quant-bot-*
```

### Check status of all services:
```bash
sudo systemctl status solana-quant-bot-*
```

### View logs for a specific service:
```bash
sudo journalctl -u solana-quant-bot-smart-money-radar -f
```

### Enable services to start on boot:
```bash
sudo systemctl enable solana-quant-bot-*
```

### Disable services from starting on boot:
```bash
sudo systemctl disable solana-quant-bot-*
```

## Configuration

Services will read configuration from:
- Environment variables in `/etc/default/solana-quant-bot`
- Or from `.env` file in the installation directory

Create `/etc/default/solana-quant-bot`:
```bash
RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY
DATABASE_URL=postgresql://user:password@localhost/solana_quant
PAPER_TRADING=false
WALLET_ADDRESS=your_wallet_address
TELEGRAM_BOT_TOKEN=your_token
TELEGRAM_CHAT_ID=your_chat_id
```

## Troubleshooting

### Service failed to start:
```bash
sudo systemctl status solana-quant-bot-<service-name>
sudo journalctl -u solana-quant-bot-<service-name> -n 50
```

### Service keeps restarting:
Check logs for errors, common issues:
- Database connection failed
- Redis connection failed
- Configuration errors
- Missing dependencies

### Manual service management:
To manage individual services:
```bash
sudo systemctl start solana-quant-bot-smart-money-radar
sudo systemctl stop solana-quant-bot-smart-money-radar
sudo systemctl restart solana-quant-bot-smart-money-radar
```
