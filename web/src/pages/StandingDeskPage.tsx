import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getStandingDesk,
  updateStandingDesk,
  changePosition,
  getDeskReport,
} from '../api/client';
import { useToast } from '../hooks/useToast';

export default function StandingDeskPage() {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const [selectedDevice, setSelectedDevice] = useState('');

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: config } = useQuery({
    queryKey: ['standingDesk', deviceId],
    queryFn: () => getStandingDesk(deviceId),
    enabled: !!deviceId,
  });

  const { data: report } = useQuery({
    queryKey: ['deskReport', deviceId],
    queryFn: () => getDeskReport(deviceId),
    enabled: !!deviceId,
  });

  const positionMutation = useMutation({
    mutationFn: (pos: string) => changePosition(pos, deviceId),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['standingDesk'] });
      queryClient.invalidateQueries({ queryKey: ['deskReport'] });
      toast(data.message, 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const updateMutation = useMutation({
    mutationFn: (data: { sit_remind_minutes?: number; stand_remind_minutes?: number; enabled?: boolean }) =>
      updateStandingDesk(data, deviceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['standingDesk'] });
      toast('Settings updated', 'success');
    },
  });

  const isStanding = config?.current_position === 'standing';
  const totalMinutes = (config?.total_stand_minutes || 0) + (config?.total_sit_minutes || 0);
  const standPercent = totalMinutes > 0 ? Math.round(((config?.total_stand_minutes || 0) / totalMinutes) * 100) : 0;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-heading">Standing Desk</h1>
          <p className="text-sm text-muted mt-1">Track sit/stand time and get reminders to switch</p>
        </div>
        {devices && devices.length > 1 && (
          <select
            className="rounded-lg border border-border bg-surface px-3 py-2 text-sm"
            value={selectedDevice}
            onChange={e => setSelectedDevice(e.target.value)}
          >
            {devices.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
        )}
      </div>

      {/* Position Toggle */}
      <div className="rounded-xl border border-border bg-surface p-6 text-center">
        <p className="text-6xl mb-4">{isStanding ? '🧍' : '🪑'}</p>
        <p className="text-xl font-bold text-heading mb-1">
          Currently {isStanding ? 'Standing' : 'Sitting'}
        </p>
        {config?.last_transition_at && (
          <p className="text-sm text-muted mb-4">Since {new Date(config.last_transition_at).toLocaleTimeString()}</p>
        )}
        <button
          onClick={() => positionMutation.mutate(isStanding ? 'sitting' : 'standing')}
          className="rounded-xl bg-brand px-8 py-3 text-lg font-semibold text-white hover:bg-brand/90"
        >
          Switch to {isStanding ? 'Sitting' : 'Standing'}
        </button>
        <p className="text-xs text-muted mt-2">Transitions today: {config?.transitions_today || 0}</p>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-3 gap-4">
        <div className="rounded-xl border border-border bg-surface p-4 text-center">
          <p className="text-2xl font-bold text-heading">{Math.round((config?.total_stand_minutes || 0) / 60)}h</p>
          <p className="text-xs text-muted">Standing</p>
        </div>
        <div className="rounded-xl border border-border bg-surface p-4 text-center">
          <p className="text-2xl font-bold text-heading">{Math.round((config?.total_sit_minutes || 0) / 60)}h</p>
          <p className="text-xs text-muted">Sitting</p>
        </div>
        <div className="rounded-xl border border-border bg-surface p-4 text-center">
          <p className="text-2xl font-bold text-heading">{standPercent}%</p>
          <p className="text-xs text-muted">Stand Ratio</p>
        </div>
      </div>

      {/* Health Report */}
      {report && report.daily_history.length > 0 && (
        <div className="rounded-xl border border-border bg-surface p-4">
          <h2 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Weekly History</h2>
          <div className="space-y-2">
            {report.daily_history.map(day => (
              <div key={day.date} className="flex items-center gap-3">
                <span className="text-xs text-muted w-20">{day.date}</span>
                <div className="flex-1 h-4 bg-canvas rounded-full overflow-hidden flex">
                  <div
                    className="h-full bg-green-500"
                    style={{ width: `${day.stand_minutes + day.sit_minutes > 0 ? (day.stand_minutes / (day.stand_minutes + day.sit_minutes)) * 100 : 0}%` }}
                  />
                  <div className="h-full bg-blue-500 flex-1" />
                </div>
                <span className="text-xs text-muted w-12 text-right">{day.transitions} sw</span>
              </div>
            ))}
          </div>
          <div className="flex gap-4 mt-2 text-xs text-muted">
            <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-full bg-green-500" /> Standing</span>
            <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-full bg-blue-500" /> Sitting</span>
          </div>
        </div>
      )}

      {/* Settings */}
      <div className="rounded-xl border border-border bg-surface p-4">
        <h2 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Reminder Settings</h2>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-xs text-muted mb-1">Sitting reminder (minutes)</label>
            <input
              type="number"
              className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
              value={config?.sit_remind_minutes || 45}
              onChange={e => updateMutation.mutate({ sit_remind_minutes: parseInt(e.target.value) })}
            />
          </div>
          <div>
            <label className="block text-xs text-muted mb-1">Standing reminder (minutes)</label>
            <input
              type="number"
              className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
              value={config?.stand_remind_minutes || 15}
              onChange={e => updateMutation.mutate({ stand_remind_minutes: parseInt(e.target.value) })}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
