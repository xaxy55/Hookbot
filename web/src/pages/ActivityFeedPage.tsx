import { useQuery } from '@tanstack/react-query';
import { getActivity, getDevices } from '../api/client';
import { useState } from 'react';

const EVENT_COLORS: Record<string, string> = {
  PreToolUse: 'text-yellow-400',
  PostToolUse: 'text-green-400',
  UserPromptSubmit: 'text-blue-400',
  TaskCompleted: 'text-purple-400',
  Stop: 'text-gray-400',
};

const EVENT_LABELS: Record<string, string> = {
  PreToolUse: 'Tool Start',
  PostToolUse: 'Tool End',
  UserPromptSubmit: 'Prompt',
  TaskCompleted: 'Task Done',
  Stop: 'Session End',
};

const TOOL_ICONS: Record<string, string> = {
  Read: 'eye',
  Write: 'pencil',
  Edit: 'edit',
  Bash: 'terminal',
  Glob: 'search',
  Grep: 'search',
  Agent: 'bot',
  WebSearch: 'globe',
  WebFetch: 'download',
  unknown: 'zap',
};

function ToolIcon({ tool }: { tool: string }) {
  const icon = TOOL_ICONS[tool] || TOOL_ICONS['unknown'];
  return (
    <div className="w-8 h-8 rounded-lg bg-gray-800 flex items-center justify-center text-xs font-mono text-gray-300 flex-shrink-0" title={tool}>
      {icon === 'terminal' && <TerminalSvg />}
      {icon === 'eye' && <EyeSvg />}
      {icon === 'pencil' && <PencilSvg />}
      {icon === 'edit' && <PencilSvg />}
      {icon === 'search' && <SearchSvg />}
      {icon === 'bot' && <BotSvg />}
      {icon === 'globe' && <GlobeSvg />}
      {icon === 'download' && <DownloadSvg />}
      {icon === 'zap' && <ZapSvg />}
    </div>
  );
}

export default function ActivityFeedPage() {
  const [deviceFilter, setDeviceFilter] = useState<string>('');

  const { data: devices } = useQuery({
    queryKey: ['devices'],
    queryFn: getDevices,
  });

  const { data: activity, isLoading } = useQuery({
    queryKey: ['activity', deviceFilter],
    queryFn: () => getActivity(100, deviceFilter || undefined),
    refetchInterval: 3000,
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-white">Activity Feed</h1>
        <div className="flex items-center gap-3">
          <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
          <span className="text-xs text-gray-500">Live</span>
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

      {isLoading ? (
        <div className="text-gray-500 text-sm">Loading activity...</div>
      ) : !activity || activity.length === 0 ? (
        <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-8 text-center">
          <p className="text-gray-500 text-sm">No activity recorded yet.</p>
          <p className="text-gray-600 text-xs mt-1">Hook events will appear here as they come in.</p>
        </div>
      ) : (
        <div className="rounded-lg border border-gray-800 bg-gray-900/50 overflow-hidden">
          <div className="divide-y divide-gray-800/50">
            {activity.map((entry) => (
              <div key={entry.id} className="px-4 py-3 flex items-center gap-3 hover:bg-gray-800/30 transition-colors">
                <ToolIcon tool={entry.tool_name} />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-white">{entry.tool_name}</span>
                    <span className={`text-xs px-1.5 py-0.5 rounded ${EVENT_COLORS[entry.event] || 'text-gray-400'} bg-gray-800`}>
                      {EVENT_LABELS[entry.event] || entry.event}
                    </span>
                  </div>
                </div>
                <div className="flex items-center gap-3 flex-shrink-0">
                  {entry.xp_earned > 0 && (
                    <span className="text-xs font-medium text-amber-400">+{entry.xp_earned} XP</span>
                  )}
                  <span className="text-xs text-gray-600 font-mono w-20 text-right">
                    {formatTime(entry.created_at)}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso + 'Z');
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    if (diffMs < 60000) return `${Math.floor(diffMs / 1000)}s ago`;
    if (diffMs < 3600000) return `${Math.floor(diffMs / 60000)}m ago`;
    if (diffMs < 86400000) return `${Math.floor(diffMs / 3600000)}h ago`;
    return d.toLocaleDateString();
  } catch {
    return iso;
  }
}

// Inline SVG icons
function TerminalSvg() {
  return <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M4 5l3 3-3 3M8 12h4" strokeLinecap="round" /></svg>;
}
function EyeSvg() {
  return <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M1 8s2.5-4 7-4 7 4 7 4-2.5 4-7 4-7-4-7-4z" /><circle cx="8" cy="8" r="2" /></svg>;
}
function PencilSvg() {
  return <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M11.5 1.5l3 3L5 14H2v-3L11.5 1.5z" /></svg>;
}
function SearchSvg() {
  return <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5"><circle cx="7" cy="7" r="4" /><path d="M10 10l4 4" strokeLinecap="round" /></svg>;
}
function BotSvg() {
  return <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5"><rect x="3" y="5" width="10" height="8" rx="1.5" /><circle cx="6" cy="9" r="1" fill="currentColor" stroke="none" /><circle cx="10" cy="9" r="1" fill="currentColor" stroke="none" /><path d="M8 2v3M5 2h6" strokeLinecap="round" /></svg>;
}
function GlobeSvg() {
  return <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5"><circle cx="8" cy="8" r="6" /><path d="M2 8h12M8 2c-2 2-2 10 0 12M8 2c2 2 2 10 0 12" /></svg>;
}
function DownloadSvg() {
  return <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M8 2v8M5 7l3 3 3-3M2 12v2h12v-2" strokeLinecap="round" /></svg>;
}
function ZapSvg() {
  return <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5"><path d="M9 1L3 9h4l-1 6 6-8H8l1-6z" /></svg>;
}
