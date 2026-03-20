import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getMonitorConfig,
  updateMonitorConfig,
  setActiveMonitor,
} from '../api/client';
import { useToast } from '../hooks/useToast';

export default function MonitorsPage() {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const [selectedDevice, setSelectedDevice] = useState('');

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: config } = useQuery({
    queryKey: ['monitorConfig', deviceId],
    queryFn: () => getMonitorConfig(deviceId),
    enabled: !!deviceId,
  });

  const setActiveMutation = useMutation({
    mutationFn: (monitor: number) => setActiveMonitor(monitor, deviceId),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['monitorConfig'] });
      toast(`Looking at monitor ${data.target_angle}°`, 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const updateMutation = useMutation({
    mutationFn: (data: Parameters<typeof updateMonitorConfig>[0]) => updateMonitorConfig(data, deviceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['monitorConfig'] });
      toast('Config updated', 'success');
    },
  });

  const monitors = Array.from({ length: config?.monitor_count || 2 }, (_, i) => i);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-heading">Multi-Monitor</h1>
          <p className="text-sm text-muted mt-1">Detect active monitor and point hookbot toward it via servo</p>
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

      {/* Monitor Visualization */}
      <div className="rounded-xl border border-border bg-surface p-6">
        <div className="flex items-end justify-center gap-4 mb-6">
          {monitors.map(i => {
            const isActive = i === config?.active_monitor;
            const angle = config?.angle_map?.[String(i)] || 90;
            return (
              <button
                key={i}
                onClick={() => setActiveMutation.mutate(i)}
                className={`relative rounded-lg border-2 transition-all ${
                  isActive
                    ? 'border-brand bg-brand/10 shadow-lg shadow-brand/20'
                    : 'border-border bg-surface hover:border-brand/30'
                }`}
                style={{ width: `${100 / (config?.monitor_count || 2)}%`, maxWidth: 200, aspectRatio: '16/10' }}
              >
                <div className="absolute inset-0 flex flex-col items-center justify-center">
                  <span className="text-2xl mb-1">{isActive ? '🖥️' : '🖥'}</span>
                  <span className="text-xs font-medium">Monitor {i + 1}</span>
                  <span className="text-[10px] text-muted">{angle}°</span>
                </div>
                {isActive && (
                  <div className="absolute -bottom-3 left-1/2 -translate-x-1/2">
                    <span className="text-xs bg-brand text-white rounded-full px-2 py-0.5">Active</span>
                  </div>
                )}
              </button>
            );
          })}
        </div>

        {/* Hookbot representation */}
        <div className="text-center mt-8">
          <div className="inline-block relative">
            <span className="text-4xl" style={{
              display: 'inline-block',
              transform: `rotate(${((config?.angle_map?.[String(config?.active_monitor || 0)] || 90) - 90) * 0.5}deg)`,
              transition: 'transform 0.5s ease',
            }}>🤖</span>
          </div>
          <p className="text-xs text-muted mt-2">
            Hookbot {config?.servo_pin ? `(servo pin ${config.servo_pin})` : '(no servo configured)'}
          </p>
        </div>
      </div>

      {/* Configuration */}
      <div className="rounded-xl border border-border bg-surface p-4 space-y-4">
        <h2 className="text-sm font-semibold text-muted uppercase tracking-wider">Configuration</h2>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-xs text-muted mb-1">Number of Monitors</label>
            <input
              type="number"
              min={1}
              max={4}
              className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
              value={config?.monitor_count || 2}
              onChange={e => updateMutation.mutate({ monitor_count: parseInt(e.target.value) })}
            />
          </div>
          <div>
            <label className="block text-xs text-muted mb-1">Servo Pin</label>
            <input
              type="number"
              className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
              value={config?.servo_pin || ''}
              onChange={e => updateMutation.mutate({ servo_pin: parseInt(e.target.value) || undefined })}
              placeholder="GPIO pin"
            />
          </div>
          <div>
            <label className="block text-xs text-muted mb-1">Detection Method</label>
            <select
              className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
              value={config?.detection_method || 'manual'}
              onChange={e => updateMutation.mutate({ detection_method: e.target.value })}
            >
              <option value="manual">Manual</option>
              <option value="usb">USB Detection</option>
            </select>
          </div>
          <div className="flex items-end">
            <label className="flex items-center gap-2 cursor-pointer pb-2">
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

        {/* Angle Map */}
        <div>
          <label className="block text-xs text-muted mb-2">Servo Angles per Monitor</label>
          <div className="flex gap-3">
            {monitors.map(i => (
              <div key={i} className="flex items-center gap-1">
                <span className="text-xs text-muted">M{i + 1}:</span>
                <input
                  type="number"
                  min={0}
                  max={180}
                  className="w-16 rounded border border-border bg-canvas px-2 py-1 text-sm text-center"
                  value={config?.angle_map?.[String(i)] || 90}
                  onChange={e => {
                    const newMap = { ...(config?.angle_map || {}) };
                    newMap[String(i)] = parseInt(e.target.value);
                    updateMutation.mutate({ angle_map: newMap });
                  }}
                />
                <span className="text-xs text-muted">°</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
