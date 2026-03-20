import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getStreamDeckButtons,
  createStreamDeckButton,
  updateStreamDeckButton,
  deleteStreamDeckButton,
  triggerStreamDeckButton,
  type StreamDeckButton,
} from '../api/client';
import { useToast } from '../hooks/useToast';

const ACTION_TYPES = [
  { value: 'state_change', label: 'Change State' },
  { value: 'animation', label: 'Play Animation' },
  { value: 'ota_deploy', label: 'Deploy OTA' },
  { value: 'webhook', label: 'Call Webhook' },
  { value: 'servo', label: 'Move Servo' },
  { value: 'sound', label: 'Play Sound' },
];

export default function StreamDeckPage() {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const [selectedDevice, setSelectedDevice] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [formData, setFormData] = useState({ position: 0, label: '', action_type: 'state_change' });

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: buttons } = useQuery({
    queryKey: ['streamdeckButtons', deviceId],
    queryFn: () => getStreamDeckButtons(deviceId),
    enabled: !!deviceId,
  });

  const createMutation = useMutation({
    mutationFn: () => createStreamDeckButton({ device_id: deviceId, ...formData }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['streamdeckButtons'] });
      setShowForm(false);
      toast('Button created', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const triggerMutation = useMutation({
    mutationFn: (id: string) => triggerStreamDeckButton(id),
    onSuccess: () => toast('Button triggered', 'success'),
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const toggleMutation = useMutation({
    mutationFn: (btn: StreamDeckButton) => updateStreamDeckButton(btn.id, { enabled: !btn.enabled }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['streamdeckButtons'] }),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteStreamDeckButton(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['streamdeckButtons'] });
      toast('Button deleted', 'success');
    },
  });

  // Create a 3x5 grid layout for the Stream Deck
  const gridSlots = Array.from({ length: 15 }, (_, i) => {
    const btn = buttons?.find(b => b.position === i);
    return { position: i, button: btn };
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-heading">Stream Deck</h1>
          <p className="text-sm text-muted mt-1">Custom buttons for hookbot actions</p>
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
        </div>
      </div>

      {/* Stream Deck Grid */}
      <div className="rounded-xl border border-border bg-surface/50 p-6">
        <div className="grid grid-cols-5 gap-3 max-w-2xl mx-auto">
          {gridSlots.map(slot => (
            <button
              key={slot.position}
              onClick={() => {
                if (slot.button) {
                  triggerMutation.mutate(slot.button.id);
                } else {
                  setFormData(f => ({ ...f, position: slot.position }));
                  setShowForm(true);
                }
              }}
              className={`aspect-square rounded-xl border-2 flex flex-col items-center justify-center text-center p-2 transition-all ${
                slot.button
                  ? slot.button.enabled
                    ? 'border-brand bg-brand/10 hover:bg-brand/20 cursor-pointer'
                    : 'border-border bg-surface/50 opacity-50'
                  : 'border-dashed border-border hover:border-brand/30 hover:bg-surface'
              }`}
            >
              {slot.button ? (
                <>
                  <span className="text-lg">{slot.button.icon || '⚡'}</span>
                  <span className="text-xs font-medium text-heading mt-1 truncate w-full">{slot.button.label}</span>
                  <span className="text-[10px] text-muted">{slot.button.action_type.replace('_', ' ')}</span>
                </>
              ) : (
                <span className="text-xl text-muted/30">+</span>
              )}
            </button>
          ))}
        </div>
      </div>

      {showForm && (
        <div className="rounded-xl border border-border bg-surface p-4 space-y-3">
          <h3 className="font-semibold text-heading">New Button (Position {formData.position})</h3>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-medium text-muted mb-1">Label</label>
              <input
                className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
                value={formData.label}
                onChange={e => setFormData(f => ({ ...f, label: e.target.value }))}
                placeholder="Deploy"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted mb-1">Action Type</label>
              <select
                className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
                value={formData.action_type}
                onChange={e => setFormData(f => ({ ...f, action_type: e.target.value }))}
              >
                {ACTION_TYPES.map(t => <option key={t.value} value={t.value}>{t.label}</option>)}
              </select>
            </div>
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => createMutation.mutate()}
              className="rounded-lg bg-brand px-4 py-2 text-sm font-medium text-white"
            >
              Create
            </button>
            <button onClick={() => setShowForm(false)} className="rounded-lg border border-border px-4 py-2 text-sm">
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Button List */}
      {buttons && buttons.length > 0 && (
        <div className="rounded-xl border border-border bg-surface p-4">
          <h2 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">All Buttons</h2>
          <div className="space-y-2">
            {buttons.map(btn => (
              <div key={btn.id} className="flex items-center justify-between py-2 border-b border-border/50 last:border-0">
                <div className="flex items-center gap-2">
                  <span className="w-6 h-6 rounded bg-brand/10 flex items-center justify-center text-xs font-mono">{btn.position}</span>
                  <span className="text-sm font-medium">{btn.icon || '⚡'} {btn.label}</span>
                  <span className="text-xs text-muted">({btn.action_type})</span>
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={() => toggleMutation.mutate(btn)}
                    className={`rounded px-2 py-1 text-xs ${btn.enabled ? 'text-green-500' : 'text-red-500'}`}
                  >
                    {btn.enabled ? 'On' : 'Off'}
                  </button>
                  <button
                    onClick={() => deleteMutation.mutate(btn.id)}
                    className="rounded px-2 py-1 text-xs text-red-500 hover:bg-red-500/10"
                  >
                    Del
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
