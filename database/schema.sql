-- Database schema for Solana Quant Bot

-- Smart Money Wallets
CREATE TABLE smart_money_wallets (
    id SERIAL PRIMARY KEY,
    address VARCHAR(44) UNIQUE NOT NULL,
    win_rate DECIMAL(5,2) NOT NULL,
    total_trades INTEGER NOT NULL,
    profit_factor DECIMAL(10,2) NOT NULL,
    avg_holding_time_hours DECIMAL(10,2),
    last_active TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    tags TEXT[],
    is_active BOOLEAN DEFAULT true,
    notes TEXT
);

-- Trade Signals
CREATE TABLE trade_signals (
    id SERIAL PRIMARY KEY,
    wallet_address VARCHAR(44) NOT NULL,
    token_mint VARCHAR(44) NOT NULL,
    action VARCHAR(10) NOT NULL, -- 'BUY' or 'SELL'
    amount DECIMAL(20,10) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    signature VARCHAR(88) UNIQUE,
    processed BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Sentiment Analysis Results
CREATE TABLE sentiment_analysis (
    id SERIAL PRIMARY KEY,
    token_mint VARCHAR(44) NOT NULL,
    hype_score DECIMAL(5,4) NOT NULL,
    sentiment VARCHAR(20) NOT NULL, -- 'Bullish', 'Bearish', 'Neutral'
    volume_velocity DECIMAL(5,4) NOT NULL,
    spam_score DECIMAL(5,4) NOT NULL,
    tweet_count INTEGER,
    engagement_score DECIMAL(5,4),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW
);

-- Trading Opportunities
CREATE TABLE trading_opportunities (
    id SERIAL PRIMARY KEY,
    token_mint VARCHAR(44) NOT NULL,
    combined_score DECIMAL(5,4) NOT NULL,
    smart_money_count INTEGER NOT NULL,
    sentiment_id INTEGER REFERENCES sentiment_analysis(id),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    status VARCHAR(20) DEFAULT 'PENDING', -- 'PENDING', 'ACCEPTED', 'REJECTED', 'EXECUTED'
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW
);

-- Positions
CREATE TABLE positions (
    id SERIAL PRIMARY KEY,
    token_mint VARCHAR(44) NOT NULL,
    entry_price DECIMAL(20,10) NOT NULL,
    current_price DECIMAL(20,10),
    size DECIMAL(20,10) NOT NULL,
    pnl_percentage DECIMAL(10,2),
    highest_price DECIMAL(20,10),
    stop_loss_price DECIMAL(20,10),
    take_profit_price DECIMAL(20,10),
    entry_time TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    exit_time TIMESTAMP WITH TIME ZONE,
    status VARCHAR(20) DEFAULT 'OPEN', -- 'OPEN', 'CLOSED', 'PARTIALLY_CLOSED'
    exit_reason VARCHAR(50),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW
);

-- Trade Executions
CREATE TABLE trade_executions (
    id SERIAL PRIMARY KEY,
    position_id INTEGER REFERENCES positions(id),
    token_mint VARCHAR(44) NOT NULL,
    action VARCHAR(10) NOT NULL, -- 'BUY' or 'SELL'
    amount DECIMAL(20,10) NOT NULL,
    price DECIMAL(20,10) NOT NULL,
    signature VARCHAR(88),
    priority_fee DECIMAL(20,10),
    slippage_basis_points INTEGER,
    status VARCHAR(20) DEFAULT 'PENDING', -- 'PENDING', 'CONFIRMED', 'FAILED'
    error_message TEXT,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW
);

-- Performance Metrics
CREATE TABLE performance_metrics (
    id SERIAL PRIMARY KEY,
    date DATE NOT NULL,
    total_trades INTEGER NOT NULL,
    winning_trades INTEGER NOT NULL,
    losing_trades INTEGER NOT NULL,
    win_rate DECIMAL(5,2) NOT NULL,
    total_pnl DECIMAL(20,10) NOT NULL,
    max_drawdown DECIMAL(10,2) NOT NULL,
    sharpe_ratio DECIMAL(10,4),
    avg_holding_time_hours DECIMAL(10,2),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW
);

-- Wallet Analytics
CREATE TABLE wallet_analytics (
    id SERIAL PRIMARY KEY,
    wallet_address VARCHAR(44) NOT NULL,
    token_mint VARCHAR(44) NOT NULL,
    entry_price DECIMAL(20,10),
    exit_price DECIMAL(20,10),
    pnl DECIMAL(20,10),
    pnl_percentage DECIMAL(10,2),
    holding_time_hours DECIMAL(10,2),
    entry_timestamp TIMESTAMP WITH TIME ZONE,
    exit_timestamp TIMESTAMP WITH TIME ZONE,
    is_winner BOOLEAN,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW
);

-- System Events (for monitoring)
CREATE TABLE system_events (
    id SERIAL PRIMARY KEY,
    component VARCHAR(50) NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL, -- 'INFO', 'WARNING', 'ERROR', 'CRITICAL'
    message TEXT NOT NULL,
    metadata JSONB,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW
);

-- Backtest Results
CREATE TABLE backtest_results (
    id SERIAL PRIMARY KEY,
    strategy_name VARCHAR(100) NOT NULL,
    start_date TIMESTAMP WITH TIME ZONE NOT NULL,
    end_date TIMESTAMP WITH TIME ZONE NOT NULL,
    initial_capital DECIMAL(20,10) NOT NULL,
    final_capital DECIMAL(20,10) NOT NULL,
    total_return DECIMAL(10,2) NOT NULL,
    total_trades INTEGER NOT NULL,
    win_rate DECIMAL(5,2) NOT NULL,
    max_drawdown DECIMAL(10,2) NOT NULL,
    sharpe_ratio DECIMAL(10,4),
    sortino_ratio DECIMAL(10,4),
    parameters JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW
);

-- Alerts and Notifications
CREATE TABLE alerts (
    id SERIAL PRIMARY KEY,
    alert_type VARCHAR(50) NOT NULL, -- 'TAKE_PROFIT', 'STOP_LOSS', 'NEW_OPPORTUNITY', 'SYSTEM_ERROR'
    token_mint VARCHAR(44),
    message TEXT NOT NULL,
    sent BOOLEAN DEFAULT false,
    sent_at TIMESTAMP WITH TIME ZONE,
    channel VARCHAR(20), -- 'TELEGRAM', 'DISCORD', 'EMAIL'
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW
);

-- Indexes for performance
CREATE INDEX idx_trade_signals_wallet ON trade_signals(wallet_address);
CREATE INDEX idx_trade_signals_token ON trade_signals(token_mint);
CREATE INDEX idx_trade_signals_timestamp ON trade_signals(timestamp);
CREATE INDEX idx_sentiment_token ON sentiment_analysis(token_mint);
CREATE INDEX idx_sentiment_timestamp ON sentiment_analysis(timestamp);
CREATE INDEX idx_positions_token ON positions(token_mint);
CREATE INDEX idx_positions_status ON positions(status);
CREATE INDEX idx_trade_executions_position ON trade_executions(position_id);
CREATE INDEX idx_system_events_timestamp ON system_events(timestamp);
CREATE INDEX idx_wallet_analytics_wallet ON wallet_analytics(wallet_address);
CREATE INDEX idx_smart_money_active ON smart_money_wallets(is_active);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_smart_money_wallets_updated_at BEFORE UPDATE ON smart_money_wallets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_positions_updated_at BEFORE UPDATE ON positions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
