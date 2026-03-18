import { useQuery } from '@tanstack/react-query';
import { getDevices, getOtaJobs, getFirmware, getGamificationStats } from '../api/client';
import { Link } from 'react-router-dom';
import StateIndicator from '../components/StateIndicator';

export default function Dashboard() {
  const { data: devices } = useQuery({
    queryKey: ['devices'],
    queryFn: getDevices,
    refetchInterval: 5000,
  });

  const { data: jobs } = useQuery({
    queryKey: ['ota-jobs'],
    queryFn: getOtaJobs,
  });

  const { data: firmware } = useQuery({
    queryKey: ['firmware'],
    queryFn: getFirmware,
  });

  const { data: stats } = useQuery({
    queryKey: ['gamification-stats'],
    queryFn: () => getGamificationStats(),
    refetchInterval: 10000,
  });

  const onlineCount = devices?.filter((d) => d.online).length ?? 0;
  const totalCount = devices?.length ?? 0;
  const activeJobs = jobs?.filter((j) => j.status === 'pending' || j.status === 'in_progress').length ?? 0;
  const fwCount = firmware?.length ?? 0;

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-bold text-fg">Overview</h1>

      {/* Stats */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard label="Devices Online" value={`${onlineCount}/${totalCount}`} color="green" />
        <StatCard label="Total Devices" value={String(totalCount)} color="blue" />
        <StatCard label="Active OTA Jobs" value={String(activeJobs)} color="orange" />
        <StatCard label="Firmware Versions" value={String(fwCount)} color="purple" />
      </div>

      {/* Gamification stats */}
      {stats && (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <Link to="/achievements" className="block">
            <div className="rounded-lg border bg-amber-500/10 border-amber-500/20 p-4 hover:bg-amber-500/15 transition-colors">
              <p className="text-xs text-subtle mb-1">Level {stats.level}</p>
              <p className="text-2xl font-bold dark:text-amber-400 light:text-amber-600">{stats.title}</p>
              <div className="mt-2 h-1.5 bg-inset rounded-full overflow-hidden">
                <div
                  className="h-full bg-amber-500 rounded-full transition-all"
                  style={{
                    width: `${stats.xp_for_next_level > stats.xp_for_current_level
                      ? ((stats.total_xp - stats.xp_for_current_level) / (stats.xp_for_next_level - stats.xp_for_current_level)) * 100
                      : 100}%`,
                  }}
                />
              </div>
            </div>
          </Link>
          <Link to="/analytics" className="block">
            <div className="rounded-lg border bg-red-500/10 border-red-500/20 p-4 hover:bg-red-500/15 transition-colors">
              <p className="text-xs text-subtle mb-1">Total XP</p>
              <p className="text-2xl font-bold text-red-400">{stats.total_xp.toLocaleString()}</p>
            </div>
          </Link>
          <Link to="/activity" className="block">
            <div className="rounded-lg border bg-cyan-500/10 border-cyan-500/20 p-4 hover:bg-cyan-500/15 transition-colors">
              <p className="text-xs text-subtle mb-1">Tool Uses</p>
              <p className="text-2xl font-bold text-cyan-400">{stats.total_tool_uses.toLocaleString()}</p>
            </div>
          </Link>
          <Link to="/achievements" className="block">
            <div className="rounded-lg border bg-orange-500/10 border-orange-500/20 p-4 hover:bg-orange-500/15 transition-colors">
              <p className="text-xs text-subtle mb-1">Coding Streak</p>
              <p className="text-2xl font-bold text-orange-400">{stats.current_streak} days</p>
            </div>
          </Link>
        </div>
      )}

      {/* Device quick list */}
      {devices && devices.length > 0 && (
        <div className="rounded-lg border border-edge bg-surface overflow-hidden">
          <div className="px-4 py-3 border-b border-edge flex items-center justify-between">
            <span className="text-sm font-medium text-fg-2">Devices</span>
            <Link to="/devices" className="text-xs text-red-400 hover:text-red-300">View all</Link>
          </div>
          <table className="w-full text-sm">
            <tbody>
              {devices.slice(0, 5).map((d) => (
                <tr key={d.id} className="border-b border-edge/50 last:border-0">
                  <td className="px-4 py-3">
                    <Link to={`/devices/${d.id}`} className="text-fg hover:text-red-400 font-medium">
                      {d.name}
                    </Link>
                  </td>
                  <td className="px-4 py-3 text-subtle text-xs font-mono">{d.ip_address}</td>
                  <td className="px-4 py-3">
                    {d.device_type && (
                      <span className={`text-[10px] px-1.5 py-0.5 rounded-full border ${
                        d.device_type === 'esp32_4848s040c_lcd'
                          ? 'border-cyan-800 text-cyan-400 bg-cyan-900/20'
                          : 'border-amber-800 text-amber-400 bg-amber-900/20'
                      }`}>
                        {d.device_type === 'esp32_4848s040c_lcd' ? 'LCD' : 'OLED'}
                      </span>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    {d.latest_status ? (
                      <StateIndicator state={d.latest_status.state} />
                    ) : (
                      <span className="text-xs text-dim">-</span>
                    )}
                  </td>
                  <td className="px-4 py-3 text-right">
                    <span className={`inline-block w-2 h-2 rounded-full ${d.online ? 'bg-green-500' : 'bg-dim'}`} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function StatCard({ label, value, color }: { label: string; value: string; color: string }) {
  const bg: Record<string, string> = {
    green: 'bg-green-500/10 border-green-500/20',
    blue: 'bg-blue-500/10 border-blue-500/20',
    orange: 'bg-orange-500/10 border-orange-500/20',
    purple: 'bg-purple-500/10 border-purple-500/20',
  };
  const text: Record<string, string> = {
    green: 'text-green-400',
    blue: 'text-blue-400',
    orange: 'text-orange-400',
    purple: 'text-purple-400',
  };
  return (
    <div className={`rounded-lg border p-4 ${bg[color]}`}>
      <p className="text-xs text-subtle mb-1">{label}</p>
      <p className={`text-2xl font-bold ${text[color]}`}>{value}</p>
    </div>
  );
}
