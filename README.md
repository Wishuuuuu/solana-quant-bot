# Solana Quant Bot - Enterprise-Grade AI Trading System

Production-ready high-frequency quant trading bot for Solana meme coins with smart money tracking, AI sentiment analysis, Jupiter DEX integration, and comprehensive risk management.

## 🏗️ Complete System Architecture

### Core Trading Engines

1. **Smart Money Radar (Rust)** - Tracks elite wallets with 60%+ win rates
2. **AI Sentiment Engine (Python)** - NLP-powered hype detection from Twitter/X
3. **Execution Engine (Rust)** - Jupiter DEX integration with dynamic priority fees
4. **Risk Manager (Rust)** - Automated take-profit, trailing stop-loss, position limits
5. **Orchestrator (Rust)** - Coordinates all components via Redis pub/sub

### Supporting Services

6. **Database Layer (PostgreSQL)** - Persistent storage for trades, analytics, backtests
7. **Wallet Tracker (Rust)** - Automated discovery and analysis of smart money wallets
8. **Backtesting Engine (Rust)** - Strategy validation with historical data
9. **Price Oracle (Rust)** - Real-time price feeds via Jupiter API
10. **Health Checker (Rust)** - Component monitoring with auto-restart
11. **Notification Service (Rust)** - Telegram/Discord alerts for trades and errors
12. **Dashboard API (Rust)** - REST API for monitoring and analytics
13. **Monitoring Dashboard (React)** - Real-time visualization of bot performance

### Advanced Features

- **Paper Trading Mode** - Test strategies without real money
- **Error Recovery System** - Circuit breakers, exponential backoff, automatic retries
- **Configuration Validation** - Startup checks for all parameters
- **Structured Logging** - Daily log rotation with component-level filtering
- **Docker Deployment** - Full containerization with docker-compose

### 🤖 100% Autonomous Features

#### Auto-Discovery System
- Automatically discovers new smart money wallets from:
  - Recent block analysis
  - Trending token holders
  - Pump.fun early buyers
- Periodically re-analyzes wallet performance
- Automatically prunes underperforming wallets

#### Auto-Strategy Optimizer
- Runs daily backtests on parameter variations
- Optimizes take-profit, stop-loss, and position sizing
- Automatically applies best-performing parameters
- Tracks Sharpe ratio and confidence metrics

#### Auto-Balance Management
- Monitors SOL balance in real-time
- Automatically pauses trading if balance too low
- Calculates optimal position sizes based on available funds
- Sends alerts for balance management

#### Scheduled Tasks
- Daily report generation (performance summary)
- Database cleanup (old logs, expired data)
- Database backups (daily at 04:00 UTC)
- Log rotation (daily at 03:00 UTC)
- Wallet performance updates (every 12 hours)
- System health reports (every hour)
- Parameter validation (every 4 hours)

#### Auto-Reporter
- Generates daily/weekly/monthly performance reports
- Sends reports via Telegram/Discord
- Tracks win rate, PnL, drawdown, Sharpe ratio
- Identifies best/worst performing tokens

#### Self-Healing
- Health checker monitors all components
- Auto-restart on failure
- Circuit breakers prevent cascading failures
- Exponential backoff for retry logic
- Escalation alerts for critical failures

## 📋 Prerequisites

### Required Software
- **Rust** 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Python** 3.10+
- **Node.js** 18+ (for dashboard)
- **Docker** & Docker Compose (for deployment)
- **PostgreSQL** 15+
- **Redis** 7+

### Required Accounts
- **Helius RPC** (free tier available at helius.xyz)
- **Twitter API** (optional, for real sentiment analysis)
- **Telegram Bot** (optional, for notifications)
- **Discord Webhook** (optional, for notifications)

## 🚀 Quick Start

### 1. Clone and Setup
```bash
cd solana-quant-bot
cp config/.env.example config/.env
```

### 2. Configure Environment
Edit `config/.env` with your credentials:
```bash
RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY
DATABASE_URL=postgresql://user:password@localhost/solana_quant
TELEGRAM_BOT_TOKEN=your_token
TELEGRAM_CHAT_ID=your_chat_id
PAPER_TRADING=true  # Start with paper trading!
WALLET_ADDRESS=your_wallet_address
```

