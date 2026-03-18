import { useQuery } from '@tanstack/react-query';
import { getAnalytics, getDevices, getGamificationStats } from '../api/client';
import { useState } from 'react';

const BAR_COLORS = [
  'bg-red-500', 'bg-blue-500', 'bg-green-500', 'bg-amber-500', 'bg-purple-500',
  'bg-pink-500', 'bg-cyan-500', 'bg-orange-500', 'bg-teal-500', 'bg-indigo-500',
];

const STATE_COLORS: Record<string, string> = {
  idle: 'bg-gray-500',
  thinking: 'bg-yellow-500',
  waiting: 'bg-blue-500',
  success: 'bg-green-500',
  error: 'bg-red-500',
  taskcheck: 'bg-purple-500',
};

export default function AnalyticsPage() {
  const [deviceFilter, setDeviceFilter] = useState<string>('');
  const [days, setDays] = useState(30);

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });

  const { data: analytics, isLoading } = useQuery({
    queryKey: ['analytics', deviceFilter, days],
    queryFn: () => getAnalytics(days, deviceFilter || undefined),
    refetchInterval: 30000,
  });

  const { data: stats } = useQuery({
    queryKey: ['gamification-stats', deviceFilter],
    queryFn: () => getGamificationStats(deviceFilter || undefined),
    refetchInterval: 10000,
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-white">Analytics</h1>
        <div className="flex items-center gap-3">
          <select
            value={days}
            onChange={(e) => setDays(Number(e.target.value))}
            className="bg-gray-800 border border-gray-700 rounded-lg px-3 py-1.5 text-sm text-gray-300"
          >
            <option value={7}>7 days</option>
            <option value={14}>14 days</option>
            <option value={30}>30 days</option>
            <option value={90}>90 days</option>
          </select>
          {devices && devices.length > 1 && (
            <select
              value={deviceFilter}
              onChange={(e) => setDeviceFilter(e.target.value)}
              className="bg-gray-800 border border-gray-700 rounded-lg px-3 py-1.5 text-sm text-gray-300"
            >
              <option value="">All devices</option>
              {devices.map((d) => (
                <option key={d.id} value={d.id}>{d.name}</option>
              ))}
            </select>
          )}
        </div>
      </div>

      {/* Summary stats */}
      {stats && (
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          <StatCard label="Total XP" value={stats.total_xp.toLocaleString()} color="amber" />
          <StatCard label="Level" value={`${stats.level} - ${stats.title}`} color="red" />
          <StatCard label="Tool Uses" value={stats.total_tool_uses.toLocaleString()} color="blue" />
          <StatCard label="Streak" value={`${stats.current_streak}d`} color="orange" />
        </div>
      )}

      {isLoading ? (
        <div className="text-gray-500 text-sm">Loading analytics...</div>
      ) : analytics ? (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* Tools per day - bar chart */}
          <ChartCard title="Tool Uses Per Day">
            {analytics.tools_per_day.length === 0 ? (
              <EmptyChart />
            ) : (
              <BarChart
                data={analytics.tools_per_day.map((d) => ({ label: d.date.slice(5), value: d.count }))}
                color="bg-red-500"
              />
            )}
          </ChartCard>

          {/* XP over time */}
          <ChartCard title="XP Earned Per Day">
            {analytics.xp_over_time.length === 0 ? (
              <EmptyChart />
            ) : (
              <BarChart
                data={analytics.xp_over_time.map((d) => ({ label: d.date.slice(5), value: d.xp }))}
                color="bg-amber-500"
              />
            )}
          </ChartCard>

          {/* Hourly activity heatmap */}
          <ChartCard title="Active Coding Hours">
            {analytics.hourly_activity.length === 0 ? (
              <EmptyChart />
            ) : (
              <HourlyChart data={analytics.hourly_activity} />
            )}
          </ChartCard>

          {/* Tool distribution */}
          <ChartCard title="Tool Distribution">
            {analytics.tool_distribution.length === 0 ? (
              <EmptyChart />
            ) : (
              <HorizontalBarChart data={analytics.tool_distribution.map((d, i) => ({
                label: d.tool_name,
                value: d.count,
                color: BAR_COLORS[i % BAR_COLORS.length],
              }))} />
            )}
          </ChartCard>

          {/* State distribution - pie-like */}
          <ChartCard title="State Distribution">
            {analytics.state_distribution.length === 0 ? (
              <EmptyChart />
            ) : (
              <StateBar data={analytics.state_distribution} />
            )}
          </ChartCard>

          {/* Session lengths */}
          <ChartCard title="Session Lengths (minutes)">
            {analytics.session_lengths.length === 0 ? (
              <EmptyChart />
            ) : (
              <BarChart
                data={analytics.session_lengths.map((d) => ({
                  label: d.date.slice(5),
                  value: Math.round(d.duration_minutes),
                }))}
                color="bg-purple-500"
              />
            )}
          </ChartCard>
        </div>
      ) : null}
    </div>
  );
}

