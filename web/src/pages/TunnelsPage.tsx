import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getTunnels, createTunnel, deleteTunnel, startTunnel, stopTunnel, quickConnectTunnel, getTunnelMetrics, getTunnelLogs } from '../api/client';
import type { TunnelConfig, TunnelLogEntry } from '../api/client';
import { useToast } from '../hooks/useToast';

function formatUptime(secs: number | null | undefined): string {
  if (!secs) return '-';
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
  return `${Math.floor(secs / 86400)}d ${Math.floor((secs % 86400) / 3600)}h`;
}

export default function TunnelsPage() {
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const [showForm, setShowForm] = useState(false);
  const [expandedTunnel, setExpandedTunnel] = useState<string | null>(null);

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

  const quickConnectMut = useMutation({
    mutationFn: () => quickConnectTunnel(),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['tunnels'] });
      if (data.assigned_url) {
        toast(`Tunnel live at ${data.assigned_url}`, 'success');
      } else {
        toast('Quick-connect tunnel started! URL will appear shortly.', 'success');
      }
    },
    onError: (e: Error) => toast(e.message, 'error'),
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
          <p className="text-sm text-muted mt-1">Manage Cloudflare Tunnels for accessing hookbots outside your LAN</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => quickConnectMut.mutate()}
            disabled={quickConnectMut.isPending}
            className="px-4 py-2 bg-green-600 text-white rounded-lg text-sm font-medium hover:bg-green-500 disabled:opacity-50 flex items-center gap-2"
          >
            {quickConnectMut.isPending ? (
              <span className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            ) : (
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" /></svg>
            )}
            Quick Connect
          </button>
          <button onClick={() => setShowForm(s => !s)} className="px-4 py-2 bg-brand text-white rounded-lg text-sm font-medium hover:bg-brand/90">
            {showForm ? 'Cancel' : 'New Tunnel'}
          </button>
        </div>
      </div>

      {/* Quick Connect info banner */}
      {tunnels.length === 0 && !showForm && (
        <div className="bg-green-500/5 border border-green-500/20 rounded-xl p-4 mb-6">
          <div className="flex items-start gap-3">
            <svg className="w-5 h-5 text-green-400 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" /></svg>
            <div>
              <p className="text-sm font-medium text-fg">Quick Connect - No account needed</p>
              <p className="text-xs text-muted mt-1">
                Click "Quick Connect" to instantly get a public URL via TryCloudflare. No Cloudflare account required.
                For custom domains and persistent tunnels, use "New Tunnel" with your Cloudflare tunnel token.
              </p>
            </div>
          </div>
        </div>
      )}

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
                className="mt-1 w-full rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg" placeholder="Cloudflare tunnel token (from dashboard)" />
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
          {tunnels.map((tunnel: TunnelConfig) => (
            <TunnelCard
              key={tunnel.id}
              tunnel={tunnel}
              expanded={expandedTunnel === tunnel.id}
              onToggle={() => setExpandedTunnel(expandedTunnel === tunnel.id ? null : tunnel.id)}
              onStart={() => startMut.mutate(tunnel.id)}
              onStop={() => stopMut.mutate(tunnel.id)}
              onDelete={() => deleteMut.mutate(tunnel.id)}
              statusColor={statusColor}
              statusBadge={statusBadge}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function TunnelCard({
  tunnel,
  expanded,
  onToggle,
  onStart,
  onStop,
  onDelete,
  statusColor,
  statusBadge,
}: {
  tunnel: TunnelConfig;
  expanded: boolean;
  onToggle: () => void;
  onStart: () => void;
  onStop: () => void;
  onDelete: () => void;
  statusColor: (s: string) => string;
  statusBadge: (s: string) => string;
}) {
  const assignedUrl = tunnel.process?.assigned_url || tunnel.hostname;

  const { data: metrics } = useQuery({
    queryKey: ['tunnel-metrics', tunnel.id],
    queryFn: () => getTunnelMetrics(tunnel.id),
    refetchInterval: 5000,
    enabled: expanded && tunnel.status === 'running',
  });

  const { data: logs = [] } = useQuery({
    queryKey: ['tunnel-logs', tunnel.id],
    queryFn: () => getTunnelLogs(tunnel.id, 50),
    refetchInterval: 3000,
    enabled: expanded,
  });

  return (
    <div className="bg-surface border border-edge rounded-xl">
      <div className="p-5">
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className={`w-2 h-2 rounded-full ${statusColor(tunnel.status)} ${tunnel.process?.connected ? 'animate-pulse' : ''}`} />
              <span className="font-medium text-fg">{tunnel.name}</span>
              <span className={`px-2 py-0.5 rounded-full text-[10px] font-medium uppercase ${statusBadge(tunnel.status)}`}>
                {tunnel.status}
              </span>
              <span className="text-xs text-dim px-2 py-0.5 rounded bg-inset">{tunnel.tunnel_type}</span>
              {tunnel.process?.connected && (
                <span className="text-[10px] text-green-400 font-medium">CONNECTED</span>
              )}
            </div>
            <div className="text-xs text-muted mt-1.5 flex flex-wrap gap-x-4">
              {assignedUrl && (
                <span>
                  URL: <a href={assignedUrl} target="_blank" rel="noopener noreferrer"
                    className="text-brand hover:underline">{assignedUrl}</a>
                </span>
              )}
              <span>Port: <span className="text-fg">{tunnel.port}</span></span>
              {tunnel.process?.pid && <span>PID: <span className="text-fg">{tunnel.process.pid}</span></span>}
              {tunnel.last_connected_at && <span>Last connected: {new Date(tunnel.last_connected_at).toLocaleString()}</span>}
            </div>
          </div>
          <div className="flex gap-2 ml-4">
            <button onClick={onToggle}
              className="px-3 py-1.5 rounded text-xs font-medium bg-surface border border-edge hover:bg-inset">
              {expanded ? 'Hide' : 'Details'}
            </button>
            {tunnel.status === 'running' ? (
              <button onClick={onStop}
                className="px-3 py-1.5 rounded text-xs font-medium bg-yellow-500/10 text-yellow-400 hover:bg-yellow-500/20">
                Stop
              </button>
            ) : (
              <button onClick={onStart}
                className="px-3 py-1.5 rounded text-xs font-medium bg-green-500/10 text-green-400 hover:bg-green-500/20">
                Start
              </button>
            )}
            <button onClick={onDelete}
              className="px-3 py-1.5 rounded text-xs font-medium bg-red-500/10 text-red-400 hover:bg-red-500/20">
              Delete
            </button>
          </div>
        </div>
      </div>

      {expanded && (
        <div className="border-t border-edge">
          {/* Metrics panel */}
          {metrics && tunnel.status === 'running' && (
            <div className="px-5 py-3 border-b border-edge">
              <h3 className="text-xs text-subtle uppercase mb-2">Process Metrics</h3>
              <div className="grid grid-cols-4 gap-4">
                <MetricBox label="Uptime" value={formatUptime(metrics.uptime_secs)} />
                <MetricBox label="PID" value={metrics.pid?.toString() || '-'} />
                <MetricBox label="Restarts" value={metrics.restart_count.toString()} />
                <MetricBox label="Connected" value={metrics.connected ? 'Yes' : 'No'}
                  color={metrics.connected ? 'text-green-400' : 'text-red-400'} />
              </div>
              {metrics.assigned_url && (
                <div className="mt-2 flex items-center gap-2">
                  <span className="text-xs text-muted">Public URL:</span>
                  <a href={metrics.assigned_url} target="_blank" rel="noopener noreferrer"
                    className="text-xs text-brand hover:underline font-mono">{metrics.assigned_url}</a>
                  <button
                    onClick={() => navigator.clipboard.writeText(metrics.assigned_url!)}
                    className="text-xs text-muted hover:text-fg"
                    title="Copy URL"
                  >
                    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                  </button>
                </div>
              )}
            </div>
          )}

          {/* Logs panel */}
          <div className="px-5 py-3">
            <h3 className="text-xs text-subtle uppercase mb-2">Process Logs</h3>
            {logs.length === 0 ? (
              <p className="text-xs text-muted">No logs available</p>
            ) : (
              <div className="bg-inset rounded-lg p-3 max-h-60 overflow-y-auto font-mono text-[11px] leading-relaxed space-y-0.5">
                {logs.map((log: TunnelLogEntry, i: number) => (
                  <div key={i} className="flex gap-2">
                    <span className="text-dim shrink-0">{log.timestamp.slice(11, 19) || '??:??:??'}</span>
                    <span className={`shrink-0 w-10 ${
                      log.level === 'error' ? 'text-red-400' :
                      log.level === 'warn' ? 'text-yellow-400' :
                      'text-dim'
                    }`}>{log.level}</span>
                    <span className="text-fg break-all">{log.message}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function MetricBox({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div>
      <div className="text-[10px] text-subtle uppercase">{label}</div>
      <div className={`text-sm font-medium ${color || 'text-fg'}`}>{value}</div>
    </div>
  );
}