### 3. Build All Components
```bash
# Build Rust components
cargo build --release

# Install Python dependencies
cd ai-sentiment-engine
pip install -r requirements.txt
cd ..

# Install React dashboard
cd dashboard
npm install
cd ..
```

### 4. Start Infrastructure
```bash
# Start Redis and PostgreSQL with Docker
docker-compose up -d redis postgres

# Or run locally:
# redis-server
# postgresql
```

### 5. Initialize Database
```bash
psql -U postgres -d solana_quant -f database/schema.sql
```

### 6. Run the System (100% Autonomous)

#### Option A: One-Command Start (Recommended)
```bash
# Linux/Mac
chmod +x start.sh
./start.sh

# Windows
start.bat
```

This will automatically start ALL components including:
- Core trading engines (5 components)
- Supporting services (4 components)
- Autonomous systems (5 components):
  - Auto-Discovery
  - Auto-Optimizer
  - Auto-Balance
  - Scheduler
  - Auto-Reporter

#### Option B: Docker Compose
```bash
docker-compose up -d
```

#### Option C: Manual Development Mode
```bash
# Terminal 1 - Smart Money Radar
cd smart-money-radar && cargo run

# Terminal 2 - AI Sentiment Engine
cd ai-sentiment-engine && python main.py

# Terminal 3 - Execution Engine
cd execution-engine && cargo run

# Terminal 4 - Risk Manager
cd risk-manager && cargo run

# Terminal 5 - Orchestrator
cd orchestrator && cargo run

# Terminal 6 - Price Oracle
cd price-oracle && cargo run

# Terminal 7 - Dashboard API
cd dashboard-api && cargo run

# Terminal 8 - Health Checker
cd health-checker && cargo run

# Terminal 9 - React Dashboard
cd dashboard && npm start

# Autonomous Components (Optional - run for full autonomy)
# Terminal 10 - Auto-Discovery
cd auto-discovery && cargo run

# Terminal 11 - Auto-Optimizer
cd auto-optimizer && cargo run

# Terminal 12 - Auto-Balance
cd auto-balance && cargo run

# Terminal 13 - Scheduler
cd scheduler && cargo run

# Terminal 14 - Auto-Reporter
cd auto-reporter && cargo run
```

### 7. Stop the System
```bash
# Linux/Mac
./stop.sh

# Windows
# Close individual windows or use Ctrl+C
```

## 📊 Monitoring Dashboard

Access the React dashboard at `http://localhost:3000` after starting the dashboard API and React app.

Features:
- Real-time position tracking
- Performance metrics (PnL, win rate, drawdown)
- Trade history
- System health status
- Alert notifications

## 🔧 Configuration

### Risk Management Parameters
```toml
[risk]
take_profit_percentage = 100.0
initial_stop_loss_percentage = 30.0
trailing_stop_percentage = 15.0
max_open_positions = 5
max_position_size = 1000000000  # 1 SOL
```

### Trading Parameters
```toml
[trading]
default_slippage = 100  # 1%
default_priority_fee = 1000
min_hype_score = 0.6
max_spam_score = 0.5
```

### Add Smart Money Wallets
Edit `wallet-tracker/src/main.rs`:
```rust
let wallets_to_analyze = vec![
    "WALLET_ADDRESS_1",
    "WALLET_ADDRESS_2",
];
```

## 🧪 Testing

### Run Unit Tests
```bash
cargo test
```

### Run Backtests
```bash
cd backtesting
cargo run
```

### Paper Trading
```bash
export PAPER_TRADING=true
cd execution-engine && cargo run
```

## 📈 Backtesting

The backtesting engine allows you to validate strategies before deployment:

```bash
cd backtesting
cargo run -- --strategy="base" --days=30 --initial-capital=10
```

Metrics calculated:
- Total return
- Win rate
- Sharpe ratio
- Maximum drawdown
- Sortino ratio
- Average holding time

