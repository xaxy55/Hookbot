import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getDeskLights,
  createDeskLight,
  updateDeskLight,
  deleteDeskLight,
  triggerDeskLightAction,
  type DeskLightConfig,
} from '../api/client';
import { useToast } from '../hooks/useToast';

const DEFAULT_STATE_COLORS: Record<string, string> = {
  idle: '#4488ff',
  working: '#44ff44',
  error: '#ff4444',
  testing: '#ffaa00',
  focus: '#8844ff',
};

export default function DeskLightsPage() {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const [selectedDevice, setSelectedDevice] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [formData, setFormData] = useState({ provider: 'hue', name: '', bridge_ip: '', api_key: '' });

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: lights } = useQuery({
    queryKey: ['deskLights', deviceId],
    queryFn: () => getDeskLights(deviceId),
    enabled: !!deviceId,
  });

  const createMutation = useMutation({
    mutationFn: () => createDeskLight({ device_id: deviceId, ...formData }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['deskLights'] });
      setShowForm(false);
      setFormData({ provider: 'hue', name: '', bridge_ip: '', api_key: '' });
      toast('Light config created', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const toggleMutation = useMutation({
    mutationFn: (light: DeskLightConfig) => updateDeskLight(light.id, { enabled: !light.enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['deskLights'] });
      toast('Light toggled', 'success');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteDeskLight(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['deskLights'] });
      toast('Light config deleted', 'success');
    },
  });

  const actionMutation = useMutation({
    mutationFn: ({ id, color }: { id: string; color: string }) => triggerDeskLightAction(id, { color }),
    onSuccess: () => toast('Light action sent', 'success'),
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-heading">Desk Lighting</h1>
          <p className="text-sm text-muted mt-1">Control Philips Hue and WLED strips synced to hookbot state</p>
        </div>
        <div className="flex gap-2">
          {devices && devices.length > 1 && (
            <select
              className="rounded-lg border border-border bg-surface px-3 py-2 text-sm"
              value={selectedDevice}
              onChange={e => setSelectedDevice(e.target.value)}
            >
              {devices.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
            </select>
          )}
          <button
            onClick={() => setShowForm(!showForm)}
            className="rounded-lg bg-brand px-4 py-2 text-sm font-medium text-white hover:bg-brand/90"
          >
            + Add Light
          </button>
        </div>
      </div>

      {showForm && (
        <div className="rounded-xl border border-border bg-surface p-4 space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-medium text-muted mb-1">Provider</label>
              <select
                className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
                value={formData.provider}
                onChange={e => setFormData(f => ({ ...f, provider: e.target.value }))}
              >
                <option value="hue">Philips Hue</option>
                <option value="wled">WLED</option>
              </select>
            </div>
            <div>
              <label className="block text-xs font-medium text-muted mb-1">Name</label>
              <input
                className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
                value={formData.name}
                onChange={e => setFormData(f => ({ ...f, name: e.target.value }))}
                placeholder="Desk LED Strip"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted mb-1">Bridge IP</label>
              <input
                className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
                value={formData.bridge_ip}
                onChange={e => setFormData(f => ({ ...f, bridge_ip: e.target.value }))}
                placeholder="192.168.1.100"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted mb-1">API Key</label>
              <input
                className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
                value={formData.api_key}
                onChange={e => setFormData(f => ({ ...f, api_key: e.target.value }))}
                placeholder="hue-api-key"
              />
            </div>
          </div>
          <button
            onClick={() => createMutation.mutate()}
            className="rounded-lg bg-brand px-4 py-2 text-sm font-medium text-white hover:bg-brand/90"
          >
            Create
          </button>
        </div>
      )}

      <div className="space-y-4">
        {lights?.map(light => (
          <div key={light.id} className="rounded-xl border border-border bg-surface p-4">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-3">
                <span className="text-2xl">{light.provider === 'hue' ? '💡' : '🌈'}</span>
                <div>
                  <h3 className="font-semibold text-heading">{light.name}</h3>
                  <p className="text-xs text-muted">{light.provider.toUpperCase()} &middot; {light.bridge_ip || 'No bridge'}</p>
                </div>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => toggleMutation.mutate(light)}
                  className={`rounded-lg px-3 py-1.5 text-xs font-medium ${light.enabled ? 'bg-green-500/10 text-green-500' : 'bg-red-500/10 text-red-500'}`}
                >
                  {light.enabled ? 'Enabled' : 'Disabled'}
                </button>
                <button
                  onClick={() => deleteMutation.mutate(light.id)}
                  className="rounded-lg px-3 py-1.5 text-xs font-medium bg-red-500/10 text-red-500 hover:bg-red-500/20"
                >
                  Delete
                </button>
              </div>
            </div>

            <div>
              <p className="text-xs font-medium text-muted mb-2">State Colors (click to preview)</p>
              <div className="flex gap-2 flex-wrap">
                {Object.entries(light.state_colors || DEFAULT_STATE_COLORS).map(([state, color]) => (
                  <button
                    key={state}
                    onClick={() => actionMutation.mutate({ id: light.id, color: color as string })}
                    className="flex items-center gap-1.5 rounded-lg border border-border px-3 py-1.5 text-xs hover:border-brand/50"
                  >
                    <span className="w-3 h-3 rounded-full" style={{ backgroundColor: color as string }} />
                    {state}
                  </button>
                ))}
              </div>
            </div>
          </div>
        ))}

        {lights?.length === 0 && (
          <div className="text-center py-12 text-muted">
            <p className="text-4xl mb-2">💡</p>
            <p>No desk lights configured yet.</p>
            <p className="text-sm">Add a Philips Hue bridge or WLED controller to sync your desk lighting with hookbot states.</p>
          </div>
        )}
      </div>
    </div>
  );
}
