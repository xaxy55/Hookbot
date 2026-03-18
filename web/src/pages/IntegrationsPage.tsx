import { useState } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import { getDevices, getOtaJobs, sendNotification, sendWebhookNotification } from '../api/client';

interface Webhook {
  id: string;
  name: string;
  url: string;
  events: string[];
  enabled: boolean;
}

const AVAILABLE_EVENTS = [
  { key: 'state_change', label: 'State Change', desc: 'When a device changes avatar state' },
  { key: 'device_online', label: 'Device Online', desc: 'When a device comes online' },
  { key: 'device_offline', label: 'Device Offline', desc: 'When a device goes offline' },
  { key: 'ota_complete', label: 'OTA Complete', desc: 'When firmware update finishes' },
  { key: 'hook_event', label: 'Hook Event', desc: 'When a Claude Code hook fires' },
];

export default function IntegrationsPage() {
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const { data: otaJobs } = useQuery({ queryKey: ['ota-jobs'], queryFn: getOtaJobs });

  const [webhooks, setWebhooks] = useState<Webhook[]>([]);
  const [showAdd, setShowAdd] = useState(false);
  const [newName, setNewName] = useState('');
  const [newUrl, setNewUrl] = useState('');
  const [newEvents, setNewEvents] = useState<string[]>([]);
  const [testResult, setTestResult] = useState<Record<string, 'ok' | 'fail' | 'pending'>>({});

  function addWebhook() {
    if (!newName || !newUrl) return;
    const webhook: Webhook = {
      id: crypto.randomUUID(),
      name: newName,
      url: newUrl,
      events: newEvents.length > 0 ? newEvents : ['state_change'],
      enabled: true,
    };
    setWebhooks(prev => [...prev, webhook]);
    setNewName('');
    setNewUrl('');
    setNewEvents([]);
    setShowAdd(false);
  }

  function toggleWebhook(id: string) {
    setWebhooks(prev => prev.map(w => w.id === id ? { ...w, enabled: !w.enabled } : w));
  }

  function removeWebhook(id: string) {
    setWebhooks(prev => prev.filter(w => w.id !== id));
  }

  function toggleEvent(event: string) {
    setNewEvents(prev =>
      prev.includes(event) ? prev.filter(e => e !== event) : [...prev, event]
    );
  }

  async function testWebhook(id: string) {
    setTestResult(prev => ({ ...prev, [id]: 'pending' }));
    const webhook = webhooks.find(w => w.id === id);
    if (!webhook) return;
    try {
      const res = await fetch(webhook.url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ event: 'test', timestamp: new Date().toISOString() }),
      });
      setTestResult(prev => ({ ...prev, [id]: res.ok ? 'ok' : 'fail' }));
    } catch {
      setTestResult(prev => ({ ...prev, [id]: 'fail' }));
    }
    setTimeout(() => setTestResult(prev => { const n = { ...prev }; delete n[id]; return n; }), 3000);
  }

  // Build API example based on devices
  const firstDevice = devices?.[0];
  const apiBase = window.location.origin;

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-bold text-white">Integrations</h1>

      {/* Claude Code Hook */}
      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-5 space-y-4">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-purple-500/10 flex items-center justify-center">
            <ClaudeIcon />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-gray-300">Claude Code</h2>
            <p className="text-[11px] text-gray-500">Built-in hook integration for real-time avatar state</p>
          </div>
          <span className="ml-auto px-2 py-0.5 text-[10px] rounded-full bg-green-500/10 text-green-400 border border-green-500/20">Built-in</span>
        </div>
        <div className="text-xs text-gray-400 space-y-1.5">
          <p>The Claude Code hook automatically maps tool events to avatar states:</p>
          <div className="grid grid-cols-2 gap-1 mt-2">
            {[
              ['PreToolUse', 'thinking'],
              ['PostToolUse', 'idle / success'],
              ['UserPromptSubmit', 'thinking'],
              ['Stop', 'idle'],
              ['TaskCompleted', 'success'],
            ].map(([event, state]) => (
              <div key={event} className="flex items-center gap-2 px-2 py-1 rounded bg-gray-800/50">
                <span className="font-mono text-purple-400">{event}</span>
                <span className="text-gray-600">→</span>
                <span className="text-gray-300">{state}</span>
              </div>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2 pt-1">
          <a
            href="/settings"
            className="px-3 py-1.5 text-xs bg-gray-700 hover:bg-gray-600 rounded-md transition-colors text-gray-300"
          >
            Configure in Settings
          </a>
        </div>
      </div>

      {/* GitHub Webhooks */}
      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-5 space-y-4">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-gray-500/10 flex items-center justify-center">
            <GitHubIcon />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-gray-300">GitHub</h2>
            <p className="text-[11px] text-gray-500">React to pushes, PRs, CI, issues, and stars</p>
          </div>
          <span className="ml-auto px-2 py-0.5 text-[10px] rounded-full bg-gray-500/10 text-gray-400 border border-gray-500/20">Webhook</span>
        </div>

        <div className="text-xs text-gray-400 space-y-1.5">
          <p>GitHub webhooks map repository events to avatar states:</p>
          <div className="grid grid-cols-2 gap-1 mt-2">
            {[
              ['push', 'success'],
              ['pull_request (opened)', 'thinking'],
              ['pull_request (merged)', 'success'],
              ['workflow_run (success)', 'success'],
              ['workflow_run (failure)', 'error'],
              ['issues (opened)', 'thinking'],
              ['issues (closed)', 'success'],
              ['check_run (success)', 'success'],
              ['check_run (failure)', 'error'],
              ['star', 'success'],
            ].map(([event, state]) => (
              <div key={event} className="flex items-center gap-2 px-2 py-1 rounded bg-gray-800/50">
                <span className="font-mono text-gray-300">{event}</span>
                <span className="text-gray-600">→</span>
                <span className="text-gray-300">{state}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Webhook URL */}
        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="text-xs text-gray-500 font-medium">Webhook URL</label>
          </div>
          <code className="block text-[11px] bg-gray-800 rounded-md px-3 py-2 text-green-400 font-mono border border-gray-700 truncate">
            POST {apiBase}/api/hook/github
          </code>
        </div>

        {/* Setup instructions */}
        <div className="rounded-md border border-gray-500/20 bg-gray-500/5 p-3">
          <p className="text-xs text-gray-300 font-medium mb-1">Setup</p>
          <ol className="text-[11px] text-gray-400 space-y-1 list-decimal list-inside">
            <li>Go to your GitHub repo → <span className="text-gray-300">Settings → Webhooks → Add webhook</span></li>
            <li>Set <span className="text-gray-300">Payload URL</span> to <code className="text-green-400">{apiBase}/api/hook/github</code></li>
            <li>Set <span className="text-gray-300">Content type</span> to <code className="text-green-400">application/json</code></li>
            <li>Select events: <span className="text-gray-300">Pushes, Pull requests, Workflow runs, Issues, Stars</span> (or "Send me everything")</li>
            <li>Click <span className="text-gray-300">Add webhook</span></li>
          </ol>
        </div>

        <div className="flex items-center gap-2 pt-1">
          <a
            href="/settings"
            className="px-3 py-1.5 text-xs bg-gray-700 hover:bg-gray-600 rounded-md transition-colors text-gray-300"
          >
            Configure in Settings
          </a>
        </div>
      </div>

      {/* Microsoft Teams */}
      <TeamsIntegration devices={devices} />

      {/* REST API */}
      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-5 space-y-4">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-blue-500/10 flex items-center justify-center">
            <ApiIcon />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-gray-300">REST API</h2>
            <p className="text-[11px] text-gray-500">Direct API access for custom integrations</p>
          </div>
          <span className="ml-auto px-2 py-0.5 text-[10px] rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20">Open</span>
        </div>

        <div className="space-y-2">
          <p className="text-xs text-gray-500">Send state changes from any tool or script:</p>
          <pre className="text-[11px] bg-gray-800 rounded-md p-3 font-mono border border-gray-700 overflow-x-auto">
            <span className="text-gray-500"># Set avatar state</span>{'\n'}
            <span className="text-green-400">curl</span> <span className="text-blue-400">-X POST</span> {apiBase}/api/devices/{firstDevice?.id || '<device-id>'}/state \{'\n'}
            {'  '}<span className="text-blue-400">-H</span> <span className="text-yellow-400">"Content-Type: application/json"</span> \{'\n'}
            {'  '}<span className="text-blue-400">-d</span> <span className="text-yellow-400">'{`{"state":"thinking"}`}'</span>
          </pre>
          <pre className="text-[11px] bg-gray-800 rounded-md p-3 font-mono border border-gray-700 overflow-x-auto">
            <span className="text-gray-500"># Push task list to OLED</span>{'\n'}
            <span className="text-green-400">curl</span> <span className="text-blue-400">-X POST</span> {apiBase}/api/devices/{firstDevice?.id || '<device-id>'}/tasks \{'\n'}
            {'  '}<span className="text-blue-400">-H</span> <span className="text-yellow-400">"Content-Type: application/json"</span> \{'\n'}
            {'  '}<span className="text-blue-400">-d</span> <span className="text-yellow-400">'{`{"items":[{"label":"Build","status":1}],"active":0}`}'</span>
          </pre>
        </div>

        <div className="rounded-md border border-gray-700 overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-700 bg-gray-800/50">
                <th className="text-left px-3 py-2 text-xs text-gray-500 font-medium">Endpoint</th>
                <th className="text-left px-3 py-2 text-xs text-gray-500 font-medium">Method</th>
                <th className="text-left px-3 py-2 text-xs text-gray-500 font-medium">Description</th>
              </tr>
            </thead>
            <tbody className="text-xs">
              {[
                ['/api/devices', 'GET', 'List all devices with status'],
                ['/api/devices/:id/state', 'POST', 'Set avatar state'],
                ['/api/devices/:id/tasks', 'POST', 'Push task list to OLED'],
                ['/api/devices/:id/config', 'GET/PUT', 'Read/update device config'],
                ['/api/devices/:id/config/push', 'POST', 'Push config to device'],
                ['/api/hook', 'POST', 'Claude Code hook endpoint'],
                ['/api/hook/github', 'POST', 'GitHub webhook endpoint'],
                ['/api/discovery', 'GET', 'Scan network for devices'],
                ['/api/devices/:id/notifications', 'POST', 'Push notification to device'],
                ['/api/notifications/webhook', 'POST', 'Webhook for Teams/Slack/etc.'],
                ['/api/ota/deploy', 'POST', 'Deploy firmware OTA'],
              ].map(([path, method, desc]) => (
                <tr key={path} className="border-b border-gray-800/50 last:border-0">
                  <td className="px-3 py-2 font-mono text-amber-400">{path}</td>
                  <td className="px-3 py-2 font-mono text-gray-400">{method}</td>
                  <td className="px-3 py-2 text-gray-400">{desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Webhooks */}
      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-5 space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-orange-500/10 flex items-center justify-center">
              <WebhookIcon />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-gray-300">Webhooks</h2>
              <p className="text-[11px] text-gray-500">Get notified when events happen</p>
            </div>
          </div>
          <button
            onClick={() => setShowAdd(!showAdd)}
            className="px-3 py-1.5 text-xs bg-orange-500/10 text-orange-400 border border-orange-500/20 rounded-lg hover:bg-orange-500/20 transition-colors"
          >
            + Add Webhook
          </button>
        </div>

        {showAdd && (
          <div className="rounded-md border border-gray-700 bg-gray-800/50 p-4 space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-xs text-gray-500 mb-1">Name</label>
                <input
                  value={newName}
                  onChange={e => setNewName(e.target.value)}
                  placeholder="e.g. Slack notifications"
                  className="w-full px-3 py-2 text-sm bg-gray-800 border border-gray-700 rounded-md text-white placeholder-gray-600"
                />
              </div>
              <div>
                <label className="block text-xs text-gray-500 mb-1">URL</label>
                <input
                  value={newUrl}
                  onChange={e => setNewUrl(e.target.value)}
                  placeholder="https://hooks.slack.com/..."
                  className="w-full px-3 py-2 text-sm bg-gray-800 border border-gray-700 rounded-md text-white placeholder-gray-600 font-mono"
                />
              </div>
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-2">Events</label>
              <div className="flex flex-wrap gap-2">
                {AVAILABLE_EVENTS.map(evt => (
                  <button
                    key={evt.key}
                    onClick={() => toggleEvent(evt.key)}
                    className={`px-2.5 py-1 text-xs rounded-md border transition-colors ${
                      newEvents.includes(evt.key)
                        ? 'border-orange-500/30 bg-orange-500/10 text-orange-400'
                        : 'border-gray-700 bg-gray-800 text-gray-500 hover:text-gray-300'
                    }`}
                    title={evt.desc}
                  >
                    {evt.label}
                  </button>
                ))}
              </div>
            </div>
            <div className="flex gap-2 pt-1">
              <button
                onClick={addWebhook}
                disabled={!newName || !newUrl}
                className="px-3 py-1.5 text-xs bg-orange-600 hover:bg-orange-700 rounded-md text-white disabled:opacity-50"
              >
                Add
              </button>
              <button
                onClick={() => { setShowAdd(false); setNewName(''); setNewUrl(''); setNewEvents([]); }}
                className="px-3 py-1.5 text-xs bg-gray-700 hover:bg-gray-600 rounded-md text-gray-300"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {webhooks.length > 0 ? (
          <div className="space-y-2">
            {webhooks.map(wh => (
              <div key={wh.id} className="flex items-center gap-3 px-3 py-2.5 rounded-md border border-gray-700 bg-gray-800/30">
                <button
                  onClick={() => toggleWebhook(wh.id)}
                  className={`relative w-8 h-4 rounded-full transition-colors flex-shrink-0 ${wh.enabled ? 'bg-green-600' : 'bg-gray-700'}`}
                >
                  <span className={`absolute top-0.5 w-3 h-3 rounded-full bg-white transition-transform ${wh.enabled ? 'left-4' : 'left-0.5'}`} />
                </button>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-white font-medium">{wh.name}</span>
                    <span className="text-[10px] text-gray-600 font-mono truncate">{wh.url}</span>
                  </div>
                  <div className="flex gap-1 mt-1">
                    {wh.events.map(e => (
                      <span key={e} className="px-1.5 py-0.5 text-[10px] rounded bg-gray-800 text-gray-500 border border-gray-700">
                        {e}
                      </span>
                    ))}
                  </div>
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={() => testWebhook(wh.id)}
                    className={`px-2 py-1 text-[10px] rounded transition-colors ${
                      testResult[wh.id] === 'ok' ? 'bg-green-600 text-white' :
                      testResult[wh.id] === 'fail' ? 'bg-red-600 text-white' :
                      testResult[wh.id] === 'pending' ? 'bg-gray-600 text-gray-300' :
                      'bg-gray-700 hover:bg-gray-600 text-gray-400'
                    }`}
                  >
                    {testResult[wh.id] === 'ok' ? 'OK' :
                     testResult[wh.id] === 'fail' ? 'Failed' :
                     testResult[wh.id] === 'pending' ? '...' : 'Test'}
                  </button>
                  <button
                    onClick={() => removeWebhook(wh.id)}
                    className="px-2 py-1 text-[10px] bg-gray-700 hover:bg-red-900 text-gray-500 hover:text-red-400 rounded transition-colors"
                  >
                    Remove
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-xs text-gray-600 text-center py-4">No webhooks configured</p>
        )}
      </div>

      {/* Device Firmware Overview */}
      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-5 space-y-4">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-teal-500/10 flex items-center justify-center">
            <FirmwareIcon />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-gray-300">Device Firmware</h2>
            <p className="text-[11px] text-gray-500">Current firmware deployed on each device</p>
          </div>
        </div>

        {devices && devices.length > 0 ? (
          <div className="rounded-md border border-gray-700 overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-700 bg-gray-800/50">
                  <th className="text-left px-3 py-2 text-xs text-gray-500 font-medium">Device</th>
                  <th className="text-left px-3 py-2 text-xs text-gray-500 font-medium">Status</th>
                  <th className="text-left px-3 py-2 text-xs text-gray-500 font-medium">Last OTA</th>
                  <th className="text-left px-3 py-2 text-xs text-gray-500 font-medium">Result</th>
                </tr>
              </thead>
              <tbody className="text-xs">
                {devices.map(d => {
                  const lastJob = otaJobs
                    ?.filter(j => j.device_id === d.id)
                    .sort((a, b) => b.created_at.localeCompare(a.created_at))[0];
                  return (
                    <tr key={d.id} className="border-b border-gray-800/50 last:border-0">
                      <td className="px-3 py-2">
                        <div className="flex items-center gap-2">
                          <span className={`w-1.5 h-1.5 rounded-full ${d.online ? 'bg-green-500' : 'bg-gray-600'}`} />
                          <span className="text-white font-medium">{d.name}</span>
                        </div>
                      </td>
                      <td className="px-3 py-2">
                        <span className={`${d.online ? 'text-green-400' : 'text-gray-500'}`}>
                          {d.online ? 'Online' : 'Offline'}
                        </span>
                      </td>
                      <td className="px-3 py-2 text-gray-400 font-mono">
                        {lastJob ? lastJob.created_at : '-'}
                      </td>
                      <td className="px-3 py-2">
                        {lastJob ? (
                          <span className={`px-1.5 py-0.5 rounded text-[10px] ${
                            lastJob.status === 'success' ? 'bg-green-500/10 text-green-400' :
                            lastJob.status === 'failed' ? 'bg-red-500/10 text-red-400' :
                            lastJob.status === 'in_progress' ? 'bg-blue-500/10 text-blue-400' :
                            'bg-gray-800 text-gray-500'
                          }`}>
                            {lastJob.status}
                          </span>
                        ) : (
                          <span className="text-gray-600">No deployments</span>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="text-xs text-gray-600 text-center py-4">No devices registered</p>
        )}
      </div>
    </div>
  );
}

// --- Teams Integration Component ---

function TeamsIntegration({ devices }: { devices?: { id: string; name: string; online: boolean }[] }) {
  const [targetDevice, setTargetDevice] = useState('');
  const [testCount, setTestCount] = useState(3);
  const [_webhookUrl, _setWebhookUrl] = useState('');
  const [copied, setCopied] = useState('');

  const sendNotif = useMutation({
    mutationFn: (args: { unread: number; active: boolean }) => {
      if (targetDevice) {
        return sendNotification(targetDevice, 'teams', args.unread, args.active);
      }
      return sendWebhookNotification({ source: 'teams', unread: args.unread, active: args.active });
    },
  });

  const clearNotif = useMutation({
    mutationFn: () => {
      if (targetDevice) {
        return sendNotification(targetDevice, 'teams', 0, false);
      }
      return sendWebhookNotification({ source: 'teams', unread: 0, active: false });
    },
  });

  const apiBase = window.location.origin;
  const webhookEndpoint = `${apiBase}/api/notifications/webhook`;

  async function copyText(text: string, label: string) {
    await navigator.clipboard.writeText(text);
    setCopied(label);
    setTimeout(() => setCopied(''), 2000);
  }

  return (
    <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-5 space-y-4">
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-lg bg-indigo-500/10 flex items-center justify-center">
          <TeamsIcon />
        </div>
        <div>
          <h2 className="text-sm font-semibold text-gray-300">Microsoft Teams</h2>
          <p className="text-[11px] text-gray-500">Show unread message count on the hookbot OLED</p>
        </div>
        <span className="ml-auto px-2 py-0.5 text-[10px] rounded-full bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">Webhook</span>
      </div>

      {/* How it works */}
      <div className="text-xs text-gray-400 space-y-2">
        <p>When you have unread Teams messages, the hookbot shows a notification badge on the OLED display with the count. Use Power Automate or any webhook to push updates.</p>
        <div className="grid grid-cols-3 gap-2 mt-2">
          <div className="flex flex-col items-center gap-1.5 p-2 rounded bg-gray-800/50 text-center">
            <span className="text-indigo-400 text-lg">1</span>
            <span className="text-[10px] text-gray-500">Teams sends webhook via Power Automate</span>
          </div>
          <div className="flex flex-col items-center gap-1.5 p-2 rounded bg-gray-800/50 text-center">
            <span className="text-indigo-400 text-lg">2</span>
            <span className="text-[10px] text-gray-500">Server forwards to ESP32</span>
          </div>
          <div className="flex flex-col items-center gap-1.5 p-2 rounded bg-gray-800/50 text-center">
            <span className="text-indigo-400 text-lg">3</span>
            <span className="text-[10px] text-gray-500">Badge appears on OLED display</span>
          </div>
        </div>
      </div>

      {/* Webhook endpoint */}
      <div>
        <div className="flex items-center justify-between mb-1">
          <label className="text-xs text-gray-500 font-medium">Webhook Endpoint</label>
          <button
            onClick={() => copyText(webhookEndpoint, 'endpoint')}
            className="text-[10px] text-gray-500 hover:text-white transition-colors"
          >
            {copied === 'endpoint' ? 'Copied!' : 'Copy'}
          </button>
        </div>
        <div className="flex items-center gap-2">
          <code className="flex-1 text-[11px] bg-gray-800 rounded-md px-3 py-2 text-indigo-400 font-mono border border-gray-700 truncate">
            POST {webhookEndpoint}
          </code>
        </div>
      </div>

      {/* Payload example */}
      <div>
        <div className="flex items-center justify-between mb-1">
          <label className="text-xs text-gray-500 font-medium">Payload Format</label>
          <button
            onClick={() => copyText(JSON.stringify({ source: 'teams', unread: 5 }, null, 2), 'payload')}
            className="text-[10px] text-gray-500 hover:text-white transition-colors"
          >
            {copied === 'payload' ? 'Copied!' : 'Copy'}
          </button>
        </div>
        <pre className="text-[11px] bg-gray-800 rounded-md p-3 font-mono border border-gray-700 text-gray-300">
{`{
  "source": "teams",     // "teams" | "slack" | "email" | custom
  "unread": 5,           // unread message count
  "device_id": "..."     // optional, broadcasts to all if omitted
}`}
        </pre>
      </div>

      {/* Power Automate setup hint */}
      <div className="rounded-md border border-indigo-500/20 bg-indigo-500/5 p-3">
        <p className="text-xs text-indigo-300 font-medium mb-1">Power Automate Setup</p>
        <ol className="text-[11px] text-gray-400 space-y-1 list-decimal list-inside">
          <li>Create a new flow triggered by <span className="text-indigo-300">"When a new message is received in Teams"</span></li>
          <li>Add an <span className="text-indigo-300">HTTP action</span> (POST) pointing to the webhook endpoint above</li>
          <li>Set body to <code className="text-indigo-400">{`{"source":"teams","unread":1}`}</code></li>
          <li>Optionally, add a "Get unread count" step and use the dynamic value</li>
        </ol>
      </div>

      {/* Test controls */}
      <div className="rounded-md border border-gray-700 bg-gray-800/50 p-4 space-y-3">
        <h3 className="text-xs font-medium text-gray-400">Test Notifications</h3>
        <div className="flex items-end gap-3">
          <div className="flex-1">
            <label className="block text-[10px] text-gray-500 mb-1">Target Device</label>
            <select
              value={targetDevice}
              onChange={e => setTargetDevice(e.target.value)}
              className="w-full px-2 py-1.5 text-xs bg-gray-800 border border-gray-700 rounded-md text-white"
            >
              <option value="">All devices (broadcast)</option>
              {devices?.map(d => (
                <option key={d.id} value={d.id}>
                  {d.name} {d.online ? '' : '(offline)'}
                </option>
              ))}
            </select>
          </div>
          <div className="w-20">
            <label className="block text-[10px] text-gray-500 mb-1">Unread</label>
            <input
              type="number"
              min={0}
              max={99}
              value={testCount}
              onChange={e => setTestCount(Number(e.target.value))}
              className="w-full px-2 py-1.5 text-xs bg-gray-800 border border-gray-700 rounded-md text-white font-mono"
            />
          </div>
          <button
            onClick={() => sendNotif.mutate({ unread: testCount, active: true })}
            disabled={sendNotif.isPending}
            className="px-3 py-1.5 text-xs bg-indigo-600 hover:bg-indigo-700 rounded-md text-white disabled:opacity-50"
          >
            {sendNotif.isPending ? '...' : 'Send'}
          </button>
          <button
            onClick={() => clearNotif.mutate()}
            disabled={clearNotif.isPending}
            className="px-3 py-1.5 text-xs bg-gray-700 hover:bg-gray-600 rounded-md text-gray-300 disabled:opacity-50"
          >
            {clearNotif.isPending ? '...' : 'Clear'}
          </button>
        </div>
        <div className="flex gap-2">
          {[1, 3, 5, 10, 25].map(n => (
            <button
              key={n}
              onClick={() => sendNotif.mutate({ unread: n, active: true })}
              className="px-2 py-1 text-[10px] bg-gray-700 hover:bg-indigo-600 rounded text-gray-400 hover:text-white transition-colors font-mono"
            >
              {n}
            </button>
          ))}
        </div>
        {sendNotif.isSuccess && <p className="text-[10px] text-green-400">Notification sent to device!</p>}
        {sendNotif.isError && <p className="text-[10px] text-red-400">{sendNotif.error.message}</p>}
        {clearNotif.isSuccess && <p className="text-[10px] text-green-400">Notifications cleared</p>}
      </div>
    </div>
  );
}

// --- Icons ---

function TeamsIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <rect x="2" y="3" width="12" height="10" rx="2" stroke="#818CF8" strokeWidth="1.3" />
      <path d="M5 6h6M5 8.5h4" stroke="#818CF8" strokeWidth="1.2" strokeLinecap="round" />
      <circle cx="12" cy="4" r="2.5" fill="#818CF8" opacity="0.4" />
    </svg>
  );
}

function ClaudeIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path d="M8 2L3 5.5V10.5L8 14L13 10.5V5.5L8 2Z" stroke="#A855F7" strokeWidth="1.3" fill="none" />
      <circle cx="8" cy="8" r="2" fill="#A855F7" opacity="0.6" />
    </svg>
  );
}

function ApiIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#3B82F6" strokeWidth="1.3">
      <path d="M4 4l-2 4 2 4M12 4l2 4-2 4M9 3l-2 10" />
    </svg>
  );
}

function WebhookIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#F97316" strokeWidth="1.3">
      <circle cx="4" cy="12" r="2" />
      <circle cx="12" cy="4" r="2" />
      <circle cx="12" cy="12" r="2" />
      <path d="M6 12h4M12 6v4M5.5 10.5L10.5 5.5" />
    </svg>
  );
}

function GitHubIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path d="M8 1.5C4.41 1.5 1.5 4.41 1.5 8c0 2.87 1.86 5.31 4.44 6.17.33.06.44-.14.44-.31v-1.13c-1.81.39-2.19-.76-2.19-.76-.3-.75-.72-.95-.72-.95-.59-.4.04-.39.04-.39.65.05 1 .67 1 .67.58.99 1.52.7 1.89.54.06-.42.23-.71.41-.87-1.44-.16-2.96-.72-2.96-3.21 0-.71.25-1.29.67-1.75-.07-.16-.29-.83.06-1.72 0 0 .55-.18 1.79.67a6.2 6.2 0 013.24 0c1.24-.85 1.79-.67 1.79-.67.35.9.13 1.56.06 1.72.42.46.67 1.04.67 1.75 0 2.5-1.52 3.05-2.97 3.21.23.2.44.6.44 1.21v1.79c0 .17.12.38.45.31A6.5 6.5 0 0014.5 8c0-3.59-2.91-6.5-6.5-6.5z" fill="#9CA3AF"/>
    </svg>
  );
}

function FirmwareIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#14B8A6" strokeWidth="1.3">
      <rect x="3" y="2" width="10" height="12" rx="1" />
      <path d="M6 5h4M6 8h4M6 11h2" />
    </svg>
  );
}