function StatCard({ label, value, color }: { label: string; value: string; color: string }) {
  const bg: Record<string, string> = {
    amber: 'bg-amber-500/10 border-amber-500/20',
    red: 'bg-red-500/10 border-red-500/20',
    blue: 'bg-blue-500/10 border-blue-500/20',
    orange: 'bg-orange-500/10 border-orange-500/20',
  };
  const text: Record<string, string> = {
    amber: 'text-amber-400',
    red: 'text-red-400',
    blue: 'text-blue-400',
    orange: 'text-orange-400',
  };
  return (
    <div className={`rounded-lg border p-4 ${bg[color]}`}>
      <p className="text-xs text-gray-500 mb-1">{label}</p>
      <p className={`text-xl font-bold ${text[color]}`}>{value}</p>
    </div>
  );
}

function ChartCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-4">
      <h3 className="text-sm font-medium text-gray-300 mb-4">{title}</h3>
      {children}
    </div>
  );
}

function EmptyChart() {
  return <div className="h-32 flex items-center justify-center text-gray-600 text-sm">No data yet</div>;
}

function BarChart({ data, color }: { data: { label: string; value: number }[]; color: string }) {
  const max = Math.max(...data.map((d) => d.value), 1);
  // Show last 14 entries max
  const visible = data.slice(-14);
  return (
    <div className="flex items-end gap-1 h-32">
      {visible.map((d, i) => (
        <div key={i} className="flex-1 flex flex-col items-center gap-1">
          <span className="text-[10px] text-gray-500">{d.value}</span>
          <div
            className={`w-full rounded-t ${color} min-h-[2px] transition-all`}
            style={{ height: `${(d.value / max) * 100}%` }}
          />
          <span className="text-[9px] text-gray-600 truncate w-full text-center">{d.label}</span>
        </div>
      ))}
    </div>
  );
}

function HourlyChart({ data }: { data: { hour: number; count: number }[] }) {
  // Fill all 24 hours
  const hours = Array.from({ length: 24 }, (_, i) => {
    const found = data.find((d) => d.hour === i);
    return { hour: i, count: found?.count ?? 0 };
  });
  const max = Math.max(...hours.map((h) => h.count), 1);

  return (
    <div className="space-y-1">
      <div className="flex gap-0.5 h-20">
        {hours.map((h) => {
          const intensity = h.count / max;
          const bg = intensity === 0 ? 'bg-gray-800' :
            intensity < 0.25 ? 'bg-green-900' :
            intensity < 0.5 ? 'bg-green-700' :
            intensity < 0.75 ? 'bg-green-500' : 'bg-green-400';
          return (
            <div key={h.hour} className="flex-1 flex flex-col justify-end gap-0.5" title={`${h.hour}:00 - ${h.count} events`}>
              <div className={`w-full rounded-sm ${bg} transition-all`} style={{ height: `${Math.max(intensity * 100, 4)}%` }} />
            </div>
          );
        })}
      </div>
      <div className="flex gap-0.5">
        {hours.map((h) => (
          <div key={h.hour} className="flex-1 text-center text-[8px] text-gray-600">
            {h.hour % 4 === 0 ? `${h.hour}` : ''}
          </div>
        ))}
      </div>
    </div>
  );
}

function HorizontalBarChart({ data }: { data: { label: string; value: number; color: string }[] }) {
  const max = Math.max(...data.map((d) => d.value), 1);
  return (
    <div className="space-y-2">
      {data.slice(0, 10).map((d, i) => (
        <div key={i} className="flex items-center gap-2">
          <span className="text-xs text-gray-400 w-24 truncate text-right">{d.label}</span>
          <div className="flex-1 h-4 bg-gray-800 rounded overflow-hidden">
            <div
              className={`h-full ${d.color} rounded transition-all`}
              style={{ width: `${(d.value / max) * 100}%` }}
            />
          </div>
          <span className="text-xs text-gray-500 w-10">{d.value}</span>
        </div>
      ))}
    </div>
  );
}

function StateBar({ data }: { data: { state: string; count: number }[] }) {
  const total = data.reduce((sum, d) => sum + d.count, 0) || 1;
  return (
    <div className="space-y-3">
      {/* Stacked bar */}
      <div className="h-6 rounded-lg overflow-hidden flex">
        {data.map((d, i) => (
          <div
            key={i}
            className={`${STATE_COLORS[d.state] || 'bg-gray-600'} transition-all`}
            style={{ width: `${(d.count / total) * 100}%` }}
            title={`${d.state}: ${d.count} (${Math.round((d.count / total) * 100)}%)`}
          />
        ))}
      </div>
      {/* Legend */}
      <div className="flex flex-wrap gap-3">
        {data.map((d, i) => (
          <div key={i} className="flex items-center gap-1.5">
            <div className={`w-2.5 h-2.5 rounded-sm ${STATE_COLORS[d.state] || 'bg-gray-600'}`} />
            <span className="text-xs text-gray-400">{d.state}</span>
            <span className="text-xs text-gray-600">{Math.round((d.count / total) * 100)}%</span>
          </div>
        ))}
      </div>
    </div>
  );
}
