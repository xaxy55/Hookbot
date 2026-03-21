import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getDeskOccupancyConfig,
  updateDeskOccupancyConfig,
  recordOccupancyEvent,
  getOccupancyEvents,
  getOccupancyReport,
} from '../api/client';
import { useToast } from '../hooks/useToast';

export default function DeskOccupancyPage() {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const [selectedDevice, setSelectedDevice] = useState('');

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: config } = useQuery({
    queryKey: ['occupancyConfig', deviceId],
    queryFn: () => getDeskOccupancyConfig(deviceId),
    enabled: !!deviceId,
  });

  const { data: events } = useQuery({
    queryKey: ['occupancyEvents', deviceId],
    queryFn: () => getOccupancyEvents(deviceId),
    enabled: !!deviceId,
    refetchInterval: 10000,
  });

  const { data: report } = useQuery({
    queryKey: ['occupancyReport', deviceId],
    queryFn: () => getOccupancyReport(deviceId),
    enabled: !!deviceId,
  });

  const eventMutation = useMutation({
    mutationFn: (eventType: string) => recordOccupancyEvent(eventType, deviceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['occupancyEvents'] });
      queryClient.invalidateQueries({ queryKey: ['occupancyReport'] });
      toast('Event recorded', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const updateMutation = useMutation({
    mutationFn: (data: { break_remind_minutes?: number; enabled?: boolean }) =>
      updateDeskOccupancyConfig(data, deviceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['occupancyConfig'] });
      toast('Settings updated', 'success');
    },
  });

  const eventButtons = [
    { type: 'occupied', label: 'At Desk', icon: '🖥️' },
    { type: 'vacant', label: 'Left Desk', icon: '🚶' },
    { type: 'break_start', label: 'Break Start', icon: '☕' },
    { type: 'break_end', label: 'Break End', icon: '💪' },
  ];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-heading">Desk Occupancy</h1>
          <p className="text-sm text-muted mt-1">Track desk usage patterns and optimize break times</p>
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

      {/* Quick Actions */}
      <div className="grid grid-cols-4 gap-3">
        {eventButtons.map(btn => (
          <button
            key={btn.type}
            onClick={() => eventMutation.mutate(btn.type)}
            className="rounded-xl border border-border bg-surface p-4 text-center hover:border-brand/50 transition-all"
          >
            <span className="text-3xl block mb-2">{btn.icon}</span>
            <span className="text-sm font-medium text-heading">{btn.label}</span>
          </button>
        ))}
      </div>

      {/* Report */}
      {report && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="rounded-xl border border-border bg-surface p-4 text-center">
            <p className="text-2xl font-bold text-heading">{report.total_desk_hours.toFixed(1)}h</p>
            <p className="text-xs text-muted">Desk Time (7d)</p>
          </div>
          <div className="rounded-xl border border-border bg-surface p-4 text-center">
            <p className="text-2xl font-bold text-heading">{report.total_break_hours.toFixed(1)}h</p>
            <p className="text-xs text-muted">Break Time (7d)</p>
          </div>
          <div className="rounded-xl border border-border bg-surface p-4 text-center">
            <p className="text-2xl font-bold text-heading">{Math.round(report.avg_session_minutes)}m</p>
            <p className="text-xs text-muted">Avg Session</p>
          </div>
          <div className="rounded-xl border border-border bg-surface p-4 text-center">
            <p className="text-2xl font-bold text-heading">{report.breaks_taken}</p>
            <p className="text-xs text-muted">Breaks Taken</p>
          </div>
        </div>
      )}

      {/* Suggestion */}
      {report && (
        <div className="rounded-xl border border-brand/20 bg-brand/5 p-4">
          <p className="text-sm font-medium text-heading">💡 {report.optimal_break_suggestion}</p>
        </div>
      )}

      {/* Daily Stats */}
      {report && report.daily_stats.length > 0 && (
        <div className="rounded-xl border border-border bg-surface p-4">
          <h2 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Daily Breakdown</h2>
          <div className="space-y-2">
            {report.daily_stats.map(day => (
              <div key={day.date} className="flex items-center justify-between py-1.5 border-b border-border/50 last:border-0">
                <span className="text-sm text-muted w-24">{day.date}</span>
                <span className="text-sm">{day.desk_hours.toFixed(1)}h desk</span>
                <span className="text-sm">{day.break_count} breaks</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Settings */}
      <div className="rounded-xl border border-border bg-surface p-4">
        <h2 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Settings</h2>
        <div className="flex items-center gap-4">
          <div>
            <label className="block text-xs text-muted mb-1">Break reminder interval (minutes)</label>
            <input
              type="number"
              className="w-32 rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
              value={config?.break_remind_minutes || 60}
              onChange={e => updateMutation.mutate({ break_remind_minutes: parseInt(e.target.value) })}
            />
          </div>
          <label className="flex items-center gap-2 cursor-pointer mt-4">
            <input
              type="checkbox"
              checked={config?.enabled ?? true}
              onChange={() => updateMutation.mutate({ enabled: !config?.enabled })}
              className="rounded"
            />
            <span className="text-sm">Enable tracking</span>
          </label>
        </div>
      </div>

      {/* Recent Events */}
      {events && events.length > 0 && (
        <div className="rounded-xl border border-border bg-surface p-4">
          <h2 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Recent Events</h2>
          <div className="space-y-1 max-h-64 overflow-y-auto">
            {events.slice(0, 20).map(evt => (
              <div key={evt.id} className="flex items-center justify-between py-1 text-sm">
                <span className="capitalize">{evt.event_type.replace('_', ' ')}</span>
                <span className="text-xs text-muted">{new Date(evt.created_at).toLocaleString()}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
