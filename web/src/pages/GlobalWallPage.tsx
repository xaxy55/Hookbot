import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getGlobalEvents, createGlobalEvent, getDevices } from '../api/client';
import type { GlobalEvent } from '../api/client';

const EVENT_ICONS: Record<string, string> = {
  level_up: '\u2B50',
  achievement: '\uD83C\uDFC6',
  streak: '\uD83D\uDD25',
  deploy: '\uD83D\uDE80',
  milestone: '\uD83C\uDF1F',
  general: '\uD83D\uDCE2',
};

const EVENT_COLORS: Record<string, string> = {
  level_up: 'border-yellow-500/30 bg-yellow-500/5',
  achievement: 'border-purple-500/30 bg-purple-500/5',
  streak: 'border-orange-500/30 bg-orange-500/5',
  deploy: 'border-blue-500/30 bg-blue-500/5',
  milestone: 'border-green-500/30 bg-green-500/5',
  general: 'border-edge bg-surface',
};

export default function GlobalWallPage() {
  const qc = useQueryClient();
  const [filter, setFilter] = useState<string>('');
  const { data: events } = useQuery({
    queryKey: ['global-events', filter],
    queryFn: () => getGlobalEvents(100, filter || undefined),
    refetchInterval: 5000,
  });
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });

  const [message, setMessage] = useState('');
  const [eventType, setEventType] = useState('general');
  const [deviceId, setDeviceId] = useState('');
  const [anonymous, setAnonymous] = useState(true);

  const postMut = useMutation({
    mutationFn: () => createGlobalEvent({
      device_id: deviceId || undefined,
      event_type: eventType,
      message,
      anonymous,
    }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ['global-events'] }); setMessage(''); },
  });

  const eventTypes = ['', 'level_up', 'achievement', 'streak', 'deploy', 'milestone', 'general'];

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-bold text-fg">Global Event Wall</h1>
      <p className="text-sm text-muted">Opt-in anonymous feed of hookbot milestones worldwide.</p>

      {/* Post event */}
      <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
        <h3 className="text-sm font-medium text-fg">Share a Milestone</h3>
        <div className="flex gap-2 items-end flex-wrap">
          <select value={eventType} onChange={e => setEventType(e.target.value)}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg">
            {Object.entries(EVENT_ICONS).map(([key, icon]) => (
              <option key={key} value={key}>{icon} {key.replace('_', ' ')}</option>
            ))}
          </select>
          <select value={deviceId} onChange={e => setDeviceId(e.target.value)}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg">
            <option value="">Anonymous</option>
            {devices?.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
          <input value={message} onChange={e => setMessage(e.target.value)}
            placeholder="What happened?"
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg flex-1 min-w-[200px]" />
          <label className="flex items-center gap-1 text-xs text-muted">
            <input type="checkbox" checked={anonymous} onChange={e => setAnonymous(e.target.checked)} />
            Anonymous
          </label>
          <button onClick={() => postMut.mutate()} disabled={!message}
            className="px-4 py-2 rounded-lg bg-brand text-white text-sm font-medium disabled:opacity-50 hover:bg-brand/80">
            Share
          </button>
        </div>
      </div>

      {/* Filter */}
      <div className="flex gap-1 flex-wrap">
        {eventTypes.map(t => (
          <button key={t || 'all'} onClick={() => setFilter(t)}
            className={`px-3 py-1 rounded-full text-xs transition-colors ${
              filter === t ? 'bg-brand/10 text-brand-fg' : 'bg-inset text-muted hover:text-fg'
            }`}>
            {t ? `${EVENT_ICONS[t] || ''} ${t.replace('_', ' ')}` : 'All'}
          </button>
        ))}
      </div>

      {/* Event feed */}
      <div className="space-y-2">
        {events?.map((e: GlobalEvent) => (
          <div key={e.id} className={`rounded-lg border p-4 ${EVENT_COLORS[e.event_type] || EVENT_COLORS.general}`}>
            <div className="flex items-start gap-3">
              <span className="text-xl">{EVENT_ICONS[e.event_type] || '\uD83D\uDCE2'}</span>
              <div className="flex-1 min-w-0">
                <p className="text-sm text-fg">{e.message}</p>
                <div className="flex gap-3 text-[11px] text-dim mt-1">
                  {e.anonymous ? (
                    <span>someone</span>
                  ) : (
                    <span>{e.device_name || 'unknown'}</span>
                  )}
                  <span>{new Date(e.created_at).toLocaleString()}</span>
                  <span className="px-1.5 rounded bg-inset">{e.event_type.replace('_', ' ')}</span>
                </div>
              </div>
            </div>
          </div>
        ))}
        {events?.length === 0 && (
          <div className="text-center text-muted py-12">
            <p className="text-sm">No events yet. Share a milestone to get started!</p>
          </div>
        )}
      </div>
    </div>
  );
}
