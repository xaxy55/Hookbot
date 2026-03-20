import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getHomeAssistantConfig,
  createHomeAssistantConfig,
  updateHomeAssistantConfig,
  deleteHomeAssistantConfig,
  getHomeAssistantEntity,
  syncHomeAssistant,
} from '../api/client';
import { useToast } from '../hooks/useToast';

export default function HomeAssistantPage() {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const [selectedDevice, setSelectedDevice] = useState('');
  const [formData, setFormData] = useState({ ha_url: '', access_token: '', entity_id: '' });

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: config } = useQuery({
    queryKey: ['haConfig', deviceId],
    queryFn: () => getHomeAssistantConfig(deviceId),
    enabled: !!deviceId,
  });

  const { data: entity } = useQuery({
    queryKey: ['haEntity', deviceId],
    queryFn: () => getHomeAssistantEntity(deviceId),
    enabled: !!deviceId && !!config,
    refetchInterval: 10000,
  });

  const createMutation = useMutation({
    mutationFn: () => createHomeAssistantConfig({ device_id: deviceId, ...formData }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['haConfig'] });
      toast('Home Assistant connected', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const updateMutation = useMutation({
    mutationFn: (data: Parameters<typeof updateHomeAssistantConfig>[1]) =>
      updateHomeAssistantConfig(config!.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['haConfig'] });
      toast('Config updated', 'success');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => deleteHomeAssistantConfig(config!.id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['haConfig'] });
      toast('Integration removed', 'success');
    },
  });

  const syncMutation = useMutation({
    mutationFn: () => syncHomeAssistant(deviceId),
    onSuccess: () => toast('State synced to Home Assistant', 'success'),
    onError: (e: Error) => toast(e.message, 'error'),
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-heading">Home Assistant</h1>
          <p className="text-sm text-muted mt-1">Expose hookbot as a Home Assistant entity for automations and voice control</p>
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

      {!config ? (
        <div className="rounded-xl border border-border bg-surface p-6">
          <h2 className="text-lg font-semibold text-heading mb-4">Connect to Home Assistant</h2>
          <div className="space-y-3 max-w-md">
            <div>
              <label className="block text-xs font-medium text-muted mb-1">Home Assistant URL</label>
              <input
                className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
                value={formData.ha_url}
                onChange={e => setFormData(f => ({ ...f, ha_url: e.target.value }))}
                placeholder="http://homeassistant.local:8123"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted mb-1">Long-Lived Access Token</label>
              <input
                type="password"
                className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
                value={formData.access_token}
                onChange={e => setFormData(f => ({ ...f, access_token: e.target.value }))}
                placeholder="eyJ0eXAi..."
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted mb-1">Entity ID (optional)</label>
              <input
                className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
                value={formData.entity_id}
                onChange={e => setFormData(f => ({ ...f, entity_id: e.target.value }))}
                placeholder="sensor.hookbot_main"
              />
            </div>
            <button
              onClick={() => createMutation.mutate()}
              className="rounded-lg bg-brand px-6 py-2 text-sm font-medium text-white hover:bg-brand/90"
            >
              Connect
            </button>
          </div>
        </div>
      ) : (
        <>
          {/* Connection Status */}
          <div className="rounded-xl border border-border bg-surface p-5">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-3">
                <span className="text-3xl">🏠</span>
                <div>
                  <h2 className="font-semibold text-heading">Connected</h2>
                  <p className="text-sm text-muted">{config.ha_url}</p>
                </div>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => syncMutation.mutate()}
                  className="rounded-lg bg-brand px-4 py-2 text-sm font-medium text-white hover:bg-brand/90"
                >
                  Sync Now
                </button>
                <button
                  onClick={() => deleteMutation.mutate()}
                  className="rounded-lg border border-red-500/30 px-4 py-2 text-sm text-red-500 hover:bg-red-500/10"
                >
                  Disconnect
                </button>
              </div>
            </div>

            {/* Toggle Settings */}
            <div className="grid grid-cols-2 gap-4 mt-4">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={config.expose_states}
                  onChange={() => updateMutation.mutate({ expose_states: !config.expose_states })}
                  className="rounded"
                />
                <span className="text-sm">Expose device states</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={config.expose_sensors}
                  onChange={() => updateMutation.mutate({ expose_sensors: !config.expose_sensors })}
                  className="rounded"
                />
                <span className="text-sm">Expose sensor data</span>
              </label>
            </div>
          </div>

          {/* Entity Preview */}
          {entity && (
            <div className="rounded-xl border border-border bg-surface p-4">
              <h2 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Entity Preview</h2>
              <div className="font-mono text-sm bg-canvas rounded-lg p-4 overflow-auto">
                <p><span className="text-muted">entity_id:</span> {entity.entity_id}</p>
                <p><span className="text-muted">state:</span> <span className="text-brand">{entity.state}</span></p>
                <p className="text-muted mt-2">attributes:</p>
                {Object.entries(entity.attributes).map(([key, value]) => (
                  <p key={key} className="ml-4">
                    <span className="text-muted">{key}:</span> {String(value)}
                  </p>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
