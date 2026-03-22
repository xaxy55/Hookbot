import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getDevices, getHealth, getServerSettings, updateServerSettings, getLogStats, pruneLogs, getProjectRoutes, createProjectRoute, updateProjectRoute, deleteProjectRoute, getVerifiedPublishers, addVerifiedPublisher, removeVerifiedPublisher } from '../api/client';
import type { ProjectRoute, VerifiedPublisher } from '../api/client';
import QRCode from '../components/QRCode';

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

  // Project routing state
  const { data: projectRoutes } = useQuery({ queryKey: ['projectRoutes'], queryFn: getProjectRoutes });
  const [showAddRoute, setShowAddRoute] = useState(false);
  const [routePath, setRoutePath] = useState('');
  const [routeDeviceId, setRouteDeviceId] = useState('');
  const [routeLabel, setRouteLabel] = useState('');
  const [editingRouteId, setEditingRouteId] = useState<string | null>(null);
  const [editPath, setEditPath] = useState('');
  const [editDeviceId, setEditDeviceId] = useState('');
  const [editLabel, setEditLabel] = useState('');

  const createRouteMutation = useMutation({
    mutationFn: (data: { project_path: string; device_id: string; label?: string }) => createProjectRoute(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['projectRoutes'] });
      setShowAddRoute(false);
      setRoutePath('');
      setRouteDeviceId('');
      setRouteLabel('');
    },
  });

  const updateRouteMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { project_path?: string; device_id?: string; label?: string } }) => updateProjectRoute(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['projectRoutes'] });
      setEditingRouteId(null);
    },
  });

  const deleteRouteMutation = useMutation({
    mutationFn: (id: string) => deleteProjectRoute(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['projectRoutes'] });
    },
  });

  // Verified publishers state
  const { data: publishers } = useQuery({ queryKey: ['verifiedPublishers'], queryFn: getVerifiedPublishers });
  const [showAddPublisher, setShowAddPublisher] = useState(false);
  const [pubName, setPubName] = useState('');
  const [pubDisplayName, setPubDisplayName] = useState('');
  const [pubBadgeType, setPubBadgeType] = useState('verified');

  const addPublisherMutation = useMutation({
    mutationFn: (data: { name: string; display_name: string; badge_type?: string }) => addVerifiedPublisher(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['verifiedPublishers'] });
      queryClient.invalidateQueries({ queryKey: ['community-plugins'] });
      queryClient.invalidateQueries({ queryKey: ['shared-assets'] });
      setShowAddPublisher(false);
      setPubName('');
      setPubDisplayName('');
      setPubBadgeType('verified');
    },
  });

  const removePublisherMutation = useMutation({
    mutationFn: (id: string) => removeVerifiedPublisher(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['verifiedPublishers'] });
      queryClient.invalidateQueries({ queryKey: ['community-plugins'] });
      queryClient.invalidateQueries({ queryKey: ['shared-assets'] });
    },
  });

  function startEditRoute(route: ProjectRoute) {
    setEditingRouteId(route.id);
    setEditPath(route.project_path);
    setEditDeviceId(route.device_id);
    setEditLabel(route.label ?? '');
  }

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

