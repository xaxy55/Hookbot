import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getDeviceLinks, createDeviceLink, updateDeviceLink, deleteDeviceLink, getDevices } from '../api/client';
import { useToast } from '../hooks/useToast';

const TRIGGER_TYPES = ['state_change', 'error', 'success', 'idle_timeout'];
const ACTION_TYPES = ['set_state', 'relay_state', 'play_animation', 'send_notification'];

export default function DeviceLinksPage() {
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const [showForm, setShowForm] = useState(false);

  const { data: links = [], isLoading } = useQuery({ queryKey: ['device-links'], queryFn: () => getDeviceLinks() });
  const { data: devices = [] } = useQuery({ queryKey: ['devices'], queryFn: getDevices });

  const [form, setForm] = useState({
    source_device_id: '',
    target_device_id: '',
    trigger_type: 'state_change',
    action_type: 'relay_state',
    cooldown_secs: 30,
    action_state: 'error',
    action_message: 'Device link triggered',
  });

  const createMut = useMutation({
    mutationFn: async () => {
      const actionConfig: Record<string, unknown> = {};
      if (form.action_type === 'set_state') actionConfig.state = form.action_state;
      if (form.action_type === 'send_notification') actionConfig.message = form.action_message;
      return createDeviceLink({
        source_device_id: form.source_device_id,
        target_device_id: form.target_device_id,
        trigger_type: form.trigger_type,
        action_type: form.action_type,
        action_config: actionConfig,
        cooldown_secs: form.cooldown_secs,
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['device-links'] });
      setShowForm(false);
      toast('Device link created', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const toggleMut = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) => updateDeviceLink(id, { enabled }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['device-links'] }),
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => deleteDeviceLink(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['device-links'] });
      toast('Link deleted', 'success');
    },
  });

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-fg">Device Links</h1>
          <p className="text-sm text-muted mt-1">Connect devices so one bot's state change triggers another</p>
        </div>
        <button onClick={() => setShowForm(s => !s)} className="px-4 py-2 bg-brand text-white rounded-lg text-sm font-medium hover:bg-brand/90">
          {showForm ? 'Cancel' : 'New Link'}
        </button>
      </div>

      {showForm && (
        <div className="bg-surface border border-edge rounded-xl p-5 mb-6 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <label className="block">
              <span className="text-xs text-subtle uppercase">Source Device</span>
              <select value={form.source_device_id} onChange={e => setForm(f => ({ ...f, source_device_id: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
                <option value="">Select...</option>
                {devices.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
              </select>
            </label>
            <label className="block">
              <span className="text-xs text-subtle uppercase">Target Device</span>
              <select value={form.target_device_id} onChange={e => setForm(f => ({ ...f, target_device_id: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
                <option value="">Select...</option>
                {devices.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
              </select>
            </label>
            <label className="block">
              <span className="text-xs text-subtle uppercase">Trigger</span>
              <select value={form.trigger_type} onChange={e => setForm(f => ({ ...f, trigger_type: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
                {TRIGGER_TYPES.map(t => <option key={t} value={t}>{t.replace('_', ' ')}</option>)}
              </select>
            </label>
            <label className="block">
              <span className="text-xs text-subtle uppercase">Action</span>
              <select value={form.action_type} onChange={e => setForm(f => ({ ...f, action_type: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
                {ACTION_TYPES.map(t => <option key={t} value={t}>{t.replace('_', ' ')}</option>)}
              </select>
            </label>
            {form.action_type === 'set_state' && (
              <label className="block">
                <span className="text-xs text-subtle uppercase">Target State</span>
                <input value={form.action_state} onChange={e => setForm(f => ({ ...f, action_state: e.target.value }))}
                  className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" />
              </label>
            )}
            <label className="block">
              <span className="text-xs text-subtle uppercase">Cooldown (seconds)</span>
              <input type="number" value={form.cooldown_secs} onChange={e => setForm(f => ({ ...f, cooldown_secs: Number(e.target.value) }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" />
            </label>
          </div>
          <button onClick={() => createMut.mutate()} disabled={!form.source_device_id || !form.target_device_id}
            className="px-4 py-2 bg-brand text-white rounded-lg text-sm font-medium hover:bg-brand/90 disabled:opacity-50">
            Create Link
          </button>
        </div>
      )}

      {isLoading ? (
        <p className="text-muted text-sm">Loading...</p>
      ) : links.length === 0 ? (
        <div className="text-center py-12 text-muted">
          <p className="text-lg">No device links yet</p>
          <p className="text-sm mt-1">Create a link to connect two devices</p>
        </div>
      ) : (
        <div className="space-y-3">
          {links.map(link => (
            <div key={link.id} className="bg-surface border border-edge rounded-xl p-4 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className={`w-2 h-2 rounded-full ${link.enabled ? 'bg-green-500' : 'bg-gray-500'}`} />
                <div>
                  <div className="text-sm font-medium text-fg">
                    {link.source_device_name || link.source_device_id.slice(0, 8)}
                    <span className="text-muted mx-2">→</span>
                    {link.target_device_name || link.target_device_id.slice(0, 8)}
                  </div>
                  <div className="text-xs text-muted mt-0.5">
                    When <span className="text-brand-fg">{link.trigger_type.replace('_', ' ')}</span> then{' '}
                    <span className="text-brand-fg">{link.action_type.replace('_', ' ')}</span>
                    {' '} (cooldown: {link.cooldown_secs}s)
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <button onClick={() => toggleMut.mutate({ id: link.id, enabled: !link.enabled })}
                  className={`px-3 py-1 rounded text-xs font-medium ${link.enabled ? 'bg-green-500/10 text-green-400' : 'bg-gray-500/10 text-gray-400'}`}>
                  {link.enabled ? 'Enabled' : 'Disabled'}
                </button>
                <button onClick={() => deleteMut.mutate(link.id)}
                  className="px-3 py-1 rounded text-xs font-medium bg-red-500/10 text-red-400 hover:bg-red-500/20">
                  Delete
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
