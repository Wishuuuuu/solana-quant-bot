import React, { useState, useEffect } from 'react';
import axios from 'axios';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer, BarChart, Bar } from 'recharts';
import { Activity, DollarSign, TrendingUp, AlertTriangle, Settings, RefreshCw } from 'lucide-react';

const API_BASE = 'http://localhost:3000/api';

function App() {
  const [stats, setStats] = useState(null);
  const [positions, setPositions] = useState([]);
  const [performance, setPerformance] = useState(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState(null);

  const fetchData = async () => {
    try {
      setLoading(true);
      const [statsRes, positionsRes, performanceRes] = await Promise.all([
        axios.get(`${API_BASE}/stats`),
        axios.get(`${API_BASE}/positions`),
        axios.get(`${API_BASE}/performance?days=7`)
      ]);

      setStats(statsRes.data);
      setPositions(positionsRes.data);
      setPerformance(performanceRes.data);
      setLastUpdate(new Date());
    } catch (error) {
      console.error('Error fetching data:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 5000); // Refresh every 5 seconds
    return () => clearInterval(interval);
  }, []);

  if (loading && !stats) {
    return (
      <div className="min-h-screen bg-gray-900 flex items-center justify-center">
        <RefreshCw className="animate-spin text-white w-8 h-8" />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-900 text-white">
      <header className="bg-gray-800 border-b border-gray-700 px-6 py-4">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Activity className="w-8 h-8 text-green-500" />
            Solana Quant Bot Dashboard
          </h1>
          <div className="flex items-center gap-4">
            <span className="text-sm text-gray-400">
              Last update: {lastUpdate?.toLocaleTimeString()}
            </span>
            <button
              onClick={fetchData}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg flex items-center gap-2"
            >
              <RefreshCw className="w-4 h-4" />
              Refresh
            </button>
          </div>
        </div>
      </header>

      <main className="p-6">
        {/* Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          <StatCard
            icon={<Activity className="w-6 h-6" />}
            title="Active Positions"
            value={stats?.active_positions || 0}
            color="blue"
          />
          <StatCard
            icon={<TrendingUp className="w-6 h-6" />}
            title="Today's Trades"
            value={stats?.total_trades_today || 0}
            color="green"
          />
          <StatCard
            icon={<DollarSign className="w-6 h-6" />}
            title="Win Rate"
            value={`${stats?.win_rate_today?.toFixed(1) || 0}%`}
            color="purple"
          />
          <StatCard
            icon={<DollarSign className="w-6 h-6" />}
            title="Today's PnL"
            value={`${stats?.pnl_today?.toFixed(2) || 0} SOL`}
            color={stats?.pnl_today >= 0 ? 'green' : 'red'}
          />
        </div>

        {/* Performance Chart */}
        <div className="bg-gray-800 rounded-lg p-6 mb-8">
          <h2 className="text-xl font-semibold mb-4">Performance (Last 7 Days)</h2>
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={performance?.daily_metrics || []}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="date" stroke="#9CA3AF" />
              <YAxis stroke="#9CA3AF" />
              <Tooltip
                contentStyle={{ backgroundColor: '#1F2937', border: 'none' }}
                itemStyle={{ color: '#FFF' }}
              />
              <Legend />
              <Line type="monotone" dataKey="total_pnl" stroke="#10B981" name="PnL (SOL)" />
              <Line type="monotone" dataKey="win_rate" stroke="#8B5CF6" name="Win Rate (%)" />
            </LineChart>
          </ResponsiveContainer>
        </div>

        {/* Active Positions */}
        <div className="bg-gray-800 rounded-lg p-6 mb-8">
          <h2 className="text-xl font-semibold mb-4">Active Positions</h2>
          {positions.length === 0 ? (
            <p className="text-gray-400">No active positions</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="text-left border-b border-gray-700">
                    <th className="pb-3">Token</th>
                    <th className="pb-3">Entry Price</th>
                    <th className="pb-3">Current Price</th>
                    <th className="pb-3">PnL %</th>
                    <th className="pb-3">Size</th>
                    <th className="pb-3">Entry Time</th>
                  </tr>
                </thead>
                <tbody>
                  {positions.map((pos, idx) => (
                    <tr key={idx} className="border-b border-gray-700">
                      <td className="py-3 font-mono text-sm">{pos.token_mint.slice(0, 8)}...</td>
                      <td className="py-3">{pos.entry_price.toFixed(6)}</td>
                      <td className="py-3">{pos.current_price.toFixed(6)}</td>
                      <td className={`py-3 ${pos.pnl_percentage >= 0 ? 'text-green-500' : 'text-red-500'}`}>
                        {pos.pnl_percentage.toFixed(2)}%
                      </td>
                      <td className="py-3">{(pos.size / 1_000_000_000).toFixed(4)} SOL</td>
                      <td className="py-3 text-sm">{new Date(pos.entry_time).toLocaleString()}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>

        {/* Performance Metrics */}
        {performance && (
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <MetricCard
              title="Total Return"
              value={`${performance.total_pnl.toFixed(2)} SOL`}
              subtitle={`${((performance.total_pnl / 10) * 100).toFixed(1)}%`}
            />
            <MetricCard
              title="Win Rate"
              value={`${performance.win_rate.toFixed(1)}%`}
              subtitle={`${performance.winning_trades}W / ${performance.losing_trades}L`}
            />
            <MetricCard
              title="Max Drawdown"
              value={`${performance.max_drawdown.toFixed(2)}%`}
              subtitle="Peak to trough"
            />
          </div>
        )}
      </main>
    </div>
  );
}

function StatCard({ icon, title, value, color }) {
  const colorClasses = {
    blue: 'bg-blue-500',
    green: 'bg-green-500',
    purple: 'bg-purple-500',
    red: 'bg-red-500',
  };

  return (
    <div className="bg-gray-800 rounded-lg p-6">
      <div className="flex items-center justify-between mb-4">
        <div className={`p-3 rounded-lg ${colorClasses[color]}`}>{icon}</div>
      </div>
      <h3 className="text-gray-400 text-sm mb-1">{title}</h3>
      <p className="text-3xl font-bold">{value}</p>
    </div>
  );
}

function MetricCard({ title, value, subtitle }) {
  return (
    <div className="bg-gray-800 rounded-lg p-6">
      <h3 className="text-gray-400 text-sm mb-2">{title}</h3>
      <p className="text-2xl font-bold mb-1">{value}</p>
      <p className="text-gray-500 text-sm">{subtitle}</p>
    </div>
  );
}

export default App;
