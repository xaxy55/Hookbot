import { useQuery } from '@tanstack/react-query';
import { getAchievements, getGamificationStats, getLeaderboard, getDevices } from '../api/client';
import { useState, type JSX } from 'react';

const BADGE_ICONS: Record<string, (props: { className?: string }) => JSX.Element> = {
  zap: ZapIcon,
  upload: UploadIcon,
  layers: LayersIcon,
  star: StarIcon,
  crown: CrownIcon,
  clock: ClockIcon,
  moon: MoonIcon,
  sunrise: SunriseIcon,
  bolt: BoltIcon,
  fire: FireIcon,
  gem: GemIcon,
  trophy: TrophyIcon,
  palette: PaletteIcon,
};

export default function AchievementsPage() {
  const [deviceFilter, setDeviceFilter] = useState<string>('');

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });

  const { data: stats } = useQuery({
    queryKey: ['gamification-stats', deviceFilter],
    queryFn: () => getGamificationStats(deviceFilter || undefined),
    refetchInterval: 10000,
  });

  const { data: badges } = useQuery({
    queryKey: ['achievements', deviceFilter],
    queryFn: () => getAchievements(deviceFilter || undefined),
    refetchInterval: 10000,
  });

  const { data: leaderboard } = useQuery({
    queryKey: ['leaderboard'],
    queryFn: getLeaderboard,
    refetchInterval: 30000,
  });

  const earnedCount = badges?.filter((b) => b.earned).length ?? 0;
  const totalBadges = badges?.length ?? 0;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-fg">Achievements</h1>
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

      {/* XP / Level card */}
      {stats && (
        <div className="rounded-lg border border-amber-500/20 bg-amber-500/5 p-6">
          <div className="flex items-center justify-between mb-4">
            <div>
              <p className="text-xs text-subtle mb-1">Level {stats.level}</p>
              <p className="text-2xl font-bold text-amber-400">{stats.title}</p>
            </div>
            <div className="text-right">
              <p className="text-xs text-subtle">Total XP</p>
              <p className="text-2xl font-bold text-amber-400">{stats.total_xp.toLocaleString()}</p>
            </div>
          </div>
          {/* XP progress bar */}
          <div className="space-y-1">
            <div className="flex justify-between text-xs text-subtle">
              <span>Level {stats.level}</span>
              <span>{stats.total_xp - stats.xp_for_current_level} / {stats.xp_for_next_level - stats.xp_for_current_level} XP</span>
              <span>Level {stats.level + 1}</span>
            </div>
            <div className="h-3 bg-inset rounded-full overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-amber-600 to-amber-400 rounded-full transition-all"
                style={{
                  width: `${stats.xp_for_next_level > stats.xp_for_current_level
                    ? ((stats.total_xp - stats.xp_for_current_level) / (stats.xp_for_next_level - stats.xp_for_current_level)) * 100
                    : 100}%`,
                }}
              />
            </div>
          </div>
          {/* Streak */}
          <div className="flex items-center gap-6 mt-4">
            <div className="flex items-center gap-2">
              <FireIcon className="text-orange-400" />
              <span className="text-sm text-fg-2">{stats.current_streak} day streak</span>
            </div>
            <div className="flex items-center gap-2">
              <TrophyIcon className="text-yellow-400" />
              <span className="text-sm text-fg-2">Best: {stats.longest_streak} days</span>
            </div>
            <div className="flex items-center gap-2">
              <StarIcon className="text-purple-400" />
              <span className="text-sm text-fg-2">{earnedCount}/{totalBadges} badges</span>
            </div>
          </div>
        </div>
      )}

      {/* Badges grid */}
      {badges && (
        <div>
          <h2 className="text-sm font-medium text-muted mb-3">Badges</h2>
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 gap-3">
            {badges.map((badge) => {
              const IconComponent = BADGE_ICONS[badge.icon] || ZapIcon;
              return (
                <div
                  key={badge.id}
                  className={`rounded-lg border p-4 text-center transition-all ${
                    badge.earned
                      ? 'border-amber-500/30 bg-amber-500/5 hover:bg-amber-500/10'
                      : 'border-edge bg-surface/30 opacity-40'
                  }`}
                >
                  <div className={`inline-flex items-center justify-center w-10 h-10 rounded-full mb-2 ${
                    badge.earned ? 'bg-amber-500/20' : 'bg-inset'
                  }`}>
                    <IconComponent className={badge.earned ? 'text-amber-400' : 'text-dim'} />
                  </div>
                  <p className={`text-xs font-medium ${badge.earned ? 'text-fg' : 'text-dim'}`}>{badge.name}</p>
                  <p className="text-[10px] text-subtle mt-0.5">{badge.description}</p>
                  {badge.earned && badge.earned_at && (
                    <p className="text-[9px] text-amber-600 mt-1">
                      {new Date(badge.earned_at + 'Z').toLocaleDateString()}
                    </p>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Leaderboard */}
      {leaderboard && leaderboard.length > 1 && (
        <div>
          <h2 className="text-sm font-medium text-muted mb-3">Leaderboard</h2>
          <div className="rounded-lg border border-edge bg-surface overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-edge text-subtle text-xs">
                  <th className="px-4 py-2 text-left">#</th>
                  <th className="px-4 py-2 text-left">Device</th>
                  <th className="px-4 py-2 text-right">Level</th>
                  <th className="px-4 py-2 text-right">XP</th>
                  <th className="px-4 py-2 text-right">Streak</th>
                  <th className="px-4 py-2 text-right">Badges</th>
                </tr>
              </thead>
              <tbody>
                {leaderboard.map((entry, i) => (
                  <tr key={entry.device_id} className="border-b border-edge/50 last:border-0">
                    <td className="px-4 py-2">
                      <span className={`font-bold ${i === 0 ? 'text-amber-400' : i === 1 ? 'text-fg-2' : i === 2 ? 'text-orange-600' : 'text-subtle'}`}>
                        {i + 1}
                      </span>
                    </td>
                    <td className="px-4 py-2 text-fg font-medium">{entry.device_name}</td>
                    <td className="px-4 py-2 text-right text-fg-2">{entry.level}</td>
                    <td className="px-4 py-2 text-right text-amber-400 font-mono">{entry.total_xp.toLocaleString()}</td>
                    <td className="px-4 py-2 text-right text-orange-400">{entry.current_streak}d</td>
                    <td className="px-4 py-2 text-right text-purple-400">{entry.achievements}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}

// --- Badge Icons ---
function ZapIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M9 1L3 9h4l-1 6 6-8H8l1-6z" /></svg>;
}
function UploadIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M8 10V3M8 3l-3 3M8 3l3 3M2 12v1a1 1 0 001 1h10a1 1 0 001-1v-1" /></svg>;
}
function LayersIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M8 1L1 5l7 4 7-4-7-4zM1 8l7 4 7-4M1 11l7 4 7-4" /></svg>;
}
function StarIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M8 1l2.2 4.5 5 .7-3.6 3.5.8 5L8 12.5 3.6 14.7l.8-5L.8 6.2l5-.7L8 1z" /></svg>;
}
function CrownIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M2 12h12l1.5-7-3.5 3L8 2 4 8 .5 5 2 12z" /></svg>;
}
function ClockIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><circle cx="8" cy="8" r="6" /><path d="M8 4v4l3 2" strokeLinecap="round" /></svg>;
}
function MoonIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M13 9.5A5.5 5.5 0 016.5 3 5.5 5.5 0 1013 9.5z" /></svg>;
}
function SunriseIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M1 12h14M3 9.5l1 1M12 9.5l1 1M8 3v3M4.5 7A3.5 3.5 0 018 5.5 3.5 3.5 0 0111.5 7" strokeLinecap="round" /></svg>;
}
function BoltIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M9 1L3 9h4l-1 6 6-8H8l1-6z" /></svg>;
}
function FireIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M8 1C6 4 4 5 4 8a4 4 0 008 0c0-3-2-4-4-7z" /><path d="M8 14a2 2 0 002-2c0-1.5-2-2.5-2-4-1 1.5-2 2.5-2 4a2 2 0 002 2z" /></svg>;
}
function GemIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M3 2h10l3 5-8 8-8-8 3-5zM1 7h14M8 15L5.5 7 8 2l2.5 5L8 15" /></svg>;
}
function TrophyIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><path d="M4 2h8v5a4 4 0 01-8 0V2zM4 4H2a1 1 0 00-1 1v1a2 2 0 002 2h1M12 4h2a1 1 0 011 1v1a2 2 0 01-2 2h-1M6 12h4M8 10v2M5 14h6" /></svg>;
}
function PaletteIcon({ className }: { className?: string }) {
  return <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={className}><circle cx="8" cy="8" r="6.5" /><circle cx="5" cy="6" r="1" fill="currentColor" stroke="none" /><circle cx="8" cy="4.5" r="1" fill="currentColor" stroke="none" /><circle cx="11" cy="6" r="1" fill="currentColor" stroke="none" /><circle cx="5" cy="10" r="1" fill="currentColor" stroke="none" /></svg>;
}
