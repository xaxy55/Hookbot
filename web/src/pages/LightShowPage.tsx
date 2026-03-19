import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getDevices, getDeviceConfig, updateDeviceConfig, pushConfig, sendState } from '../api/client';
import type { AvatarState } from '../types';

// ─── Constants ─────────────────────────────────────────────────

const STATES: AvatarState[] = ['idle', 'thinking', 'waiting', 'success', 'taskcheck', 'error'];

const STATE_LABELS: Record<AvatarState, string> = {
  idle: 'Idle',
  thinking: 'Thinking',
  waiting: 'Waiting',
  success: 'Success',
  taskcheck: 'Task Check',
  error: 'Error',
};

const STATE_ICONS: Record<AvatarState, string> = {
  idle: '~',
  thinking: '?',
  waiting: '...',
  success: '!',
  taskcheck: '#',
  error: 'x',
};

const DEFAULT_COLORS: Record<AvatarState, string> = {
  idle: '#3b82f6',
  thinking: '#a855f7',
  waiting: '#fbbf24',
  success: '#22c55e',
  taskcheck: '#14b8a6',
  error: '#ef4444',
};

const PRESET_PALETTE = [
  '#ef4444', '#f97316', '#f59e0b', '#eab308', '#84cc16', '#22c55e',
  '#14b8a6', '#06b6d4', '#3b82f6', '#6366f1', '#8b5cf6', '#a855f7',
  '#d946ef', '#ec4899', '#f43f5e', '#ffffff', '#94a3b8', '#000000',
];

// ─── Themes ────────────────────────────────────────────────────

interface Theme {
  name: string;
  emoji: string;
  colors: Record<AvatarState, string>;
}

const THEMES: Theme[] = [
  {
    name: 'Default',
    emoji: '*',
    colors: { idle: '#3b82f6', thinking: '#a855f7', waiting: '#fef3c7', success: '#22c55e', taskcheck: '#14b8a6', error: '#ef4444' },
  },
  {
    name: 'Cyberpunk',
    emoji: '/',
    colors: { idle: '#ff007f', thinking: '#00ffff', waiting: '#ffff00', success: '#39ff14', taskcheck: '#ff00ff', error: '#ff6600' },
  },
  {
    name: 'Ocean',
    emoji: '~',
    colors: { idle: '#1e3a5f', thinking: '#14b8a6', waiting: '#67e8f9', success: '#059669', taskcheck: '#06b6d4', error: '#f87171' },
  },
  {
    name: 'Sunset',
    emoji: '-',
    colors: { idle: '#f97316', thinking: '#ec4899', waiting: '#eab308', success: '#fbbf24', taskcheck: '#f59e0b', error: '#dc2626' },
  },
  {
    name: 'Matrix',
    emoji: '>',
    colors: { idle: '#00ff00', thinking: '#00cc00', waiting: '#00aa00', success: '#00ff44', taskcheck: '#00dd00', error: '#00bb00' },
  },
  {
    name: 'Ice',
    emoji: '+',
    colors: { idle: '#bfdbfe', thinking: '#93c5fd', waiting: '#dbeafe', success: '#60a5fa', taskcheck: '#e0f2fe', error: '#38bdf8' },
  },
  {
    name: 'Lava',
    emoji: '^',
    colors: { idle: '#dc2626', thinking: '#ea580c', waiting: '#f59e0b', success: '#b91c1c', taskcheck: '#f97316', error: '#fbbf24' },
  },
  {
    name: 'Vaporwave',
    emoji: '&',
    colors: { idle: '#ff6ec7', thinking: '#8b5cf6', waiting: '#06b6d4', success: '#d946ef', taskcheck: '#a78bfa', error: '#ec4899' },
  },
  {
    name: 'Stealth',
    emoji: '.',
    colors: { idle: '#3a3530', thinking: '#4a4540', waiting: '#2a2520', success: '#5a5550', taskcheck: '#3a3a3a', error: '#4a4040' },
  },
];

