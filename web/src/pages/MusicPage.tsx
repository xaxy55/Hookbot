import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getMusicConfig,
  createMusicConfig,
  updateMusicConfig,
  deleteMusicConfig,
  getNowPlaying,
  musicAction,
  type MusicConfig,
} from '../api/client';
import { useToast } from '../hooks/useToast';

export default function MusicPage() {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const [selectedDevice, setSelectedDevice] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [provider, setProvider] = useState('spotify');

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: configs } = useQuery({
    queryKey: ['musicConfig', deviceId],
    queryFn: () => getMusicConfig(deviceId),
    enabled: !!deviceId,
  });

  const { data: nowPlaying } = useQuery({
    queryKey: ['nowPlaying', deviceId],
    queryFn: () => getNowPlaying(deviceId),
    enabled: !!deviceId,
    refetchInterval: 5000,
  });

  const createMutation = useMutation({
    mutationFn: () => createMusicConfig({ device_id: deviceId, provider }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['musicConfig'] });
      setShowForm(false);
      toast('Music integration added', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const toggleMutation = useMutation({
    mutationFn: (cfg: MusicConfig) => updateMusicConfig(cfg.id, { enabled: !cfg.enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['musicConfig'] });
      toast('Toggled', 'success');
    },
  });

  const togglePauseMutation = useMutation({
    mutationFn: (cfg: MusicConfig) => updateMusicConfig(cfg.id, { auto_pause_meetings: !cfg.auto_pause_meetings }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['musicConfig'] });
      toast('Auto-pause setting updated', 'success');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => deleteMusicConfig(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['musicConfig'] });
      toast('Integration removed', 'success');
    },
  });

  const actionMutation = useMutation({
    mutationFn: (action: string) => musicAction(action, deviceId),
    onSuccess: () => toast('Action sent', 'success'),
    onError: (e: Error) => toast(e.message, 'error'),
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-heading">Music Integration</h1>
          <p className="text-sm text-muted mt-1">Spotify and Apple Music control synced to your coding flow</p>
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
            + Connect
          </button>
        </div>
      </div>

      {/* Now Playing */}
      <div className="rounded-xl border border-border bg-surface p-5">
        <h2 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Now Playing</h2>
        {nowPlaying?.is_playing ? (
          <div className="flex items-center gap-4">
            {nowPlaying.album_art_url && (
              <img src={nowPlaying.album_art_url} alt="" className="w-16 h-16 rounded-lg" />
            )}
            <div className="flex-1">
              <p className="font-semibold text-heading">{nowPlaying.track_name}</p>
              <p className="text-sm text-muted">{nowPlaying.artist_name} &middot; {nowPlaying.album_name}</p>
            </div>
          </div>
        ) : (
          <p className="text-muted text-sm">Nothing playing right now</p>
        )}
        <div className="flex gap-2 mt-3">
          {['previous', 'play', 'pause', 'next'].map(action => (
            <button
              key={action}
              onClick={() => actionMutation.mutate(action)}
              className="rounded-lg border border-border px-3 py-1.5 text-sm hover:border-brand/50 capitalize"
            >
              {action === 'previous' ? '⏮' : action === 'next' ? '⏭' : action === 'play' ? '▶' : '⏸'} {action}
            </button>
          ))}
        </div>
      </div>

      {showForm && (
        <div className="rounded-xl border border-border bg-surface p-4 space-y-3">
          <label className="block text-xs font-medium text-muted mb-1">Provider</label>
          <select
            className="w-full rounded-lg border border-border bg-canvas px-3 py-2 text-sm"
            value={provider}
            onChange={e => setProvider(e.target.value)}
          >
            <option value="spotify">Spotify</option>
            <option value="apple_music">Apple Music</option>
          </select>
          <button
            onClick={() => createMutation.mutate()}
            className="rounded-lg bg-brand px-4 py-2 text-sm font-medium text-white"
          >
            Connect
          </button>
        </div>
      )}

      {/* Integrations */}
      <div className="space-y-3">
        {configs?.map(cfg => (
          <div key={cfg.id} className="rounded-xl border border-border bg-surface p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span className="text-2xl">{cfg.provider === 'spotify' ? '🎵' : '🎶'}</span>
                <div>
                  <h3 className="font-semibold text-heading capitalize">{cfg.provider.replace('_', ' ')}</h3>
                  <p className="text-xs text-muted">
                    Auto-pause on meetings: {cfg.auto_pause_meetings ? 'Yes' : 'No'}
                  </p>
                </div>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => togglePauseMutation.mutate(cfg)}
                  className="rounded-lg border border-border px-3 py-1.5 text-xs hover:border-brand/50"
                >
                  Toggle Auto-Pause
                </button>
                <button
                  onClick={() => toggleMutation.mutate(cfg)}
                  className={`rounded-lg px-3 py-1.5 text-xs font-medium ${cfg.enabled ? 'bg-green-500/10 text-green-500' : 'bg-red-500/10 text-red-500'}`}
                >
                  {cfg.enabled ? 'Enabled' : 'Disabled'}
                </button>
                <button
                  onClick={() => deleteMutation.mutate(cfg.id)}
                  className="rounded-lg px-3 py-1.5 text-xs bg-red-500/10 text-red-500 hover:bg-red-500/20"
                >
                  Remove
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
