import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getTunnels, createTunnel, updateTunnel, deleteTunnel, startTunnel, stopTunnel } from '../api/client';
import { useToast } from '../hooks/useToast';

export default function TunnelsPage() {
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const [showForm, setShowForm] = useState(false);

  const { data: tunnels = [], isLoading } = useQuery({
    queryKey: ['tunnels'],
    queryFn: getTunnels,
    refetchInterval: 5000,
  });

  const [form, setForm] = useState({
    name: '',
    tunnel_type: 'cloudflare',
    hostname: '',
    port: 3000,
    auth_token: '',
  });

  const createMut = useMutation({
    mutationFn: () => createTunnel({
      name: form.name,
      tunnel_type: form.tunnel_type,
      hostname: form.hostname || undefined,
      port: form.port,
      auth_token: form.auth_token || undefined,
    }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tunnels'] });
      setShowForm(false);
      setForm({ name: '', tunnel_type: 'cloudflare', hostname: '', port: 3000, auth_token: '' });
      toast('Tunnel created', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const startMut = useMutation({
    mutationFn: (id: string) => startTunnel(id),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['tunnels'] });
      toast(data.message || 'Tunnel started', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const stopMut = useMutation({
    mutationFn: (id: string) => stopTunnel(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tunnels'] });
      toast('Tunnel stopped', 'success');
    },
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => deleteTunnel(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tunnels'] });
      toast('Tunnel deleted', 'success');
    },
  });

  const statusColor = (status: string) => {
    switch (status) {
      case 'running': return 'bg-green-500';
      case 'error': return 'bg-red-500';
      default: return 'bg-gray-500';
    }
  };

  const statusBadge = (status: string) => {
    switch (status) {
      case 'running': return 'bg-green-500/10 text-green-400';
      case 'error': return 'bg-red-500/10 text-red-400';
      default: return 'bg-gray-500/10 text-gray-400';
    }
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-fg">Remote Access</h1>
          <p className="text-sm text-muted mt-1">Manage tunnels for accessing hookbots outside your LAN</p>
        </div>
        <button onClick={() => setShowForm(s => !s)} className="px-4 py-2 bg-brand text-white rounded-lg text-sm font-medium hover:bg-brand/90">
          {showForm ? 'Cancel' : 'New Tunnel'}
        </button>
      </div>

      {showForm && (
        <div className="bg-surface border border-edge rounded-xl p-5 mb-6 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <label className="block">
              <span className="text-xs text-subtle uppercase">Name</span>
              <input value={form.name} onChange={e => setForm(f => ({ ...f, name: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" placeholder="my-tunnel" />
            </label>
            <label className="block">
              <span className="text-xs text-subtle uppercase">Type</span>
              <select value={form.tunnel_type} onChange={e => setForm(f => ({ ...f, tunnel_type: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
                <option value="cloudflare">Cloudflare Tunnel</option>
                <option value="ngrok">ngrok</option>
                <option value="custom">Custom</option>
              </select>
            </label>
            <label className="block">
              <span className="text-xs text-subtle uppercase">Hostname</span>
              <input value={form.hostname} onChange={e => setForm(f => ({ ...f, hostname: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" placeholder="hookbot.example.com" />
            </label>
            <label className="block">
              <span className="text-xs text-subtle uppercase">Local Port</span>
              <input type="number" value={form.port} onChange={e => setForm(f => ({ ...f, port: Number(e.target.value) }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" />
            </label>
            <label className="block col-span-2">
              <span className="text-xs text-subtle uppercase">Auth Token</span>
              <input type="password" value={form.auth_token} onChange={e => setForm(f => ({ ...f, auth_token: e.target.value }))}
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" placeholder="Tunnel provider auth token" />
            </label>
          </div>
          <button onClick={() => createMut.mutate()} disabled={!form.name}
            className="px-4 py-2 bg-brand text-white rounded-lg text-sm font-medium hover:bg-brand/90 disabled:opacity-50">
            Create Tunnel
          </button>
        </div>
      )}

      {isLoading ? (
        <p className="text-muted text-sm">Loading...</p>
      ) : tunnels.length === 0 ? (
        <div className="text-center py-12 text-muted">
          <p className="text-lg">No tunnels configured</p>
          <p className="text-sm mt-1">Set up a tunnel to access your hookbots remotely</p>
        </div>
      ) : (
        <div className="space-y-3">
          {tunnels.map(tunnel => (
            <div key={tunnel.id} className="bg-surface border border-edge rounded-xl p-5">
              <div className="flex items-start justify-between">
                <div>
                  <div className="flex items-center gap-2">
                    <span className={`w-2 h-2 rounded-full ${statusColor(tunnel.status)}`} />
                    <span className="font-medium text-fg">{tunnel.name}</span>
                    <span className={`px-2 py-0.5 rounded-full text-[10px] font-medium uppercase ${statusBadge(tunnel.status)}`}>
                      {tunnel.status}
                    </span>
                    <span className="text-xs text-dim px-2 py-0.5 rounded bg-inset">{tunnel.tunnel_type}</span>
                  </div>
                  <div className="text-xs text-muted mt-1.5 space-x-4">
                    {tunnel.hostname && <span>Host: <span className="text-fg">{tunnel.hostname}</span></span>}
                    <span>Port: <span className="text-fg">{tunnel.port}</span></span>
                    {tunnel.last_connected_at && <span>Last connected: {new Date(tunnel.last_connected_at).toLocaleString()}</span>}
                  </div>
                </div>
                <div className="flex gap-2">
                  {tunnel.status === 'running' ? (
                    <button onClick={() => stopMut.mutate(tunnel.id)}
                      className="px-3 py-1.5 rounded text-xs font-medium bg-yellow-500/10 text-yellow-400 hover:bg-yellow-500/20">
                      Stop
                    </button>
                  ) : (
                    <button onClick={() => startMut.mutate(tunnel.id)}
                      className="px-3 py-1.5 rounded text-xs font-medium bg-green-500/10 text-green-400 hover:bg-green-500/20">
                      Start
                    </button>
                  )}
                  <button onClick={() => deleteMut.mutate(tunnel.id)}
                    className="px-3 py-1.5 rounded text-xs font-medium bg-red-500/10 text-red-400 hover:bg-red-500/20">
                    Delete
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