// ─── Party Mode Cycle ──────────────────────────────────────────

const PARTY_CYCLE: AvatarState[] = ['success', 'error', 'thinking', 'waiting', 'taskcheck', 'idle'];

// ─── Component ─────────────────────────────────────────────────

export default function LightShowPage() {
  const queryClient = useQueryClient();

  // Device selection
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [selectedDevice, setSelectedDevice] = useState('');
  const deviceId = selectedDevice || devices?.[0]?.id;

  // Config query
  const { data: config } = useQuery({
    queryKey: ['deviceConfig', deviceId],
    queryFn: () => getDeviceConfig(deviceId!),
    enabled: !!deviceId,
  });

  // Local state
  const [colors, setColors] = useState<Record<AvatarState, string>>(DEFAULT_COLORS);
  const [brightness, setBrightness] = useState(128);
  const [activeState, setActiveState] = useState<AvatarState>('idle');
  const [hexInput, setHexInput] = useState('');
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  // Party mode
  const [partyActive, setPartyActive] = useState(false);
  const [partySpeed, setPartySpeed] = useState(500);
  const partyRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const partyIndexRef = useRef(0);

  // Sync local state from config
  useEffect(() => {
    if (config) {
      setBrightness(config.led_brightness ?? 128);
      if (config.led_colors) {
        const merged = { ...DEFAULT_COLORS };
        for (const s of STATES) {
          if (config.led_colors[s]) merged[s] = config.led_colors[s];
        }
        setColors(merged);
      }
    }
  }, [config]);

  // Keep hex input in sync with active state
  useEffect(() => {
    setHexInput(colors[activeState].replace('#', ''));
  }, [activeState, colors]);

  // ─── Mutations ───────────────────────────────────────────────

  const saveMutation = useMutation({
    mutationFn: async () => {
      if (!deviceId) throw new Error('No device selected');
      await updateDeviceConfig(deviceId, {
        led_brightness: brightness,
        led_colors: colors as Record<string, string>,
      });
      await pushConfig(deviceId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['deviceConfig', deviceId] });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    },
  });

  const sendStateMutation = useMutation({
    mutationFn: (state: AvatarState) => {
      if (!deviceId) throw new Error('No device selected');
      return sendState(deviceId, state);
    },
  });

  // ─── Handlers ────────────────────────────────────────────────

  const setColorForState = useCallback((state: AvatarState, color: string) => {
    setColors(prev => ({ ...prev, [state]: color }));
  }, []);

  const applyTheme = useCallback((theme: Theme) => {
    setColors({ ...theme.colors });
  }, []);

  const handleHexSubmit = useCallback(() => {
    let hex = hexInput.trim().replace('#', '');
    if (/^[0-9a-fA-F]{6}$/.test(hex)) {
      setColorForState(activeState, '#' + hex.toLowerCase());
    } else if (/^[0-9a-fA-F]{3}$/.test(hex)) {
      hex = hex.split('').map(c => c + c).join('');
      setColorForState(activeState, '#' + hex.toLowerCase());
    }
  }, [hexInput, activeState, setColorForState]);

  // Party mode controls
  const startParty = useCallback(() => {
    if (!deviceId) return;
    setPartyActive(true);
    partyIndexRef.current = 0;
    sendStateMutation.mutate(PARTY_CYCLE[0]);
  }, [deviceId, sendStateMutation]);

  const stopParty = useCallback(() => {
    setPartyActive(false);
    if (partyRef.current) {
      clearInterval(partyRef.current);
      partyRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (partyActive && deviceId) {
      if (partyRef.current) clearInterval(partyRef.current);
      partyRef.current = setInterval(() => {
        partyIndexRef.current = (partyIndexRef.current + 1) % PARTY_CYCLE.length;
        const state = PARTY_CYCLE[partyIndexRef.current];
        sendStateMutation.mutate(state);
      }, partySpeed);
    } else if (partyRef.current) {
      clearInterval(partyRef.current);
      partyRef.current = null;
    }
    return () => {
      if (partyRef.current) clearInterval(partyRef.current);
    };
  }, [partyActive, partySpeed, deviceId]);

  const handleSave = useCallback(() => {
    setSaving(true);
    saveMutation.mutate(undefined, {
      onSettled: () => setSaving(false),
    });
  }, [saveMutation]);

  // ─── Derived ─────────────────────────────────────────────────

  const brightnessPercent = Math.round((brightness / 255) * 100);
  const currentColor = colors[activeState];

  // ─── Render ──────────────────────────────────────────────────

  return (
    <div className="space-y-6 max-w-4xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-fg">LED Light Show</h1>
          <p className="text-subtle text-sm mt-1">Customize LED colors per avatar state, pick themes, and run party mode.</p>
        </div>

        {/* Device selector */}
        {devices && devices.length > 1 && (
          <select
            className="bg-surface border border-edge rounded px-3 py-2 text-fg text-sm"
            value={selectedDevice}
            onChange={e => setSelectedDevice(e.target.value)}
          >
            {devices.map(d => (
              <option key={d.id} value={d.id}>{d.name}</option>
            ))}
          </select>
        )}
      </div>

      {!deviceId && (
        <div className="bg-surface border border-edge rounded-lg p-8 text-center text-muted">
          No devices found. Register a device first.
        </div>
      )}

      {deviceId && (
        <>
          {/* ── Sync Row ─────────────────────────────────────── */}
          <div className="bg-surface border border-edge rounded-lg p-4">
            <div className="text-xs text-muted uppercase tracking-wider mb-3 font-medium">All State Colors</div>
            <div className="flex items-center justify-center gap-3">
              {STATES.map(s => (
                <button
                  key={s}
                  onClick={() => setActiveState(s)}
                  className={`flex flex-col items-center gap-1.5 group transition-transform ${activeState === s ? 'scale-110' : 'hover:scale-105'}`}
                >
                  <div
                    className={`w-12 h-12 rounded-full border-2 transition-all shadow-lg ${
                      activeState === s ? 'border-white ring-2 ring-offset-2 ring-offset-surface' : 'border-edge'
                    }`}
                    style={{
                      backgroundColor: colors[s],
                      boxShadow: activeState === s ? `0 0 20px ${colors[s]}80` : `0 0 8px ${colors[s]}40`,
                    }}
                  />
                  <span className={`text-xs font-medium ${activeState === s ? 'text-fg' : 'text-muted'}`}>
                    {STATE_LABELS[s]}
                  </span>
                </button>
              ))}
            </div>
          </div>

          {/* ── Color Editor ─────────────────────────────────── */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Active state preview */}
            <div className="bg-surface border border-edge rounded-lg p-6 flex flex-col items-center justify-center">
              <div className="text-xs text-muted uppercase tracking-wider mb-4 font-medium">
                {STATE_LABELS[activeState]} State
              </div>
              <div
                className="w-32 h-32 rounded-2xl border-2 border-edge mb-4 transition-colors duration-300"
                style={{
                  backgroundColor: currentColor,
                  boxShadow: `0 0 40px ${currentColor}60, 0 0 80px ${currentColor}30`,
                  opacity: brightness / 255,
                }}
              />
              <div className="text-lg font-mono font-bold text-fg">{currentColor.toUpperCase()}</div>
              <div className="text-xs text-muted mt-1">
                State icon: {STATE_ICONS[activeState]}
              </div>

              {/* Hex input */}
              <div className="flex items-center gap-2 mt-4 w-full max-w-xs">
                <span className="text-muted font-mono">#</span>
                <input
                  type="text"
                  className="bg-inset border border-edge rounded px-3 py-2 text-fg font-mono text-sm flex-1 w-0"
                  value={hexInput}
                  onChange={e => setHexInput(e.target.value.replace('#', ''))}
                  onBlur={handleHexSubmit}
                  onKeyDown={e => e.key === 'Enter' && handleHexSubmit()}
                  maxLength={6}
                  placeholder="ff00ff"
                />
                <input
                  type="color"
                  value={currentColor}
                  onChange={e => setColorForState(activeState, e.target.value)}
                  className="w-10 h-10 rounded cursor-pointer border border-edge bg-transparent"
                />
              </div>
            </div>

            {/* Preset colors */}
            <div className="bg-surface border border-edge rounded-lg p-6">
              <div className="text-xs text-muted uppercase tracking-wider mb-3 font-medium">Quick Colors</div>
              <div className="grid grid-cols-6 gap-2">
                {PRESET_PALETTE.map(color => (
                  <button
                    key={color}
                    onClick={() => setColorForState(activeState, color)}
                    className={`w-full aspect-square rounded-lg border-2 transition-all hover:scale-110 ${
                      currentColor === color ? 'border-white scale-110' : 'border-edge'
                    }`}
                    style={{
                      backgroundColor: color,
                      boxShadow: currentColor === color ? `0 0 12px ${color}80` : undefined,
                    }}
                    title={color}
                  />
                ))}
              </div>

              {/* State tabs for quick switching */}
              <div className="mt-4 pt-4 border-t border-edge">
                <div className="text-xs text-muted uppercase tracking-wider mb-2 font-medium">Edit State</div>
                <div className="flex flex-wrap gap-1.5">
                  {STATES.map(s => (
                    <button
                      key={s}
                      onClick={() => setActiveState(s)}
                      className={`px-3 py-1.5 rounded text-xs font-medium transition-all ${
                        activeState === s
                          ? 'text-white shadow-md'
                          : 'bg-inset text-muted hover:text-fg border border-edge'
                      }`}
                      style={activeState === s ? {
                        backgroundColor: colors[s],
                        boxShadow: `0 0 10px ${colors[s]}60`,
                      } : undefined}
                    >
                      {STATE_LABELS[s]}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>

          {/* ── Brightness ───────────────────────────────────── */}
          <div className="bg-surface border border-edge rounded-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <div>
                <div className="text-xs text-muted uppercase tracking-wider font-medium">Brightness</div>
                <div className="text-2xl font-bold text-fg mt-1">{brightnessPercent}%</div>
              </div>
              <div
                className="w-16 h-16 rounded-full border-2 border-edge transition-all"
                style={{
                  backgroundColor: currentColor,
                  opacity: brightness / 255,
                  boxShadow: `0 0 ${Math.round(brightness / 5)}px ${currentColor}${Math.round(brightness / 2.55).toString(16).padStart(2, '0')}`,
                }}
              />
            </div>
            <input
              type="range"
              min={0}
              max={255}
              value={brightness}
              onChange={e => setBrightness(Number(e.target.value))}
              className="w-full h-3 rounded-full appearance-none cursor-pointer"
              style={{
                background: `linear-gradient(to right, #1e1e1e 0%, ${currentColor} 100%)`,
              }}
            />
            <div className="flex justify-between text-xs text-dim mt-1">
              <span>Off</span>
              <span>Max</span>
            </div>
          </div>

          {/* ── Themes ───────────────────────────────────────── */}
          <div className="bg-surface border border-edge rounded-lg p-6">
            <div className="text-xs text-muted uppercase tracking-wider mb-4 font-medium">Color Themes</div>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
              {THEMES.map(theme => {
                const isActive = STATES.every(s => colors[s] === theme.colors[s]);
                return (
                  <button
                    key={theme.name}
                    onClick={() => applyTheme(theme)}
                    className={`rounded-lg border-2 p-3 text-left transition-all hover:scale-[1.02] ${
                      isActive
                        ? 'border-brand-fg bg-brand/15 shadow-md'
                        : 'border-edge bg-inset hover:border-subtle'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm font-semibold text-fg">{theme.name}</span>
                      {isActive && <span className="text-xs text-brand-fg font-medium">Active</span>}
                    </div>
                    <div className="flex gap-1">
                      {STATES.map(s => (
                        <div
                          key={s}
                          className="flex-1 h-6 rounded first:rounded-l-md last:rounded-r-md"
                          style={{
                            backgroundColor: theme.colors[s],
                            boxShadow: `inset 0 -1px 2px rgba(0,0,0,0.2)`,
                          }}
                        />
                      ))}
                    </div>
                  </button>
                );
              })}
            </div>
          </div>

          {/* ── Party Mode ───────────────────────────────────── */}
          <div className={`border rounded-lg p-6 transition-all ${
            partyActive
              ? 'bg-surface border-brand-fg shadow-lg shadow-brand/20'
              : 'bg-surface border-edge'
          }`}>
            <div className="flex items-center justify-between mb-4">
              <div>
                <div className="text-xs text-muted uppercase tracking-wider font-medium">Party Mode</div>
                <p className="text-xs text-dim mt-1">Rapidly cycles through states to create a light show effect.</p>
              </div>
              {partyActive && (
                <div className="flex gap-1">
                  {PARTY_CYCLE.map((s, i) => (
                    <div
                      key={i}
                      className="w-3 h-3 rounded-full animate-pulse"
                      style={{
                        backgroundColor: colors[s],
                        animationDelay: `${i * 100}ms`,
                      }}
                    />
                  ))}
                </div>
              )}
            </div>

            <div className="flex flex-col sm:flex-row items-start sm:items-center gap-4">
              {/* Big party button */}
              <button
                onClick={partyActive ? stopParty : startParty}
                disabled={!deviceId}
                className={`px-8 py-4 rounded-xl font-bold text-lg transition-all ${
                  partyActive
                    ? 'bg-red-600 hover:bg-red-700 text-white shadow-lg shadow-red-600/30'
                    : 'text-white shadow-lg hover:scale-105'
                }`}
                style={!partyActive ? {
                  background: `linear-gradient(135deg, ${colors.success}, ${colors.thinking}, ${colors.error})`,
                  boxShadow: `0 4px 20px ${colors.thinking}40`,
                } : undefined}
              >
                {partyActive ? '[ STOP ]' : '>>> START PARTY <<<'}
              </button>

              {/* Speed control */}
              <div className="flex-1 w-full sm:w-auto">
                <div className="flex items-center justify-between text-xs text-muted mb-1">
                  <span>Speed</span>
                  <span className="font-mono">{partySpeed}ms</span>
                </div>
                <input
                  type="range"
                  min={200}
                  max={2000}
                  step={50}
                  value={partySpeed}
                  onChange={e => setPartySpeed(Number(e.target.value))}
                  className="w-full h-2 rounded-full appearance-none cursor-pointer bg-inset"
                />
                <div className="flex justify-between text-xs text-dim mt-0.5">
                  <span>Fast</span>
                  <span>Slow</span>
                </div>
              </div>
            </div>
          </div>

          {/* ── Save & Push ──────────────────────────────────── */}
          <div className="flex items-center justify-between bg-surface border border-edge rounded-lg p-4">
            <div className="text-sm text-muted">
              {saveMutation.isError && (
                <span className="text-red-400">Error: {(saveMutation.error as Error).message}</span>
              )}
              {saved && (
                <span className="text-green-400 font-medium">Saved and pushed to device!</span>
              )}
              {!saved && !saveMutation.isError && (
                <span>Save colors and push config to the device.</span>
              )}
            </div>
            <button
              onClick={handleSave}
              disabled={saving || !deviceId}
              className={`px-6 py-3 rounded-lg font-semibold text-sm transition-all ${
                saving
                  ? 'bg-brand/15 text-brand-fg cursor-wait'
                  : saved
                    ? 'bg-green-600 text-white'
                    : 'bg-brand/15 text-brand-fg hover:bg-brand/25 hover:scale-105'
              }`}
            >
              {saving ? 'Saving...' : saved ? 'Done!' : 'Save & Push to Device'}
            </button>
          </div>
        </>
      )}
    </div>
  );
}
