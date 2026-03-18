import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getDevice, getDeviceConfig, getDeviceHistory, sendState, sendTasks, updateDevice, updateDeviceConfig, pushConfig, getOtaJobs, getFirmware, getServos, setServoAngle, restServos, configureServos, getSensors, updateSensors, getRules, createRule, updateRule, deleteRule } from '../api/client';
import type { ServoChannel, SensorChannelConfig, AutomationRule } from '../api/client';
import StateIndicator from '../components/StateIndicator';
import type { AvatarState } from '../types';

const STATES: AvatarState[] = ['idle', 'thinking', 'waiting', 'success', 'taskcheck', 'error'];

const STATE_BTN_COLORS: Record<AvatarState, string> = {
  idle: 'bg-blue-900 hover:bg-blue-800',
  thinking: 'bg-purple-900 hover:bg-purple-800',
  waiting: 'bg-yellow-900 hover:bg-yellow-800',
  success: 'bg-green-900 hover:bg-green-800',
  taskcheck: 'bg-teal-900 hover:bg-teal-800',
  error: 'bg-red-900 hover:bg-red-800',
};

function formatUptime(ms: number): string {
  const secs = Math.floor(ms / 1000);
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  return `${h}h ${m}m ${s}s`;
}

function formatBytes(bytes: number): string {
  return `${(bytes / 1024).toFixed(1)} KB`;
}

type Tab = 'status' | 'identity' | 'hardware' | 'servos' | 'personality' | 'sensors' | 'automation' | 'history';

