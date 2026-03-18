import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getDevices, getHealth, getServerSettings, updateServerSettings, getLogStats, pruneLogs } from '../api/client';

type HookMode = 'direct' | 'server';

interface HookConfig {
  host: string;
  mode: HookMode;
  deviceId?: string;
}

export default function Settings() {
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const { data: health } = useQuery({ queryKey: ['health'], queryFn: getHealth });

  const [hookMode, setHookMode] = useState<HookMode>('server');
  const [hookHost, setHookHost] = useState('http://localhost:3000');
  const [hookDeviceId, setHookDeviceId] = useState('');
  const [directHost, setDirectHost] = useState('http://hookbot.local');
  const [copied, setCopied] = useState('');

  const queryClient = useQueryClient();
  const { data: serverSettings } = useQuery({ queryKey: ['serverSettings'], queryFn: getServerSettings });
  const { data: logStats } = useQuery({ queryKey: ['logStats'], queryFn: getLogStats, refetchInterval: 10000 });
  const [retentionInput, setRetentionInput] = useState<string>('');
  const retentionValue = retentionInput || String(serverSettings?.log_retention_hours ?? '');

  const updateRetention = useMutation({
    mutationFn: (hours: number) => updateServerSettings({ log_retention_hours: hours }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['serverSettings'] });
      queryClient.invalidateQueries({ queryKey: ['logStats'] });
    },
  });

  const pruneLogsMutation = useMutation({
    mutationFn: pruneLogs,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['logStats'] });
    },
  });

  function getHookConfig(): HookConfig {
    if (hookMode === 'server') {
      return {
        host: hookHost,
        mode: 'server',
        ...(hookDeviceId ? { deviceId: hookDeviceId } : {}),
      };
    }
    return { host: directHost, mode: 'direct' };
  }

  function getConfigJson() {
    const config = getHookConfig();
    if (config.mode === 'direct') {
      return JSON.stringify({ host: config.host }, null, 2);
    }
    const obj: Record<string, string> = { host: config.host, mode: 'server' };
    if (config.deviceId) obj.device_id = config.deviceId;
    return JSON.stringify(obj, null, 2);
  }

  function getProjectConfig() {
    const config = getHookConfig();
    if (config.mode === 'direct') {
      return JSON.stringify({ host: config.host }, null, 2);
    }
    const obj: Record<string, string> = { host: config.host, mode: 'server' };
    if (config.deviceId) obj.device_id = config.deviceId;
    return JSON.stringify(obj, null, 2);
  }

  async function copyToClipboard(text: string, label: string) {
    await navigator.clipboard.writeText(text);
    setCopied(label);
    setTimeout(() => setCopied(''), 2000);
  }

  const serverOnline = health?.status === 'ok';

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-bold text-fg">Settings</h1>

      {/* Hook Configuration */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-5">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-fg-2">Hook Configuration</h2>
          <div className={`flex items-center gap-1.5 text-xs ${serverOnline ? 'text-green-400' : 'text-red-400'}`}>
            <span className={`w-1.5 h-1.5 rounded-full ${serverOnline ? 'bg-green-400' : 'bg-red-400'}`} />
            Server {serverOnline ? 'online' : 'offline'}
          </div>
        </div>

        {/* Mode selector */}
        <div>
          <label className="block text-xs text-subtle mb-2">Routing Mode</label>
          <div className="grid grid-cols-2 gap-2">
            <button
              onClick={() => setHookMode('server')}
              className={`p-3 rounded-lg border text-left transition-all ${
                hookMode === 'server'
                  ? 'border-red-500/30 bg-red-500/10'
                  : 'border-edge bg-inset/50 hover:border-muted'
              }`}
            >
              <div className="flex items-center gap-2 mb-1">
                <span className={`w-3 h-3 rounded-full border-2 ${hookMode === 'server' ? 'border-red-500 bg-red-500' : 'border-dim'}`} />
                <span className="text-sm font-medium text-fg">Server (recommended)</span>
              </div>
              <p className="text-[11px] text-subtle ml-5">
                Routes through management server. Supports multi-device, per-project routing, and logging.
              </p>
            </button>
            <button
              onClick={() => setHookMode('direct')}
              className={`p-3 rounded-lg border text-left transition-all ${
                hookMode === 'direct'
                  ? 'border-red-500/30 bg-red-500/10'
                  : 'border-edge bg-inset/50 hover:border-muted'
              }`}
            >
              <div className="flex items-center gap-2 mb-1">
                <span className={`w-3 h-3 rounded-full border-2 ${hookMode === 'direct' ? 'border-red-500 bg-red-500' : 'border-dim'}`} />
                <span className="text-sm font-medium text-fg">Direct</span>
              </div>
              <p className="text-[11px] text-subtle ml-5">
                Sends state changes directly to a single ESP32. Simple, no server needed.
              </p>
            </button>
          </div>
        </div>

        {/* Mode-specific settings */}
        {hookMode === 'server' ? (
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-subtle mb-1">Server URL</label>
              <input
                value={hookHost}
                onChange={e => setHookHost(e.target.value)}
                className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg font-mono"
              />
            </div>
            <div>
              <label className="block text-xs text-subtle mb-1">Target Device (optional)</label>
              <select
                value={hookDeviceId}
                onChange={e => setHookDeviceId(e.target.value)}
                className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg"
              >
                <option value="">Auto (first registered device)</option>
                {devices?.map(d => <option key={d.id} value={d.id}>{d.name} ({d.ip_address})</option>)}
              </select>
              <p className="text-[10px] text-dim mt-1">
                Leave empty to use the default device. Set per-project using a .hookbot file.
              </p>
            </div>
          </div>
        ) : (
          <div>
            <label className="block text-xs text-subtle mb-1">Device URL</label>
            <input
              value={directHost}
              onChange={e => setDirectHost(e.target.value)}
              className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg font-mono"
            />
          </div>
        )}

        {/* Generated config */}
        <div className="space-y-3">
          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs text-subtle font-medium">hooks/hookbot-config.json</label>
              <button
                onClick={() => copyToClipboard(getConfigJson(), 'config')}
                className="text-[10px] text-subtle hover:text-fg transition-colors"
              >
                {copied === 'config' ? 'Copied!' : 'Copy'}
              </button>
            </div>
            <pre className="text-[11px] bg-inset rounded-md p-3 text-green-400 font-mono border border-edge">
              {getConfigJson()}
            </pre>
          </div>

          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs text-subtle font-medium">Per-project: .hookbot (optional)</label>
              <button
                onClick={() => copyToClipboard(getProjectConfig(), 'project')}
                className="text-[10px] text-subtle hover:text-fg transition-colors"
              >
                {copied === 'project' ? 'Copied!' : 'Copy'}
              </button>
            </div>
            <pre className="text-[11px] bg-inset rounded-md p-3 text-blue-400 font-mono border border-edge">
              {getProjectConfig()}
            </pre>
            <p className="text-[10px] text-dim mt-1">
              Place in project root to route this workspace to a specific device.
            </p>
          </div>
        </div>

        {/* Install command */}
        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="text-xs text-subtle font-medium">Install hooks</label>
            <button
              onClick={() => copyToClipboard('bash hooks/install.sh', 'install')}
              className="text-[10px] text-subtle hover:text-fg transition-colors"
            >
              {copied === 'install' ? 'Copied!' : 'Copy'}
            </button>
          </div>
          <pre className="text-[11px] bg-inset rounded-md p-3 text-yellow-400 font-mono border border-edge">
            bash hooks/install.sh
          </pre>
        </div>
      </div>

      {/* Status Log Retention */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">Status Log Retention</h2>
        <p className="text-xs text-subtle">
          Automatically prune status log entries older than the retention period. Changes take effect on the next poll cycle.
        </p>

        <div className="flex items-end gap-3">
          <div className="flex-1">
            <label className="block text-xs text-subtle mb-1">Retention Period (hours)</label>
            <input
              type="number"
              min={1}
              value={retentionValue}
              onChange={e => setRetentionInput(e.target.value)}
              className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg font-mono"
            />
          </div>
          <button
            onClick={() => {
              const hours = parseInt(retentionValue);
              if (hours >= 1) {
                updateRetention.mutate(hours);
                setRetentionInput('');
              }
            }}
            disabled={updateRetention.isPending || !retentionValue || parseInt(retentionValue) < 1}
            className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-md transition-colors"
          >
            {updateRetention.isPending ? 'Saving...' : 'Save'}
          </button>
        </div>

        {logStats && (
          <div className="rounded-md border border-edge bg-inset/50 p-3 space-y-2">
            <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
              <span className="text-subtle">Total entries</span>
              <span className="text-fg font-mono">{logStats.total_entries.toLocaleString()}</span>
              <span className="text-subtle">Expired entries</span>
              <span className="text-fg font-mono">{logStats.expired_entries.toLocaleString()}</span>
              <span className="text-subtle">Oldest entry</span>
              <span className="text-fg font-mono text-[11px]">{logStats.oldest_entry ?? 'none'}</span>
              <span className="text-subtle">Retention</span>
              <span className="text-fg font-mono">{logStats.retention_hours}h</span>
            </div>
            {logStats.expired_entries > 0 && (
              <button
                onClick={() => pruneLogsMutation.mutate()}
                disabled={pruneLogsMutation.isPending}
                className="w-full mt-2 px-3 py-1.5 text-xs bg-red-600/20 hover:bg-red-600/30 text-red-400 border border-red-600/30 rounded-md transition-colors disabled:opacity-50"
              >
                {pruneLogsMutation.isPending
                  ? 'Pruning...'
                  : `Prune ${logStats.expired_entries.toLocaleString()} expired entries`}
              </button>
            )}
            {pruneLogsMutation.isSuccess && (
              <p className="text-[11px] text-green-400">
                Pruned {pruneLogsMutation.data.deleted.toLocaleString()} entries
              </p>
            )}
          </div>
        )}
      </div>

      {/* Server Config (env vars) */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">Server Environment</h2>
        <p className="text-xs text-subtle">
          Configured via environment variables or docker-compose.yml.
        </p>
        <div className="rounded-md border border-edge overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-edge bg-inset/50">
                <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Variable</th>
                <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Default</th>
                <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Description</th>
              </tr>
            </thead>
            <tbody className="text-xs">
              {[
                ['BIND_ADDR', '0.0.0.0:3000', 'Server listen address'],
                ['DATABASE_URL', 'data/hookbot.db', 'SQLite database path'],
                ['FIRMWARE_DIR', 'data/firmware', 'Firmware binary storage'],
                ['POLL_INTERVAL', '10', 'Device poll interval (seconds)'],
                ['LOG_RETENTION_HOURS', '24', 'Status log retention (hours, default)'],
                ['MDNS_PREFIX', 'hookbot', 'mDNS hostname prefix for discovery'],
              ].map(([name, def, desc]) => (
                <tr key={name} className="border-b border-edge/50 last:border-0">
                  <td className="px-3 py-2 font-mono dark:text-amber-400 light:text-amber-600">{name}</td>
                  <td className="px-3 py-2 font-mono text-subtle">{def}</td>
                  <td className="px-3 py-2 text-muted">{desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
