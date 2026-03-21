import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import {
  getDevices,
  getFlowStates,
  getCodeQuality,
  getWeeklyDigest,
  getBurnoutCheck,
  getProjectTime,
  getPairProgramming,
  getRetrospective,
} from '../api/client';

export default function DeveloperInsightsPage() {
  const [deviceFilter, setDeviceFilter] = useState<string>('');
  const [days, setDays] = useState(30);

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });

  const did = deviceFilter || undefined;

  const { data: flow } = useQuery({
    queryKey: ['insights-flow', days, did],
    queryFn: () => getFlowStates(days, did),
    refetchInterval: 60000,
  });

  const { data: quality } = useQuery({
    queryKey: ['insights-quality', days, did],
    queryFn: () => getCodeQuality(days, did),
    refetchInterval: 60000,
  });

  const { data: digest } = useQuery({
    queryKey: ['insights-digest', did],
    queryFn: () => getWeeklyDigest(did),
    refetchInterval: 60000,
  });

  const { data: burnout } = useQuery({
    queryKey: ['insights-burnout', days, did],
    queryFn: () => getBurnoutCheck(days, did),
    refetchInterval: 60000,
  });

  const { data: projectTime } = useQuery({
    queryKey: ['insights-projects', days, did],
    queryFn: () => getProjectTime(days, did),
    refetchInterval: 60000,
  });

  const { data: pairing } = useQuery({
    queryKey: ['insights-pairing', days, did],
    queryFn: () => getPairProgramming(days, did),
    refetchInterval: 60000,
  });

  const { data: retro } = useQuery({
    queryKey: ['insights-retro', days, did],
    queryFn: () => getRetrospective(days, did),
    refetchInterval: 60000,
  });

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-fg">Developer Insights</h1>
          <p className="text-sm text-subtle mt-0.5">Deep productivity intelligence for your coding patterns</p>
        </div>
        <div className="flex items-center gap-3">
          <select
            value={days}
            onChange={(e) => setDays(Number(e.target.value))}
            className="bg-inset border border-edge rounded-lg px-3 py-1.5 text-sm text-fg-2"
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
              className="bg-inset border border-edge rounded-lg px-3 py-1.5 text-sm text-fg-2"
            >
              <option value="">All devices</option>
              {devices.map((d) => (
                <option key={d.id} value={d.id}>{d.name}</option>
              ))}
            </select>
          )}
        </div>
      </div>

      {/* Weekly Digest */}
      {digest && (
        <div className="rounded-lg border border-edge bg-surface p-5">
          <h2 className="text-base font-semibold text-fg mb-3">Weekly Digest</h2>
          <div className="grid grid-cols-3 gap-4 mb-4">
            <MiniStat label="Tool Uses" value={digest.this_week.tool_uses} change={digest.change_pct.tool_uses} />
            <MiniStat label="XP Earned" value={digest.this_week.xp_earned} change={digest.change_pct.xp} />
            <MiniStat label="Sessions" value={digest.this_week.sessions} />
          </div>
          {digest.top_tools.length > 0 && (
            <div className="mb-3">
              <p className="text-xs text-subtle mb-1.5 font-medium">Top tools this week</p>
              <div className="flex flex-wrap gap-2">
                {digest.top_tools.map((t) => (
                  <span key={t.tool} className="bg-brand/10 text-brand text-xs px-2 py-0.5 rounded-full">
                    {t.tool} ({t.count})
                  </span>
                ))}
              </div>
            </div>
          )}
          {digest.tips.map((tip, i) => (
            <p key={i} className="text-sm text-fg-2 mt-1">
              <span className="text-amber-500 mr-1">*</span> {tip}
            </p>
          ))}
        </div>
      )}

      {/* Flow State + Burnout side by side */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Flow State */}
        {flow && (
          <div className="rounded-lg border border-edge bg-surface p-5">
            <h2 className="text-base font-semibold text-fg mb-3">Flow State Detection</h2>
            <div className="grid grid-cols-2 gap-3 mb-4">
              <StatBox label="Flow Sessions" value={flow.flow_sessions} />
              <StatBox label="Flow Rate" value={`${flow.flow_rate_pct}%`} />
              <StatBox label="Avg Duration" value={`${flow.avg_flow_duration_min} min`} />
              <StatBox label="Total Sessions" value={flow.total_sessions} />
            </div>
            {flow.peak_flow_hours.length > 0 && (
              <div>
                <p className="text-xs text-subtle mb-2 font-medium">Peak flow hours</p>
                <div className="flex gap-2">
                  {flow.peak_flow_hours.map((h) => (
                    <span key={h.hour} className="bg-green-500/10 text-green-400 text-xs px-2 py-1 rounded">
                      {h.hour}:00 ({h.count})
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {/* Burnout Check */}
        {burnout && (
          <div className="rounded-lg border border-edge bg-surface p-5">
            <h2 className="text-base font-semibold text-fg mb-3">Burnout Early Warning</h2>
            <div className="flex items-center gap-3 mb-4">
              <div className={`text-2xl font-bold ${riskColor(burnout.risk_level)}`}>
                {burnout.risk_score}
              </div>
              <div>
                <div className={`text-sm font-medium ${riskColor(burnout.risk_level)}`}>
                  {burnout.risk_level.charAt(0).toUpperCase() + burnout.risk_level.slice(1)} Risk
                </div>
                <div className="text-xs text-subtle">Burnout risk score (0-100)</div>
              </div>
            </div>
            <div className="grid grid-cols-3 gap-3 mb-3">
              <StatBox label="Late Nights" value={burnout.late_night_days} />
              <StatBox label="Weekend Days" value={burnout.weekend_work_days} />
              <StatBox label="Max Streak" value={`${burnout.max_consecutive_days}d`} />
            </div>
            {burnout.warnings.map((w, i) => (
              <p key={i} className="text-sm text-fg-2 mt-1">
                <span className={burnout.risk_level === 'low' ? 'text-green-500' : 'text-amber-500'}>
                  {burnout.risk_level === 'low' ? '+' : '!'}
                </span>{' '}
                {w}
              </p>
            ))}
          </div>
        )}
      </div>

      {/* Code Quality Correlation */}
      {quality && (
        <div className="rounded-lg border border-edge bg-surface p-5">
          <h2 className="text-base font-semibold text-fg mb-3">Code Quality Correlation</h2>
          {quality.optimal_hour !== null && (
            <p className="text-sm text-fg-2 mb-3">
              Your optimal coding hour: <span className="text-brand font-semibold">{quality.optimal_hour}:00</span>
            </p>
          )}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {/* Hourly productivity */}
            <div>
              <p className="text-xs text-subtle mb-2 font-medium">Productivity by hour</p>
              <div className="space-y-1">
                {quality.xp_by_hour.filter(h => h.count >= 3).map((h) => {
                  const maxXp = Math.max(...quality.xp_by_hour.filter(x => x.count >= 3).map(x => x.avg_xp), 1);
                  return (
                    <div key={h.hour} className="flex items-center gap-2 text-xs">
                      <span className="w-10 text-right text-subtle">{h.hour}:00</span>
                      <div className="flex-1 bg-inset rounded-full h-3 overflow-hidden">
                        <div
                          className="bg-brand h-full rounded-full transition-all"
                          style={{ width: `${(h.avg_xp / maxXp) * 100}%` }}
                        />
                      </div>
                      <span className="w-10 text-subtle">{h.avg_xp.toFixed(1)}</span>
                    </div>
                  );
                })}
              </div>
            </div>
            {/* Day of week */}
            <div>
              <p className="text-xs text-subtle mb-2 font-medium">Activity by day of week</p>
              <div className="space-y-1">
                {quality.by_day_of_week.map((d) => {
                  const maxUses = Math.max(...quality.by_day_of_week.map(x => x.tool_uses), 1);
                  return (
                    <div key={d.day} className="flex items-center gap-2 text-xs">
                      <span className="w-10 text-right text-subtle">{d.day}</span>
                      <div className="flex-1 bg-inset rounded-full h-3 overflow-hidden">
                        <div
                          className="bg-purple-500 h-full rounded-full transition-all"
                          style={{ width: `${(d.tool_uses / maxUses) * 100}%` }}
                        />
                      </div>
                      <span className="w-14 text-subtle">{d.tool_uses} uses</span>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Project Time Tracking */}
      {projectTime && projectTime.projects.length > 0 && (
        <div className="rounded-lg border border-edge bg-surface p-5">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-base font-semibold text-fg">Project Time Tracking</h2>
            <span className="text-sm text-subtle">
              Total: ~{projectTime.total_estimated_hours}h over {projectTime.period_days} days
            </span>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-xs text-subtle border-b border-edge">
                  <th className="pb-2 font-medium">Project</th>
                  <th className="pb-2 font-medium text-right">Tool Uses</th>
                  <th className="pb-2 font-medium text-right">Est. Time</th>
                  <th className="pb-2 font-medium text-right">XP</th>
                  <th className="pb-2 font-medium text-right">Active Days</th>
                </tr>
              </thead>
              <tbody>
                {projectTime.projects.map((p) => (
                  <tr key={p.project} className="border-b border-edge/50">
                    <td className="py-2 text-fg font-medium">{p.project}</td>
                    <td className="py-2 text-right text-fg-2">{p.tool_uses}</td>
                    <td className="py-2 text-right text-fg-2">
                      {p.estimated_minutes >= 60
                        ? `${(p.estimated_minutes / 60).toFixed(1)}h`
                        : `${p.estimated_minutes}m`}
                    </td>
                    <td className="py-2 text-right text-amber-500">{p.total_xp}</td>
                    <td className="py-2 text-right text-fg-2">{p.active_days}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Pair Programming + Retrospective */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Pair Programming */}
        {pairing && (
          <div className="rounded-lg border border-edge bg-surface p-5">
            <h2 className="text-base font-semibold text-fg mb-3">Pair Programming Stats</h2>
            <div className="grid grid-cols-2 gap-3">
              <StatBox label="Intensive Sessions" value={pairing.high_density_sessions} />
              <StatBox label="Pair Rate" value={`${pairing.pair_rate_pct}%`} />
              <StatBox label="Total Sessions" value={pairing.total_sessions} />
              <StatBox label="Multi-Device" value={pairing.concurrent_device_sessions} />
            </div>
          </div>
        )}

        {/* Retrospective */}
        {retro && (
          <div className="rounded-lg border border-edge bg-surface p-5">
            <h2 className="text-base font-semibold text-fg mb-3">
              Sprint Retrospective ({retro.sprint_days}d)
            </h2>
            <div className="grid grid-cols-2 gap-3 mb-3">
              <StatBox label="Tool Uses" value={retro.summary.total_tool_uses} />
              <StatBox label="XP" value={retro.summary.total_xp} />
              <StatBox label="Sessions" value={retro.summary.total_sessions} />
              <StatBox label="Active Days" value={retro.summary.active_days} />
            </div>
            {retro.talking_points.went_well.length > 0 && (
              <div className="mt-3">
                <p className="text-xs font-medium text-green-400 mb-1">Went well</p>
                {retro.talking_points.went_well.map((p, i) => (
                  <p key={i} className="text-sm text-fg-2">+ {p}</p>
                ))}
              </div>
            )}
            {retro.talking_points.to_improve.length > 0 && (
              <div className="mt-2">
                <p className="text-xs font-medium text-amber-400 mb-1">To improve</p>
                {retro.talking_points.to_improve.map((p, i) => (
                  <p key={i} className="text-sm text-fg-2">- {p}</p>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function MiniStat({ label, value, change }: { label: string; value: number; change?: number }) {
  return (
    <div className="bg-inset rounded-lg p-3">
      <p className="text-xs text-subtle">{label}</p>
      <p className="text-lg font-bold text-fg">{value.toLocaleString()}</p>
      {change !== undefined && change !== 0 && (
        <p className={`text-xs ${change > 0 ? 'text-green-400' : 'text-red-400'}`}>
          {change > 0 ? '+' : ''}{change}% vs last week
        </p>
      )}
    </div>
  );
}

function StatBox({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="bg-inset rounded-lg p-2.5 text-center">
      <p className="text-xs text-subtle">{label}</p>
      <p className="text-sm font-semibold text-fg mt-0.5">{value}</p>
    </div>
  );
}

function riskColor(level: string) {
  switch (level) {
    case 'low': return 'text-green-400';
    case 'moderate': return 'text-yellow-400';
    case 'elevated': return 'text-orange-400';
    case 'high': return 'text-red-400';
    default: return 'text-fg';
  }
}
