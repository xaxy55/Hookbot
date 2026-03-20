import { useQuery } from '@tanstack/react-query';
import { getTeamDashboard } from '../api/client';
import type { TeamMember } from '../api/client';

const STATE_COLORS: Record<string, string> = {
  coding: 'bg-green-500',
  happy: 'bg-blue-500',
  thinking: 'bg-yellow-500',
  error: 'bg-red-500',
  excited: 'bg-purple-500',
  sleeping: 'bg-gray-400',
  idle: 'bg-gray-400',
};

export default function TeamDashboardPage() {
  const { data: team } = useQuery({
    queryKey: ['team-dashboard'],
    queryFn: getTeamDashboard,
    refetchInterval: 3000,
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-fg">Team Dashboard</h1>
        {team && (
          <div className="flex items-center gap-2 text-sm">
            <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            <span className="text-green-600 dark:text-green-400 font-medium">{team.active_count} active</span>
            <span className="text-muted">/ {team.total_count} total</span>
          </div>
        )}
      </div>

      {/* Summary cards */}
      {team && (
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div className="rounded-lg border border-edge bg-surface p-4">
            <div className="text-3xl font-bold text-green-500">{team.active_count}</div>
            <div className="text-sm text-muted">Currently Coding</div>
          </div>
          <div className="rounded-lg border border-edge bg-surface p-4">
            <div className="text-3xl font-bold text-brand">
              {team.members.reduce((sum, m) => sum + m.total_xp, 0).toLocaleString()}
            </div>
            <div className="text-sm text-muted">Total Team XP</div>
          </div>
          <div className="rounded-lg border border-edge bg-surface p-4">
            <div className="text-3xl font-bold text-amber-500">
              {Math.max(...team.members.map(m => m.current_streak), 0)}
            </div>
            <div className="text-sm text-muted">Best Active Streak</div>
          </div>
        </div>
      )}

      {/* Team members grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {team?.members.map((m: TeamMember) => (
          <div
            key={m.device_id}
            className={`rounded-lg border bg-surface p-4 transition-all ${
              m.is_coding ? 'border-green-500/40 shadow-sm shadow-green-500/10' : 'border-edge'
            }`}
          >
            <div className="flex items-center gap-3 mb-3">
              <div className={`w-3 h-3 rounded-full ${m.is_coding ? 'bg-green-500 animate-pulse' : 'bg-gray-400'}`} />
              <span className="text-sm font-medium text-fg">{m.device_name}</span>
              <span className={`ml-auto text-xs px-2 py-0.5 rounded-full ${
                STATE_COLORS[m.current_state] || 'bg-gray-400'
              }/10 text-fg`}>
                {m.current_state}
              </span>
            </div>

            <div className="grid grid-cols-3 gap-2 text-center">
              <div>
                <div className="text-lg font-bold text-brand">{m.level}</div>
                <div className="text-[11px] text-muted">Level</div>
              </div>
              <div>
                <div className="text-lg font-bold text-fg">{m.total_xp.toLocaleString()}</div>
                <div className="text-[11px] text-muted">XP</div>
              </div>
              <div>
                <div className="text-lg font-bold text-amber-500">{m.current_streak}</div>
                <div className="text-[11px] text-muted">Streak</div>
              </div>
            </div>

            {m.last_activity_at && (
              <div className="text-[11px] text-dim mt-3 text-right">
                Last active: {new Date(m.last_activity_at).toLocaleTimeString()}
              </div>
            )}
          </div>
        ))}
      </div>

      {(!team || team.members.length === 0) && (
        <div className="text-center text-muted py-12">
          <p className="text-sm">No team members yet. Register devices to see them here.</p>
        </div>
      )}
    </div>
  );
}
