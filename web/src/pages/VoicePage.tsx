import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getVoiceStatus, sendVoiceCommand, sendTts, sendAnnouncement, getDevices } from '../api/client';
import { useToast } from '../hooks/useToast';
import { useState } from 'react';

export default function VoicePage() {
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const { data: devices = [] } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [deviceId, setDeviceId] = useState<string>('');
  const [transcript, setTranscript] = useState('');
  const [ttsText, setTtsText] = useState('');
  const [ttsVoice, setTtsVoice] = useState('default');

  const effectiveDeviceId = deviceId || devices[0]?.id;

  const { data: status } = useQuery({
    queryKey: ['voice-status', effectiveDeviceId],
    queryFn: () => getVoiceStatus(effectiveDeviceId),
    enabled: !!effectiveDeviceId,
    refetchInterval: 10000,
  });

  const commandMut = useMutation({
    mutationFn: (transcript: string) =>
      sendVoiceCommand({ transcript, device_id: effectiveDeviceId }),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['voice-status'] });
      toast(`${data.action}: ${data.detail}${data.executed ? ' (executed)' : ''}`, data.executed ? 'success' : 'info');
      setTranscript('');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const ttsMut = useMutation({
    mutationFn: (text: string) =>
      sendTts({ text, device_id: effectiveDeviceId, voice: ttsVoice }),
    onSuccess: (data) => {
      toast(data.sent_to_device ? 'Sent to device' : 'Device unreachable', data.sent_to_device ? 'success' : 'error');
      setTtsText('');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const announceMut = useMutation({
    mutationFn: (type: string) =>
      sendAnnouncement({ device_id: effectiveDeviceId, announcement_type: type }),
    onSuccess: (data) => {
      toast(data.sent_to_device ? `Announced: "${data.text}"` : 'Device unreachable', data.sent_to_device ? 'success' : 'error');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-fg">Voice Control</h1>
          <p className="text-sm text-muted mt-1">Voice commands and text-to-speech for your hookbot</p>
        </div>
        {devices.length > 1 && (
          <select value={deviceId} onChange={e => setDeviceId(e.target.value)}
            className="rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
            <option value="">Default device</option>
            {devices.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
        )}
      </div>

      {/* Voice Status */}
      {status && (
        <div className="bg-surface border border-edge rounded-xl p-5 mb-6">
          <h2 className="text-sm font-medium text-subtle uppercase mb-3">Voice Status</h2>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
            <div>
              <div className="text-xs text-muted">Status</div>
              <div className={`text-sm font-medium ${status.enabled ? 'text-green-400' : 'text-red-400'}`}>
                {status.enabled ? 'Enabled' : 'Disabled'}
              </div>
            </div>
            <div>
              <div className="text-xs text-muted">Wake Word</div>
              <div className="text-sm font-medium text-fg">"{status.wake_word}"</div>
            </div>
            <div>
              <div className="text-xs text-muted">Total Commands</div>
              <div className="text-sm font-medium text-fg">{status.total_commands}</div>
            </div>
            <div>
              <div className="text-xs text-muted">Last Command</div>
              <div className="text-sm font-medium text-fg">{status.last_command || 'None'}</div>
              {status.last_command_at && (
                <div className="text-[10px] text-dim">{new Date(status.last_command_at).toLocaleString()}</div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Send Voice Command */}
      <div className="bg-surface border border-edge rounded-xl p-5 mb-6">
        <h2 className="text-sm font-medium text-subtle uppercase mb-3">Voice Command</h2>
        <p className="text-xs text-muted mb-3">Simulate a voice command by typing a transcript</p>
        <div className="flex gap-2">
          <input
            type="text"
            value={transcript}
            onChange={e => setTranscript(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && transcript.trim() && commandMut.mutate(transcript.trim())}
            placeholder='e.g. "set idle", "check status", "thinking"'
            className="flex-1 rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg placeholder:text-dim"
          />
          <button
            onClick={() => transcript.trim() && commandMut.mutate(transcript.trim())}
            disabled={commandMut.isPending || !transcript.trim()}
            className="px-4 py-2 rounded-lg bg-brand text-white text-sm font-medium hover:bg-brand/80 disabled:opacity-50"
          >
            {commandMut.isPending ? 'Sending...' : 'Send'}
          </button>
        </div>
        <div className="mt-3 flex flex-wrap gap-2">
          {['idle', 'thinking', 'success', 'error', 'waiting', 'check status', 'what level'].map(cmd => (
            <button key={cmd} onClick={() => commandMut.mutate(cmd)}
              className="px-3 py-1.5 rounded-lg bg-inset border border-edge text-xs text-muted hover:text-fg hover:border-brand/30 transition-colors">
              {cmd}
            </button>
          ))}
        </div>
      </div>

      {/* Text-to-Speech */}
      <div className="bg-surface border border-edge rounded-xl p-5 mb-6">
        <h2 className="text-sm font-medium text-subtle uppercase mb-3">Text-to-Speech</h2>
        <p className="text-xs text-muted mb-3">Send text to the device for TTS playback via I2S DAC speaker</p>
        <div className="flex gap-2 mb-3">
          <input
            type="text"
            value={ttsText}
            onChange={e => setTtsText(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && ttsText.trim() && ttsMut.mutate(ttsText.trim())}
            placeholder="Type something to speak..."
            className="flex-1 rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg placeholder:text-dim"
          />
          <select value={ttsVoice} onChange={e => setTtsVoice(e.target.value)}
            className="rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
            <option value="default">Default</option>
            <option value="deep">Deep</option>
            <option value="high">High</option>
          </select>
          <button
            onClick={() => ttsText.trim() && ttsMut.mutate(ttsText.trim())}
            disabled={ttsMut.isPending || !ttsText.trim()}
            className="px-4 py-2 rounded-lg bg-brand text-white text-sm font-medium hover:bg-brand/80 disabled:opacity-50"
          >
            {ttsMut.isPending ? 'Speaking...' : 'Speak'}
          </button>
        </div>
      </div>

      {/* Smart Announcements */}
      <div className="bg-surface border border-edge rounded-xl p-5">
        <h2 className="text-sm font-medium text-subtle uppercase mb-3">Smart Announcements</h2>
        <p className="text-xs text-muted mb-3">Generate context-aware announcements using recent activity data</p>
        <div className="grid grid-cols-3 gap-3">
          {[
            { type: 'greeting', label: 'Greeting', desc: 'Time-based greeting' },
            { type: 'status', label: 'Status', desc: 'Current activity summary' },
            { type: 'summary', label: 'Summary', desc: 'Session summary' },
          ].map(item => (
            <button key={item.type}
              onClick={() => announceMut.mutate(item.type)}
              disabled={announceMut.isPending}
              className="p-4 rounded-lg bg-inset border border-edge hover:border-brand/30 transition-colors text-left">
              <div className="text-sm font-medium text-fg">{item.label}</div>
              <div className="text-xs text-muted mt-1">{item.desc}</div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