## 🔔 Notifications

Configure Telegram alerts in `config/.env`:
```bash
TELEGRAM_BOT_TOKEN=your_bot_token
TELEGRAM_CHAT_ID=your_chat_id
```

Alert types:
- Trade executed
- Take profit triggered
- Stop loss triggered
- New trading opportunity
- System errors

## 🛡️ Safety Features

1. **Paper Trading Mode** - Test without real money
2. **Configuration Validation** - Startup checks for all parameters
3. **Circuit Breakers** - Automatic pause on repeated failures
4. **Position Limits** - Maximum exposure caps
5. **Slippage Protection** - Configurable tolerance
6. **Health Monitoring** - Auto-restart unhealthy components
7. **Error Recovery** - Exponential backoff and retries

## 🐳 Docker Deployment

### Production Deployment
```bash
# Build and start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop all services
docker-compose down

# Restart specific service
docker-compose restart execution-engine
```

### Environment Variables
Create `.env` file:
```bash
RPC_URL=your_rpc_url
DATABASE_URL=postgresql://user:pass@postgres:5432/solana_quant
REDIS_URL=redis://redis:6379
PAPER_TRADING=false
TELEGRAM_BOT_TOKEN=your_token
TELEGRAM_CHAT_ID=your_chat_id
```

## 📝 Logging

Logs are written to `logs/` directory with daily rotation:
- `smart-money-radar.YYYY-MM-DD.log`
- `ai-sentiment-engine.YYYY-MM-DD.log`
- `execution-engine.YYYY-MM-DD.log`
- `risk-manager.YYYY-MM-DD.log`
- `orchestrator.YYYY-MM-DD.log`
- And more...

View logs in real-time:
```bash
tail -f logs/execution-engine.$(date +%Y-%m-%d).log
```

## 🔍 Troubleshooting

### Redis Connection
```bash
redis-cli ping
# Should return: PONG
```

### Database Connection
```bash
psql -U postgres -d solana_quant -c "SELECT 1;"
```

### RPC Errors
- Verify Helius API key
- Check RPC credits
- Try different endpoint

### Component Not Starting
```bash
# Check health status
curl http://localhost:3000/api/health

# View component logs
docker-compose logs <component-name>
```

## 🔒 Security Best Practices

1. **Never commit** `config/wallet.json` to git
2. **Use environment variables** for all secrets
3. **Enable 2FA** on all accounts
4. **Rotate keys** regularly
5. **Monitor positions** daily
6. **Start with paper trading**
7. **Use dedicated trading wallet** with minimal funds
8. **Never share private keys**

## ⚡ Performance Optimization

1. **Use private RPC** - Dedicated endpoint for low latency
2. **Optimize priority fees** - Dynamic calculation based on network
3. **Geographic proximity** - Host near RPC region
4. **Connection pooling** - Reuse database and RPC connections
5. **Async processing** - Non-blocking I/O throughout
6. **Redis caching** - Price data and frequently accessed values

## 📚 API Documentation

### Dashboard API Endpoints

#### Health Check
```
GET /api/health
```

#### Get Active Positions
```
GET /api/positions
```

#### Get Performance Metrics
```
GET /api/performance?days=7
```

#### Get System Stats
```
GET /api/stats
```

## 🤝 Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Add tests for new features
4. Submit a pull request

## 📄 License

MIT License - Use at your own risk. Trading cryptocurrency involves significant risk.

## ⚠️ Disclaimer

This software is for educational purposes only. The authors are not responsible for any financial losses incurred while using this bot. Always trade responsibly and never invest more than you can afford to lose.

## 🆘 Support

For issues and questions:
- Check the troubleshooting section
- Review logs in `logs/` directory
- Open an issue on GitHub
- Join our Discord community (link coming soon)

## 🗺️ Roadmap

- [ ] Multi-chain support (Ethereum, Base)
- [ ] Advanced ML models for sentiment
- [ ] WebSocket-based real-time dashboard
- [ ] Mobile app for monitoring
- [ ] Strategy marketplace
- [ ] Copy trading functionality