<<<<<<< Updated upstream
      {/* QR Code Quick Pair */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-sm font-semibold text-fg-2">Quick Pair</h2>
            <p className="text-xs text-subtle mt-1">
              Scan this QR code from your phone to open the Hookbot dashboard.
            </p>
          </div>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className="text-subtle">
            <rect x="1" y="1" width="5" height="5" rx="0.5" />
            <rect x="10" y="1" width="5" height="5" rx="0.5" />
            <rect x="1" y="10" width="5" height="5" rx="0.5" />
            <rect x="11" y="11" width="4" height="4" rx="0.5" />
            <path d="M10 10h1M14 10h1M10 14h1" />
          </svg>
        </div>

        <div className="flex items-center gap-6">
          <div className="rounded-lg bg-white p-3 shrink-0">
            <QRCode value={window.location.origin} size={140} fgColor="#000000" bgColor="#ffffff" />
          </div>
          <div className="space-y-3 text-xs">
            <div>
              <span className="text-subtle block mb-1">Dashboard URL</span>
              <code className="text-green-400 bg-inset px-2 py-1 rounded text-[11px] font-mono block break-all">
                {window.location.origin}
              </code>
            </div>
            <p className="text-subtle leading-relaxed">
              Point your phone camera at the QR code to open this dashboard. Make sure your phone is on the same network.
            </p>
            <button
              onClick={() => {
                navigator.clipboard.writeText(window.location.origin);
                setCopied('qr-url');
                setTimeout(() => setCopied(''), 2000);
              }}
              className="px-3 py-1.5 text-xs border border-edge rounded-md text-subtle hover:text-fg hover:border-muted transition-colors"
            >
              {copied === 'qr-url' ? 'Copied!' : 'Copy URL'}
            </button>
          </div>
        </div>
      </div>

=======
>>>>>>> Stashed changes
      {/* GitHub Webhook */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">GitHub Webhook</h2>
        <p className="text-xs text-subtle">
          Receive GitHub events (pushes, PRs, CI, issues) and map them to avatar states. Works the same way as the Claude Code hook.
        </p>

        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="text-xs text-subtle font-medium">Webhook URL</label>
            <button
              onClick={() => copyToClipboard(`${hookHost}/api/hook/github`, 'github-url')}
              className="text-[10px] text-subtle hover:text-fg transition-colors"
            >
              {copied === 'github-url' ? 'Copied!' : 'Copy'}
            </button>
          </div>
          <pre className="text-[11px] bg-inset rounded-md p-3 text-green-400 font-mono border border-edge">
            {hookHost}/api/hook/github
          </pre>
        </div>

        {hookDeviceId && (
          <div>
            <div className="flex items-center justify-between mb-1">
              <label className="text-xs text-subtle font-medium">With device targeting</label>
              <button
                onClick={() => copyToClipboard(`${hookHost}/api/hook/github?device_id=${hookDeviceId}`, 'github-url-device')}
                className="text-[10px] text-subtle hover:text-fg transition-colors"
              >
                {copied === 'github-url-device' ? 'Copied!' : 'Copy'}
              </button>
            </div>
            <pre className="text-[11px] bg-inset rounded-md p-3 text-blue-400 font-mono border border-edge">
              {hookHost}/api/hook/github?device_id={hookDeviceId}
            </pre>
          </div>
        )}

        <div className="text-[11px] text-subtle space-y-1">
          <p>In your GitHub repo: <span className="text-fg">Settings → Webhooks → Add webhook</span></p>
          <p>Content type: <code className="text-green-400 bg-inset px-1 rounded">application/json</code></p>
          <p>Events: Pushes, Pull requests, Workflow runs, Issues, Stars</p>
        </div>
      </div>

<<<<<<< Updated upstream
      {/* Project Routing */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-fg-2">Project Routing</h2>
          <button
            onClick={() => setShowAddRoute(!showAddRoute)}
            className="px-3 py-1.5 text-xs bg-blue-600 hover:bg-blue-500 text-white rounded-md transition-colors"
          >
            {showAddRoute ? 'Cancel' : 'Add Route'}
          </button>
        </div>
        <p className="text-xs text-subtle">
          Route hook events from specific project directories to different devices. The project path should match the working directory sent by the hook script (e.g. <code className="text-green-400 bg-inset px-1 rounded">/Users/you/projects/myapp</code>).
        </p>

        {/* Add route form */}
        {showAddRoute && (
          <div className="rounded-md border border-edge bg-inset/50 p-4 space-y-3">
            <div>
              <label className="block text-xs text-subtle mb-1">Project Path</label>
              <input
                value={routePath}
                onChange={e => setRoutePath(e.target.value)}
                placeholder="/Users/you/projects/myapp"
                className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg font-mono"
              />
            </div>
            <div>
              <label className="block text-xs text-subtle mb-1">Target Device</label>
              <select
                value={routeDeviceId}
                onChange={e => setRouteDeviceId(e.target.value)}
                className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg"
              >
                <option value="">Select a device...</option>
                {devices?.map(d => <option key={d.id} value={d.id}>{d.name} ({d.ip_address})</option>)}
              </select>
            </div>
            <div>
              <label className="block text-xs text-subtle mb-1">Label (optional)</label>
              <input
                value={routeLabel}
                onChange={e => setRouteLabel(e.target.value)}
                placeholder="e.g. Work project"
                className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg"
              />
            </div>
            <button
              onClick={() => {
                if (routePath && routeDeviceId) {
                  createRouteMutation.mutate({
                    project_path: routePath,
                    device_id: routeDeviceId,
                    ...(routeLabel ? { label: routeLabel } : {}),
                  });
                }
              }}
              disabled={!routePath || !routeDeviceId || createRouteMutation.isPending}
              className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-md transition-colors"
            >
              {createRouteMutation.isPending ? 'Adding...' : 'Add Route'}
            </button>
          </div>
        )}

        {/* Routes table */}
        {projectRoutes && projectRoutes.length > 0 ? (
          <div className="rounded-md border border-edge overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-edge bg-inset/50">
                  <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Project Path</th>
                  <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Device</th>
                  <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Label</th>
                  <th className="text-right px-3 py-2 text-xs text-subtle font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="text-xs">
                {projectRoutes.map(route => (
                  <tr key={route.id} className="border-b border-edge/50 last:border-0">
                    {editingRouteId === route.id ? (
                      <>
                        <td className="px-3 py-2">
                          <input
                            value={editPath}
                            onChange={e => setEditPath(e.target.value)}
                            className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                          />
                        </td>
                        <td className="px-3 py-2">
                          <select
                            value={editDeviceId}
                            onChange={e => setEditDeviceId(e.target.value)}
                            className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg"
                          >
                            {devices?.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
                          </select>
                        </td>
                        <td className="px-3 py-2">
                          <input
                            value={editLabel}
                            onChange={e => setEditLabel(e.target.value)}
                            className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg"
                          />
                        </td>
                        <td className="px-3 py-2 text-right space-x-2">
                          <button
                            onClick={() => updateRouteMutation.mutate({
                              id: route.id,
                              data: { project_path: editPath, device_id: editDeviceId, label: editLabel },
                            })}
                            disabled={updateRouteMutation.isPending}
                            className="text-green-400 hover:text-green-300 transition-colors"
                          >
                            Save
                          </button>
                          <button
                            onClick={() => setEditingRouteId(null)}
                            className="text-subtle hover:text-fg transition-colors"
                          >
                            Cancel
                          </button>
                        </td>
                      </>
                    ) : (
                      <>
                        <td className="px-3 py-2 font-mono text-fg">{route.project_path}</td>
                        <td className="px-3 py-2 text-fg">{route.device_name ?? route.device_id}</td>
                        <td className="px-3 py-2 text-subtle">{route.label ?? '-'}</td>
                        <td className="px-3 py-2 text-right space-x-2">
                          <button
                            onClick={() => startEditRoute(route)}
                            className="text-blue-400 hover:text-blue-300 transition-colors"
                          >
                            Edit
                          </button>
                          <button
                            onClick={() => deleteRouteMutation.mutate(route.id)}
                            disabled={deleteRouteMutation.isPending}
                            className="text-red-400 hover:text-red-300 transition-colors"
                          >
                            Delete
                          </button>
                        </td>
                      </>
                    )}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="rounded-md border border-edge bg-inset/50 p-4 text-center">
            <p className="text-xs text-subtle">No project routes configured. All hook events will be sent to the default device.</p>
          </div>
        )}
      </div>

      {/* Verified Publishers */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-sm font-semibold text-fg-2">Verified Publishers</h2>
            <p className="text-xs text-subtle mt-1">
              Manage verified publisher badges for the community plugin store and shared assets.
            </p>
          </div>
          <button
            onClick={() => setShowAddPublisher(!showAddPublisher)}
            className="px-3 py-1.5 text-xs bg-blue-600 hover:bg-blue-500 text-white rounded-md transition-colors"
          >
            {showAddPublisher ? 'Cancel' : 'Add Publisher'}
          </button>
        </div>

        {/* Add publisher form */}
        {showAddPublisher && (
          <div className="rounded-md border border-edge bg-inset/50 p-4 space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-xs text-subtle mb-1">Publisher Name (must match author field)</label>
                <input
                  value={pubName}
                  onChange={e => setPubName(e.target.value)}
                  placeholder="e.g. xaxy"
                  className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg font-mono"
                />
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Display Name</label>
                <input
                  value={pubDisplayName}
                  onChange={e => setPubDisplayName(e.target.value)}
                  placeholder="e.g. Xaxy Official"
                  className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg"
                />
              </div>
            </div>
            <div>
              <label className="block text-xs text-subtle mb-1">Badge Type</label>
              <select
                value={pubBadgeType}
                onChange={e => setPubBadgeType(e.target.value)}
                className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg"
              >
                <option value="verified">Verified</option>
                <option value="official">Official</option>
                <option value="partner">Partner</option>
              </select>
            </div>
            <button
              onClick={() => {
                if (pubName && pubDisplayName) {
                  addPublisherMutation.mutate({
                    name: pubName,
                    display_name: pubDisplayName,
                    badge_type: pubBadgeType,
                  });
                }
              }}
              disabled={!pubName || !pubDisplayName || addPublisherMutation.isPending}
              className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-md transition-colors"
            >
              {addPublisherMutation.isPending ? 'Adding...' : 'Add Publisher'}
            </button>
          </div>
        )}

        {/* Publishers table */}
        {publishers && publishers.length > 0 ? (
          <div className="rounded-md border border-edge overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-edge bg-inset/50">
                  <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Name</th>
                  <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Display Name</th>
                  <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Badge</th>
                  <th className="text-left px-3 py-2 text-xs text-subtle font-medium">Verified At</th>
                  <th className="text-right px-3 py-2 text-xs text-subtle font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="text-xs">
                {publishers.map((pub: VerifiedPublisher) => (
                  <tr key={pub.id} className="border-b border-edge/50 last:border-0">
                    <td className="px-3 py-2 font-mono text-fg flex items-center gap-1.5">
                      <svg className="w-3.5 h-3.5 text-blue-400" viewBox="0 0 20 20" fill="currentColor">
                        <path fillRule="evenodd" d="M6.267 3.455a3.066 3.066 0 001.745-.723 3.066 3.066 0 013.976 0 3.066 3.066 0 001.745.723 3.066 3.066 0 012.812 2.812c.051.643.304 1.254.723 1.745a3.066 3.066 0 010 3.976 3.066 3.066 0 00-.723 1.745 3.066 3.066 0 01-2.812 2.812 3.066 3.066 0 00-1.745.723 3.066 3.066 0 01-3.976 0 3.066 3.066 0 00-1.745-.723 3.066 3.066 0 01-2.812-2.812 3.066 3.066 0 00-.723-1.745 3.066 3.066 0 010-3.976 3.066 3.066 0 00.723-1.745 3.066 3.066 0 012.812-2.812zm7.44 5.252a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                      </svg>
                      {pub.name}
                    </td>
                    <td className="px-3 py-2 text-fg">{pub.display_name}</td>
                    <td className="px-3 py-2">
                      <span className={`px-1.5 py-0.5 text-[10px] rounded font-medium uppercase tracking-wider ${
                        pub.badge_type === 'official' ? 'bg-green-800 text-green-300' :
                        pub.badge_type === 'partner' ? 'bg-purple-800 text-purple-300' :
                        'bg-blue-800 text-blue-300'
                      }`}>
                        {pub.badge_type}
                      </span>
                    </td>
                    <td className="px-3 py-2 text-subtle">{new Date(pub.verified_at).toLocaleDateString()}</td>
                    <td className="px-3 py-2 text-right">
                      <button
                        onClick={() => removePublisherMutation.mutate(pub.id)}
                        disabled={removePublisherMutation.isPending}
                        className="text-red-400 hover:text-red-300 transition-colors"
                      >
                        Remove
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="rounded-md border border-edge bg-inset/50 p-4 text-center">
            <p className="text-xs text-subtle">No verified publishers yet. Add one to show verification badges on their plugins and assets.</p>
          </div>
        )}
      </div>

=======
>>>>>>> Stashed changes
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