export default function DeviceDetail() {
  const { id } = useParams<{ id: string }>();
  const qc = useQueryClient();
  const [tab, setTab] = useState<Tab>('status');

  const { data: device } = useQuery({
    queryKey: ['device', id],
    queryFn: () => getDevice(id!),
    refetchInterval: 5000,
    enabled: !!id,
  });

  const { data: config } = useQuery({
    queryKey: ['config', id],
    queryFn: () => getDeviceConfig(id!),
    enabled: !!id,
  });

  const { data: history } = useQuery({
    queryKey: ['history', id],
    queryFn: () => getDeviceHistory(id!),
    refetchInterval: 10000,
    enabled: !!id,
  });

  const { data: otaJobs } = useQuery({ queryKey: ['ota-jobs'], queryFn: getOtaJobs });
  const { data: firmwares } = useQuery({ queryKey: ['firmware'], queryFn: getFirmware });

  const lastSuccessJob = otaJobs
    ?.filter(j => j.device_id === id && j.status === 'success')
    .sort((a, b) => b.created_at.localeCompare(a.created_at))[0];
  const currentFirmware = lastSuccessJob
    ? firmwares?.find(f => f.id === lastSuccessJob.firmware_id)
    : undefined;

  const stateMut = useMutation({
    mutationFn: (state: string) => sendState(id!, state),
  });

  if (!device) return <p className="text-subtle">Loading...</p>;

  const TABS: { key: Tab; label: string }[] = [
    { key: 'status', label: 'Status' },
    { key: 'identity', label: 'Identity' },
    { key: 'hardware', label: 'Hardware' },
    { key: 'servos', label: 'Servos' },
    { key: 'personality', label: 'Personality' },
    { key: 'sensors', label: 'Sensors' },
    { key: 'automation', label: 'Automation' },
    { key: 'history', label: 'History' },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-fg">{device.name}</h1>
          <p className="text-sm text-subtle">
            {device.hostname}.local ({device.ip_address})
          </p>
        </div>
        <div className="flex items-center gap-2">
          {device.device_type && (
            <span className={`px-2 py-1 rounded-full text-[10px] border font-mono ${
              device.device_type === 'esp32_4848s040c_lcd'
                ? 'bg-cyan-500/10 text-cyan-400 border-cyan-500/20'
                : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
            }`}>
              {device.device_type === 'esp32_4848s040c_lcd' ? 'LCD Touch' : 'OLED'}
            </span>
          )}
          {currentFirmware && (
            <span className="px-2 py-1 rounded-full text-[10px] bg-teal-500/10 text-teal-400 border border-teal-500/20 font-mono">
              fw v{currentFirmware.version}
            </span>
          )}
          <div className={`px-3 py-1 rounded-full text-xs ${device.online ? 'bg-green-900/50 text-green-400' : 'bg-inset text-subtle'}`}>
            {device.online ? 'Online' : 'Offline'}
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 border-b border-edge pb-px">
        {TABS.map(t => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`px-4 py-2 text-sm rounded-t-md transition-colors ${
              tab === t.key
                ? 'bg-inset text-fg border-b-2 border-red-500'
                : 'text-subtle hover:text-fg-2'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {tab === 'status' && (
        <div className="space-y-6">
          {device.latest_status && (
            <div className="rounded-lg border border-edge bg-surface p-4">
              <div className="flex items-center gap-6">
                <StateIndicator state={device.latest_status.state} size="lg" />
                <div className="text-sm text-muted">
                  Uptime: <span className="text-fg">{formatUptime(device.latest_status.uptime_ms)}</span>
                </div>
                <div className="text-sm text-muted">
                  Free heap: <span className="text-fg">{formatBytes(device.latest_status.free_heap)}</span>
                </div>
              </div>
            </div>
          )}

          <div className="rounded-lg border border-edge bg-surface p-4">
            <h2 className="text-sm font-semibold text-fg-2 mb-3">Manual Controls</h2>
            <div className="grid grid-cols-3 sm:grid-cols-6 gap-2">
              {STATES.map((s) => (
                <button
                  key={s}
                  onClick={() => stateMut.mutate(s)}
                  disabled={stateMut.isPending}
                  className={`px-3 py-2 text-sm rounded-md text-fg capitalize ${STATE_BTN_COLORS[s]} disabled:opacity-50`}
                >
                  {s}
                </button>
              ))}
            </div>
          </div>

          {currentFirmware && (
            <div className="rounded-lg border border-edge bg-surface p-4">
              <h2 className="text-sm font-semibold text-fg-2 mb-3">Firmware</h2>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 text-sm">
                <div>
                  <span className="text-xs text-subtle block">Version</span>
                  <span className="text-teal-400 font-mono">{currentFirmware.version}</span>
                </div>
                <div>
                  <span className="text-xs text-subtle block">Filename</span>
                  <span className="text-fg">{currentFirmware.filename}</span>
                </div>
                <div>
                  <span className="text-xs text-subtle block">Size</span>
                  <span className="text-fg">{(currentFirmware.size_bytes / 1024).toFixed(1)} KB</span>
                </div>
                <div>
                  <span className="text-xs text-subtle block">Deployed</span>
                  <span className="text-fg font-mono text-xs">{lastSuccessJob?.updated_at}</span>
                </div>
              </div>
              {currentFirmware.notes && (
                <p className="text-xs text-muted mt-2">{currentFirmware.notes}</p>
              )}
            </div>
          )}

          <TaskListControl deviceId={id!} />
        </div>
      )}

      {tab === 'identity' && (
        <IdentityTab deviceId={id!} device={device} onSave={() => qc.invalidateQueries({ queryKey: ['device', id] })} />
      )}

      {tab === 'hardware' && config && (
        <HardwareTab deviceId={id!} config={config} onSave={() => qc.invalidateQueries({ queryKey: ['config', id] })} />
      )}

      {tab === 'servos' && (
        <ServosTab deviceId={id!} online={device.online} />
      )}

      {tab === 'personality' && config && (
        <PersonalityTab deviceId={id!} config={config} onSave={() => qc.invalidateQueries({ queryKey: ['config', id] })} />
      )}

      {tab === 'sensors' && (
        <SensorsTab deviceId={id!} online={device.online} />
      )}

      {tab === 'automation' && (
        <AutomationTab deviceId={id!} />
      )}

      {tab === 'history' && (
        <div className="rounded-lg border border-edge bg-surface overflow-hidden">
          {history && history.length > 0 ? (
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-edge text-subtle text-left">
                  <th className="px-4 py-3 font-medium">Time</th>
                  <th className="px-4 py-3 font-medium">State</th>
                  <th className="px-4 py-3 font-medium text-right">Uptime</th>
                  <th className="px-4 py-3 font-medium text-right">Free Heap</th>
                </tr>
              </thead>
              <tbody>
                {history.map((s, i) => (
                  <tr key={i} className="border-b border-edge/50 last:border-0">
                    <td className="px-4 py-2.5 text-muted font-mono text-xs">{s.recorded_at}</td>
                    <td className="px-4 py-2.5"><StateIndicator state={s.state} /></td>
                    <td className="px-4 py-2.5 text-muted text-right font-mono text-xs">{formatUptime(s.uptime_ms)}</td>
                    <td className="px-4 py-2.5 text-muted text-right font-mono text-xs">{formatBytes(s.free_heap)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="text-center py-8 text-dim text-sm">No history</div>
          )}
        </div>
      )}
    </div>
  );
}

// --- Identity Tab ---

function IdentityTab({ deviceId, device, onSave }: {
  deviceId: string;
  device: { name: string; hostname: string; ip_address: string; purpose?: string; personality?: string };
  onSave: () => void;
}) {
  const [name, setName] = useState(device.name);
  const [hostname, setHostname] = useState(device.hostname);
  const [ip, setIp] = useState(device.ip_address);
  const [purpose, setPurpose] = useState(device.purpose || '');
  const [personality, setPersonality] = useState(device.personality || '');

  const update = useMutation({
    mutationFn: () => updateDevice(deviceId, { name, hostname, ip_address: ip, purpose, personality }),
    onSuccess: onSave,
  });

  return (
    <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
      <h2 className="text-sm font-semibold text-fg-2">Device Identity</h2>
      <Field label="Name" value={name} onChange={setName} placeholder="e.g. CEO Bot" />
      <Field label="Hostname" value={hostname} onChange={setHostname} placeholder="e.g. hookbot" hint="Used for mDNS (hostname.local)" />
      <Field label="IP Address" value={ip} onChange={setIp} placeholder="192.168.1.x" />
      <Field label="Purpose / Role" value={purpose} onChange={setPurpose} placeholder="e.g. Calendar notifications, code assistant" />
      <div>
        <label className="block text-xs text-subtle mb-1">Personality</label>
        <textarea
          value={personality}
          onChange={e => setPersonality(e.target.value)}
          placeholder="Describe the bot's personality for prompt context..."
          rows={3}
          className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg placeholder-dim resize-none"
        />
      </div>

      <div className="flex items-center gap-3 pt-2">
        <button
          onClick={() => update.mutate()}
          disabled={update.isPending}
          className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
        >
          {update.isPending ? 'Saving...' : 'Save Identity'}
        </button>
        {update.isSuccess && <span className="text-xs text-green-400">Saved</span>}
        {update.isError && <span className="text-xs text-red-400">{update.error.message}</span>}
      </div>
    </div>
  );
}

// --- Hardware Tab ---

function HardwareTab({ deviceId, config, onSave }: {
  deviceId: string;
  config: { led_brightness: number; sound_enabled: boolean; sound_volume: number; custom_data?: Record<string, unknown> | null };
  onSave: () => void;
}) {
  const hw = (config.custom_data as Record<string, unknown>) || {};
  const [ledPin, setLedPin] = useState(String(hw.led_pin ?? 16));
  const [numLeds, setNumLeds] = useState(String(hw.num_leds ?? 1));
  const [buzzerPin, setBuzzerPin] = useState(String(hw.buzzer_pin ?? 25));
  const [oledSda, setOledSda] = useState(String(hw.oled_sda ?? 21));
  const [oledScl, setOledScl] = useState(String(hw.oled_scl ?? 22));
  const [screenW, setScreenW] = useState(String(hw.screen_width ?? 128));
  const [screenH, setScreenH] = useState(String(hw.screen_height ?? 64));
  const [displayEnabled, setDisplayEnabled] = useState(hw.display_enabled !== false);
  const [soundEnabled, setSoundEnabled] = useState(config.sound_enabled);

  const update = useMutation({
    mutationFn: () => updateDeviceConfig(deviceId, {
      sound_enabled: soundEnabled,
      custom_data: {
        ...hw,
        led_pin: Number(ledPin),
        num_leds: Number(numLeds),
        buzzer_pin: Number(buzzerPin),
        oled_sda: Number(oledSda),
        oled_scl: Number(oledScl),
        screen_width: Number(screenW),
        screen_height: Number(screenH),
        display_enabled: displayEnabled,
      },
    }),
    onSuccess: onSave,
  });

  const pushMut = useMutation({
    mutationFn: () => pushConfig(deviceId),
  });

  return (
    <div className="space-y-6">
      {/* GPIO Pins */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">GPIO Pins</h2>
        <div className="grid grid-cols-2 sm:grid-cols-3 gap-4">
          <Field label="LED Pin" value={ledPin} onChange={setLedPin} type="number" />
          <Field label="Num LEDs" value={numLeds} onChange={setNumLeds} type="number" />
          <Field label="Buzzer Pin" value={buzzerPin} onChange={setBuzzerPin} type="number" />
          <Field label="OLED SDA" value={oledSda} onChange={setOledSda} type="number" />
          <Field label="OLED SCL" value={oledScl} onChange={setOledScl} type="number" />
        </div>
      </div>

      {/* Display */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">Display</h2>
        <Toggle label="Display Enabled" value={displayEnabled} onChange={setDisplayEnabled} />
        <div className="grid grid-cols-2 gap-4">
          <Field label="Width (px)" value={screenW} onChange={setScreenW} type="number" />
          <Field label="Height (px)" value={screenH} onChange={setScreenH} type="number" />
        </div>
      </div>

      {/* Audio */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">Audio</h2>
        <Toggle label="Sound Enabled" value={soundEnabled} onChange={setSoundEnabled} />
      </div>

      <div className="flex items-center gap-3">
        <button
          onClick={() => update.mutate()}
          disabled={update.isPending}
          className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
        >
          {update.isPending ? 'Saving...' : 'Save Hardware Config'}
        </button>
        <button
          onClick={() => pushMut.mutate()}
          disabled={pushMut.isPending}
          className="px-4 py-2 text-sm bg-raised hover:bg-raised rounded-md disabled:opacity-50"
        >
          {pushMut.isPending ? 'Pushing...' : 'Push to Device'}
        </button>
        {update.isSuccess && <span className="text-xs text-green-400">Saved</span>}
        {pushMut.isSuccess && <span className="text-xs text-green-400">Pushed</span>}
      </div>
    </div>
  );
}

// --- Personality Tab ---

const DEFAULT_LED_COLORS: Record<string, string> = {
  idle: '#00003c', thinking: '#6600cc', waiting: '#b8860b',
  success: '#006400', taskcheck: '#005555', error: '#cc0000',
};

function PersonalityTab({ deviceId, config, onSave }: {
  deviceId: string;
  config: { led_brightness: number; sound_volume: number; led_colors?: Record<string, string> | null; avatar_preset?: Record<string, unknown> | null };
  onSave: () => void;
}) {
  const [brightness, setBrightness] = useState(config.led_brightness);
  const [volume, setVolume] = useState(config.sound_volume);
  const [ledColors, setLedColors] = useState<Record<string, string>>({
    ...DEFAULT_LED_COLORS,
    ...(config.led_colors || {}),
  });
  const avatarPreset = config.avatar_preset ? JSON.stringify(config.avatar_preset, null, 2) : '';

  const update = useMutation({
    mutationFn: () => updateDeviceConfig(deviceId, {
      led_brightness: brightness,
      sound_volume: volume,
      led_colors: ledColors,
      avatar_preset: avatarPreset ? JSON.parse(avatarPreset) : undefined,
    }),
    onSuccess: onSave,
  });

  const pushMut = useMutation({
    mutationFn: () => pushConfig(deviceId),
  });

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">LED</h2>
        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="text-xs text-subtle">Brightness</label>
            <span className="text-xs font-mono text-subtle">{brightness}</span>
          </div>
          <input
            type="range" min={0} max={255} value={brightness}
            onChange={e => setBrightness(Number(e.target.value))}
            className="w-full"
          />
        </div>

        <div>
          <label className="text-xs text-subtle mb-2 block">Per-State Colors</label>
          <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
            {STATES.map(s => (
              <div key={s} className="flex items-center gap-2">
                <input
                  type="color"
                  value={ledColors[s] || DEFAULT_LED_COLORS[s]}
                  onChange={e => setLedColors(prev => ({ ...prev, [s]: e.target.value }))}
                  className="w-8 h-8 rounded border border-edge bg-inset cursor-pointer"
                />
                <span className="text-xs text-muted capitalize">{s}</span>
              </div>
            ))}
          </div>
          <button
            onClick={() => setLedColors({ ...DEFAULT_LED_COLORS })}
            className="mt-2 text-[10px] text-subtle hover:text-fg-2"
          >
            Reset to defaults
          </button>
        </div>
      </div>

      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">Sound</h2>
        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="text-xs text-subtle">Volume</label>
            <span className="text-xs font-mono text-subtle">{volume}%</span>
          </div>
          <input
            type="range" min={0} max={100} value={volume}
            onChange={e => setVolume(Number(e.target.value))}
            className="w-full"
          />
        </div>
      </div>

      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-fg-2">Avatar Preset</h2>
          <Link
            to={`/avatar?device=${deviceId}`}
            className="px-3 py-1.5 text-xs bg-red-500/10 text-red-400 border border-red-500/30 rounded-lg hover:bg-red-500/20 transition-colors"
          >
            Open Avatar Editor
          </Link>
        </div>
        {avatarPreset ? (
          <pre className="text-[11px] bg-inset rounded-md p-3 overflow-x-auto text-muted font-mono max-h-40">{avatarPreset}</pre>
        ) : (
          <p className="text-xs text-dim">No avatar preset configured. Use the Avatar Editor to design one.</p>
        )}
      </div>

      <div className="flex items-center gap-3">
        <button
          onClick={() => update.mutate()}
          disabled={update.isPending}
          className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
        >
          {update.isPending ? 'Saving...' : 'Save Personality'}
        </button>
        <button
          onClick={() => pushMut.mutate()}
          disabled={pushMut.isPending}
          className="px-4 py-2 text-sm bg-raised hover:bg-raised rounded-md disabled:opacity-50"
        >
          {pushMut.isPending ? 'Pushing...' : 'Push to Device'}
        </button>
        {update.isSuccess && <span className="text-xs text-green-400">Saved</span>}
        {pushMut.isSuccess && <span className="text-xs text-green-400">Pushed</span>}
      </div>
    </div>
  );
}

// --- Task List Control ---

function TaskListControl({ deviceId }: { deviceId: string }) {
  const [tasks, setTasks] = useState([
    { label: 'Read files', status: 2 },
    { label: 'Plan changes', status: 2 },
    { label: 'Write code', status: 1 },
    { label: 'Run tests', status: 0 },
    { label: 'Commit', status: 0 },
  ]);
  const [activeIdx, setActiveIdx] = useState(2);

  const pushTasks = useMutation({
    mutationFn: () => sendTasks(deviceId, tasks, activeIdx),
  });

  const clearTasks = useMutation({
    mutationFn: () => sendTasks(deviceId, [], 0),
  });

  function cycleStatus(i: number) {
    setTasks(prev => {
      const next = [...prev];
      next[i] = { ...next[i], status: (next[i].status + 1) % 4 };
      return next;
    });
  }

  function addTask() {
    setTasks(prev => [...prev, { label: 'New task', status: 0 }]);
  }

  function removeTask(i: number) {
    setTasks(prev => prev.filter((_, idx) => idx !== i));
    if (activeIdx >= tasks.length - 1) setActiveIdx(Math.max(0, activeIdx - 1));
  }

  const STATUS_LABELS = ['Pending', 'Active', 'Done', 'Failed'];
  const STATUS_COLORS = ['text-subtle', 'text-blue-400', 'text-green-400', 'text-red-400'];

  return (
    <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold text-fg-2">Task List (OLED)</h2>
        <div className="flex gap-1">
          <button onClick={addTask} className="px-2 py-1 text-[10px] bg-raised hover:bg-raised rounded text-fg-2">+ Add</button>
          <button
            onClick={() => pushTasks.mutate()}
            disabled={pushTasks.isPending}
            className="px-2 py-1 text-[10px] bg-green-600 hover:bg-green-700 rounded text-fg disabled:opacity-50"
          >
            {pushTasks.isPending ? '...' : 'Push'}
          </button>
          <button
            onClick={() => clearTasks.mutate()}
            disabled={clearTasks.isPending}
            className="px-2 py-1 text-[10px] bg-raised hover:bg-raised rounded text-fg-2 disabled:opacity-50"
          >
            Clear
          </button>
        </div>
      </div>
      <div className="space-y-1">
        {tasks.map((t, i) => (
          <div key={i} className="flex items-center gap-2 text-sm">
            <button
              onClick={() => setActiveIdx(i)}
              className={`w-5 h-5 flex-shrink-0 rounded-full border text-[10px] flex items-center justify-center ${
                i === activeIdx ? 'border-blue-500 bg-blue-500/20 text-blue-400' : 'border-edge text-dim'
              }`}
            >
              {i + 1}
            </button>
            <input
              value={t.label}
              onChange={e => setTasks(prev => {
                const next = [...prev];
                next[i] = { ...next[i], label: e.target.value };
                return next;
              })}
              className="flex-1 px-2 py-1 text-xs bg-inset border border-edge rounded text-fg"
              maxLength={15}
            />
            <button
              onClick={() => cycleStatus(i)}
              className={`px-2 py-0.5 text-[10px] rounded ${STATUS_COLORS[t.status]} bg-inset border border-edge`}
            >
              {STATUS_LABELS[t.status]}
            </button>
            <button onClick={() => removeTask(i)} className="text-dim hover:text-red-400 text-xs">x</button>
          </div>
        ))}
      </div>
      {pushTasks.isSuccess && <p className="text-[10px] text-green-400">Sent to OLED!</p>}
      {clearTasks.isSuccess && <p className="text-[10px] text-green-400">Cleared</p>}
    </div>
  );
}

// --- Servos Tab ---

const STATE_LABELS: Record<string, string> = {
  idle: 'Idle', thinking: 'Thinking', waiting: 'Waiting',
  success: 'Success', taskcheck: 'Task Check', error: 'Error',
};

function ServosTab({ deviceId, online }: { deviceId: string; online: boolean }) {
  const qc = useQueryClient();

  const { data: servoData, isLoading } = useQuery({
    queryKey: ['servos', deviceId],
    queryFn: () => getServos(deviceId),
    enabled: online,
    refetchInterval: 5000,
  });

  const [editChannels, setEditChannels] = useState<Partial<ServoChannel>[] | null>(null);
  const [editMaps, setEditMaps] = useState<Record<string, number[]> | null>(null);

  const channels = servoData?.channels ?? [];
  const stateMaps = servoData?.state_maps ?? {};

  const angleMut = useMutation({
    mutationFn: ({ ch, angle }: { ch: number; angle: number }) => setServoAngle(deviceId, ch, angle),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['servos', deviceId] }),
  });

  const restMut = useMutation({
    mutationFn: () => restServos(deviceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['servos', deviceId] }),
  });

  const configMut = useMutation({
    mutationFn: (config: { channels?: Partial<ServoChannel>[]; state_maps?: Record<string, number[]> }) =>
      configureServos(deviceId, config),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['servos', deviceId] });
      setEditChannels(null);
      setEditMaps(null);
    },
  });

  if (!online) {
    return (
      <div className="text-center py-12 rounded-lg border border-edge bg-surface">
        <p className="text-subtle text-sm">Device is offline</p>
        <p className="text-dim text-xs mt-1">Servo configuration is read directly from the device</p>
      </div>
    );
  }

  if (isLoading) return <p className="text-subtle text-sm">Loading servo data...</p>;

  const enabledChannels = channels.filter(c => c.enabled);

  return (
    <div className="space-y-6">
      {/* Live Angle Control */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-fg-2">Angle Control</h2>
          <button
            onClick={() => restMut.mutate()}
            disabled={restMut.isPending}
            className="px-3 py-1.5 text-xs bg-raised hover:bg-inset rounded-md text-fg-2 disabled:opacity-50 transition-colors"
          >
            {restMut.isPending ? 'Moving...' : 'Rest All'}
          </button>
        </div>

        {enabledChannels.length === 0 ? (
          <p className="text-xs text-dim">No servos enabled. Configure pins below to enable channels.</p>
        ) : (
          <div className="space-y-3">
            {channels.map((ch, i) => {
              if (!ch.enabled) return null;
              return (
                <div key={i}>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-xs text-subtle">
                      Ch {i}: {ch.label || `Servo ${i}`}
                    </label>
                    <span className="text-xs font-mono text-subtle">{ch.current}°</span>
                  </div>
                  <input
                    type="range"
                    min={ch.min}
                    max={ch.max}
                    value={ch.current}
                    onChange={e => angleMut.mutate({ ch: i, angle: Number(e.target.value) })}
                    className="w-full"
                  />
                  <div className="flex justify-between text-[10px] text-dim">
                    <span>{ch.min}°</span>
                    <span className="text-muted">rest: {ch.rest}°</span>
                    <span>{ch.max}°</span>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Pin Configuration */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">Pin Configuration</h2>
        <p className="text-[11px] text-dim">Set pin to -1 to disable a channel. Changes are saved to device NVS.</p>

        {(() => {
          const editing = editChannels ?? channels.map(c => ({ ...c }));
          return (
            <>
              <div className="space-y-3">
                {editing.map((ch, i) => (
                  <div key={i} className="rounded-md border border-edge bg-inset/50 p-3 space-y-2">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-medium text-fg-2">Channel {i}</span>
                      <Toggle
                        label=""
                        value={ch.pin !== undefined && ch.pin >= 0}
                        onChange={v => {
                          const next = [...editing];
                          next[i] = { ...next[i], pin: v ? 0 : -1 };
                          setEditChannels(next);
                        }}
                      />
                    </div>
                    <div className="grid grid-cols-2 sm:grid-cols-5 gap-2">
                      <div>
                        <label className="block text-[10px] text-dim mb-0.5">Pin</label>
                        <input
                          type="number"
                          value={ch.pin ?? -1}
                          onChange={e => {
                            const next = [...editing];
                            next[i] = { ...next[i], pin: Number(e.target.value) };
                            setEditChannels(next);
                          }}
                          className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                        />
                      </div>
                      <div>
                        <label className="block text-[10px] text-dim mb-0.5">Min</label>
                        <input
                          type="number"
                          value={ch.min ?? 0}
                          onChange={e => {
                            const next = [...editing];
                            next[i] = { ...next[i], min: Number(e.target.value) };
                            setEditChannels(next);
                          }}
                          className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                        />
                      </div>
                      <div>
                        <label className="block text-[10px] text-dim mb-0.5">Max</label>
                        <input
                          type="number"
                          value={ch.max ?? 180}
                          onChange={e => {
                            const next = [...editing];
                            next[i] = { ...next[i], max: Number(e.target.value) };
                            setEditChannels(next);
                          }}
                          className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                        />
                      </div>
                      <div>
                        <label className="block text-[10px] text-dim mb-0.5">Rest</label>
                        <input
                          type="number"
                          value={ch.rest ?? 90}
                          onChange={e => {
                            const next = [...editing];
                            next[i] = { ...next[i], rest: Number(e.target.value) };
                            setEditChannels(next);
                          }}
                          className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                        />
                      </div>
                      <div>
                        <label className="block text-[10px] text-dim mb-0.5">Label</label>
                        <input
                          type="text"
                          value={ch.label ?? ''}
                          onChange={e => {
                            const next = [...editing];
                            next[i] = { ...next[i], label: e.target.value };
                            setEditChannels(next);
                          }}
                          className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg"
                          maxLength={16}
                        />
                      </div>
                    </div>
                  </div>
                ))}
              </div>

              <div className="flex items-center gap-3">
                <button
                  onClick={() => configMut.mutate({ channels: editing })}
                  disabled={configMut.isPending}
                  className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
                >
                  {configMut.isPending ? 'Saving...' : 'Save Pin Config'}
                </button>
                {editChannels && (
                  <button onClick={() => setEditChannels(null)} className="text-xs text-subtle hover:text-fg-2">
                    Cancel
                  </button>
                )}
                {configMut.isSuccess && !editChannels && <span className="text-xs text-green-400">Saved to device</span>}
              </div>
            </>
          );
        })()}
      </div>

      {/* State Map Editor */}
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">State Maps</h2>
        <p className="text-[11px] text-dim">
          Define servo angles for each avatar state. When the state changes, servos animate to these positions.
        </p>

        {(() => {
          const maps = editMaps ?? { ...stateMaps };
          const stateKeys = STATES.map(s => s as string);

          return (
            <>
              <div className="space-y-3">
                {stateKeys.map(state => {
                  const angles = maps[state] ?? Array(channels.length).fill(90);
                  return (
                    <div key={state} className="rounded-md border border-edge bg-inset/50 p-3">
                      <div className="flex items-center gap-2 mb-2">
                        <StateIndicator state={state} />
                        <span className="text-xs font-medium text-fg-2">{STATE_LABELS[state] ?? state}</span>
                      </div>
                      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                        {channels.map((ch, i) => (
                          <div key={i}>
                            <label className="block text-[10px] text-dim mb-0.5">
                              {ch.label || `Ch ${i}`}
                            </label>
                            <input
                              type="number"
                              min={ch.min}
                              max={ch.max}
                              value={angles[i] ?? 90}
                              onChange={e => {
                                const nextMaps = { ...maps };
                                const nextAngles = [...angles];
                                nextAngles[i] = Number(e.target.value);
                                nextMaps[state] = nextAngles;
                                setEditMaps(nextMaps);
                              }}
                              className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                            />
                          </div>
                        ))}
                      </div>
                    </div>
                  );
                })}
              </div>

              <div className="flex items-center gap-3">
                <button
                  onClick={() => configMut.mutate({ state_maps: maps })}
                  disabled={configMut.isPending}
                  className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
                >
                  {configMut.isPending ? 'Saving...' : 'Save State Maps'}
                </button>
                {editMaps && (
                  <button onClick={() => setEditMaps(null)} className="text-xs text-subtle hover:text-fg-2">
                    Cancel
                  </button>
                )}
              </div>
            </>
          );
        })()}
      </div>
    </div>
  );
}

// --- Sensors Tab ---

const SENSOR_TYPES = ['disabled', 'digital', 'analog'];

function SensorsTab({ deviceId, online }: { deviceId: string; online: boolean }) {
  const qc = useQueryClient();
  const { data: sensors, isLoading } = useQuery({
    queryKey: ['sensors', deviceId],
    queryFn: () => getSensors(deviceId),
    enabled: online,
    refetchInterval: 3000,
  });

  const [editing, setEditing] = useState<Partial<SensorChannelConfig>[] | null>(null);

  const saveMut = useMutation({
    mutationFn: (channels: Partial<SensorChannelConfig>[]) => updateSensors(deviceId, channels),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['sensors', deviceId] });
      setEditing(null);
    },
  });

  if (!online) {
    return (
      <div className="text-center py-12 rounded-lg border border-edge bg-surface">
        <p className="text-subtle text-sm">Device is offline</p>
        <p className="text-dim text-xs mt-1">Sensor configuration is read directly from the device</p>
      </div>
    );
  }

  if (isLoading) return <p className="text-subtle text-sm">Loading sensors...</p>;

  const channels = editing ?? (sensors || Array.from({ length: 8 }, (_, i) => ({
    channel: i, pin: -1, sensor_type: 'disabled', label: '', poll_interval_ms: 1000, threshold: 0,
  })));

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <h2 className="text-sm font-semibold text-fg-2">Sensor Channels</h2>
        <p className="text-[11px] text-dim">Configure GPIO sensors. Digital for buttons/PIR, Analog for light/temp sensors.</p>

        <div className="space-y-3">
          {channels.map((ch, i) => (
            <div key={i} className="rounded-md border border-edge bg-inset/50 p-3 space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium text-fg-2">Channel {i}</span>
                {sensors?.[i]?.last_value !== undefined && ch.sensor_type !== 'disabled' && (
                  <span className="text-xs font-mono text-green-400">
                    Value: {sensors[i].last_value}
                  </span>
                )}
              </div>
              <div className="grid grid-cols-2 sm:grid-cols-5 gap-2">
                <div>
                  <label className="block text-[10px] text-dim mb-0.5">Type</label>
                  <select
                    value={ch.sensor_type || 'disabled'}
                    onChange={e => {
                      const next = [...channels];
                      next[i] = { ...next[i], sensor_type: e.target.value, channel: i };
                      setEditing(next);
                    }}
                    className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg"
                  >
                    {SENSOR_TYPES.map(t => <option key={t} value={t}>{t}</option>)}
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] text-dim mb-0.5">Pin</label>
                  <input
                    type="number" value={ch.pin ?? -1}
                    onChange={e => {
                      const next = [...channels];
                      next[i] = { ...next[i], pin: Number(e.target.value), channel: i };
                      setEditing(next);
                    }}
                    className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                  />
                </div>
                <div>
                  <label className="block text-[10px] text-dim mb-0.5">Label</label>
                  <input
                    type="text" value={ch.label || ''}
                    onChange={e => {
                      const next = [...channels];
                      next[i] = { ...next[i], label: e.target.value, channel: i };
                      setEditing(next);
                    }}
                    className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg"
                    maxLength={15}
                  />
                </div>
                <div>
                  <label className="block text-[10px] text-dim mb-0.5">Poll (ms)</label>
                  <input
                    type="number" value={ch.poll_interval_ms ?? 1000}
                    onChange={e => {
                      const next = [...channels];
                      next[i] = { ...next[i], poll_interval_ms: Number(e.target.value), channel: i };
                      setEditing(next);
                    }}
                    className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                  />
                </div>
                <div>
                  <label className="block text-[10px] text-dim mb-0.5">Threshold</label>
                  <input
                    type="number" value={ch.threshold ?? 0}
                    onChange={e => {
                      const next = [...channels];
                      next[i] = { ...next[i], threshold: Number(e.target.value), channel: i };
                      setEditing(next);
                    }}
                    className="w-full px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                  />
                </div>
              </div>
            </div>
          ))}
        </div>

        <div className="flex items-center gap-3">
          <button
            onClick={() => saveMut.mutate(channels as Partial<SensorChannelConfig>[])}
            disabled={saveMut.isPending}
            className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
          >
            {saveMut.isPending ? 'Saving...' : 'Save & Push to Device'}
          </button>
          {editing && (
            <button onClick={() => setEditing(null)} className="text-xs text-subtle hover:text-fg-2">Cancel</button>
          )}
          {saveMut.isSuccess && !editing && <span className="text-xs text-green-400">Saved</span>}
        </div>
      </div>
    </div>
  );
}

// --- Automation Tab ---

const TRIGGER_TYPES = ['sensor_threshold', 'state_change', 'time_of_day', 'button_press', 'webhook'];
const ACTION_TYPES = ['change_state', 'move_servo', 'send_notification', 'call_webhook', 'play_sound'];

function AutomationTab({ deviceId }: { deviceId: string }) {
  const qc = useQueryClient();
  const { data: rules, isLoading } = useQuery({
    queryKey: ['rules', deviceId],
    queryFn: () => getRules(deviceId),
  });

  const [showForm, setShowForm] = useState(false);
  const [name, setName] = useState('');
  const [triggerType, setTriggerType] = useState('sensor_threshold');
  const [triggerConfig, setTriggerConfig] = useState('{}');
  const [actionType, setActionType] = useState('change_state');
  const [actionConfig, setActionConfig] = useState('{}');
  const [cooldown, setCooldown] = useState(60);

  const createMut = useMutation({
    mutationFn: () => createRule(deviceId, {
      name,
      trigger_type: triggerType,
      trigger_config: JSON.parse(triggerConfig || '{}'),
      action_type: actionType,
      action_config: JSON.parse(actionConfig || '{}'),
      cooldown_secs: cooldown,
    }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['rules', deviceId] });
      setShowForm(false);
      setName('');
      setTriggerConfig('{}');
      setActionConfig('{}');
    },
  });

  const toggleMut = useMutation({
    mutationFn: ({ ruleId, enabled }: { ruleId: string; enabled: boolean }) =>
      updateRule(deviceId, ruleId, { enabled }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['rules', deviceId] }),
  });

  const deleteMut = useMutation({
    mutationFn: (ruleId: string) => deleteRule(deviceId, ruleId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['rules', deviceId] }),
  });

  if (isLoading) return <p className="text-subtle text-sm">Loading rules...</p>;

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-fg-2">Automation Rules</h2>
          <button
            onClick={() => setShowForm(!showForm)}
            className="px-3 py-1.5 text-xs bg-green-600 hover:bg-green-700 rounded-md"
          >
            {showForm ? 'Cancel' : '+ New Rule'}
          </button>
        </div>

        {showForm && (
          <div className="rounded-md border border-edge bg-inset/50 p-4 space-y-3">
            <Field label="Rule Name" value={name} onChange={setName} placeholder="e.g. Motion → Wake" />
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-subtle mb-1">Trigger Type</label>
                <select value={triggerType} onChange={e => setTriggerType(e.target.value)}
                  className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg">
                  {TRIGGER_TYPES.map(t => <option key={t} value={t}>{t.replace(/_/g, ' ')}</option>)}
                </select>
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Action Type</label>
                <select value={actionType} onChange={e => setActionType(e.target.value)}
                  className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg">
                  {ACTION_TYPES.map(t => <option key={t} value={t}>{t.replace(/_/g, ' ')}</option>)}
                </select>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-subtle mb-1">Trigger Config (JSON)</label>
                <textarea value={triggerConfig} onChange={e => setTriggerConfig(e.target.value)}
                  rows={2} className="w-full px-3 py-2 text-xs bg-inset border border-edge rounded-md text-fg font-mono resize-none" />
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Action Config (JSON)</label>
                <textarea value={actionConfig} onChange={e => setActionConfig(e.target.value)}
                  rows={2} className="w-full px-3 py-2 text-xs bg-inset border border-edge rounded-md text-fg font-mono resize-none" />
              </div>
            </div>
            <Field label="Cooldown (seconds)" value={String(cooldown)} onChange={v => setCooldown(Number(v))} type="number" />
            <button
              onClick={() => createMut.mutate()}
              disabled={createMut.isPending || !name}
              className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
            >
              {createMut.isPending ? 'Creating...' : 'Create Rule'}
            </button>
          </div>
        )}

        {rules && rules.length > 0 ? (
          <div className="space-y-2">
            {rules.map(rule => (
              <div key={rule.id} className="rounded-md border border-edge bg-inset/50 p-3 flex items-center justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className={`w-2 h-2 rounded-full ${rule.enabled ? 'bg-green-500' : 'bg-dim'}`} />
                    <span className="text-sm font-medium text-fg truncate">{rule.name}</span>
                  </div>
                  <div className="text-[10px] text-muted mt-0.5 font-mono">
                    {rule.trigger_type.replace(/_/g, ' ')} → {rule.action_type.replace(/_/g, ' ')}
                    {rule.cooldown_secs > 0 && <span className="ml-2 text-dim">({rule.cooldown_secs}s cooldown)</span>}
                  </div>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <button
                    onClick={() => toggleMut.mutate({ ruleId: rule.id, enabled: !rule.enabled })}
                    className={`px-2 py-1 text-[10px] rounded ${rule.enabled ? 'bg-green-600/20 text-green-400' : 'bg-raised text-subtle'}`}
                  >
                    {rule.enabled ? 'On' : 'Off'}
                  </button>
                  <button
                    onClick={() => deleteMut.mutate(rule.id)}
                    className="text-dim hover:text-red-400 text-xs"
                  >
                    ✕
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-6 text-dim text-sm">
            No automation rules yet. Create one to automate device behavior.
          </div>
        )}
      </div>
    </div>
  );
}

// --- Shared Components ---

function Field({ label, value, onChange, placeholder, hint, type = 'text' }: {
  label: string; value: string; onChange: (v: string) => void;
  placeholder?: string; hint?: string; type?: string;
}) {
  return (
    <div>
      <label className="block text-xs text-subtle mb-1">{label}</label>
      <input
        type={type}
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg placeholder-dim"
      />
      {hint && <p className="text-[10px] text-dim mt-0.5">{hint}</p>}
    </div>
  );
}

function Toggle({ label, value, onChange }: { label: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-muted">{label}</span>
      <button
        onClick={() => onChange(!value)}
        className={`relative w-10 h-5 rounded-full transition-colors ${value ? 'bg-green-600' : 'bg-raised'}`}
      >
        <span className={`absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform ${value ? 'left-5' : 'left-0.5'}`} />
      </button>
    </div>
  );
}
