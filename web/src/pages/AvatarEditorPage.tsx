import { useState, useRef, useEffect, useCallback } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getDevices, updateDeviceConfig, getDeviceConfig, pushConfig, getOwnedItems } from '../api/client';
import type { AvatarState } from '../types';

interface AvatarParams {
  eyeX: number;
  eyeY: number;
  eyeOpen: number;
  mouthCurve: number;
  mouthOpen: number;
  bounce: number;
  shake: number;
  browAngle: number;
  browY: number;
}

interface Accessories {
  topHat: boolean;
  cigar: boolean;
  glasses: boolean;
  monocle: boolean;
  bowtie: boolean;
  crown: boolean;
  horns: boolean;
  halo: boolean;
}

const DEFAULT_PARAMS: AvatarParams = {
  eyeX: 0, eyeY: 0, eyeOpen: 0.9,
  mouthCurve: 0.15, mouthOpen: 0,
  bounce: 0, shake: 0,
  browAngle: -0.4, browY: 0,
};

const DEFAULT_ACCESSORIES: Accessories = {
  topHat: true, cigar: true, glasses: false,
  monocle: false, bowtie: false, crown: false,
  horns: false, halo: false,
};

const STATE_PRESETS: Record<AvatarState, AvatarParams> = {
  idle:      { eyeX: 0, eyeY: 0, eyeOpen: 0.9, mouthCurve: 0.15, mouthOpen: 0, bounce: 0, shake: 0, browAngle: -0.4, browY: 0 },
  thinking:  { eyeX: 0.5, eyeY: -0.2, eyeOpen: 0.7, mouthCurve: -0.2, mouthOpen: 0, bounce: 0, shake: 0, browAngle: -0.7, browY: -1 },
  waiting:   { eyeX: 0, eyeY: 0, eyeOpen: 0.85, mouthCurve: -0.5, mouthOpen: 0.2, bounce: 0, shake: 0, browAngle: -0.6, browY: -1 },
  success:   { eyeX: 0, eyeY: 0, eyeOpen: 0.6, mouthCurve: 1, mouthOpen: 0.3, bounce: 0, shake: 0, browAngle: -0.6, browY: 0.5 },
  taskcheck: { eyeX: 0, eyeY: 0, eyeOpen: 0.75, mouthCurve: 0.3, mouthOpen: 0, bounce: 0, shake: 0, browAngle: -0.3, browY: 0.3 },
  error:     { eyeX: 0, eyeY: 0, eyeOpen: 1.2, mouthCurve: -1, mouthOpen: 0.4, bounce: 0, shake: 2, browAngle: -1, browY: -2 },
};

const STATES: AvatarState[] = ['idle', 'thinking', 'waiting', 'success', 'taskcheck', 'error'];

export default function AvatarEditorPage() {
  const qc = useQueryClient();
  const [searchParams] = useSearchParams();
  const [params, setParams] = useState<AvatarParams>(DEFAULT_PARAMS);
  const [accessories, setAccessories] = useState<Accessories>(DEFAULT_ACCESSORIES);
  const [activeState, setActiveState] = useState<AvatarState>('idle');
  const [animating, setAnimating] = useState(false);
  const [selectedDevice, setSelectedDevice] = useState(searchParams.get('device') || '');
  const [dragging, setDragging] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<number>(0);
  const timeRef = useRef(0);

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });

  const { data: deviceConfig } = useQuery({
    queryKey: ['config', selectedDevice],
    queryFn: () => getDeviceConfig(selectedDevice),
    enabled: !!selectedDevice,
  });

  const { data: ownedItems } = useQuery({
    queryKey: ['store-owned', selectedDevice],
    queryFn: () => getOwnedItems(selectedDevice || undefined),
    enabled: !!selectedDevice,
  });

  // Map store accessory IDs to accessory keys
  const ACC_STORE_MAP: Record<string, keyof Accessories> = {
    acc_tophat: 'topHat',
    acc_glasses: 'glasses',
    acc_bowtie: 'bowtie',
    acc_cigar: 'cigar',
    acc_horns: 'horns',
    acc_monocle: 'monocle',
    acc_crown: 'crown',
    acc_halo: 'halo',
  };

  function isAccessoryOwned(key: keyof Accessories): boolean {
    if (!selectedDevice || !ownedItems) return true; // No device selected = show all
    if (key === 'topHat') return true; // Top hat is free (starter item)
    const storeId = Object.entries(ACC_STORE_MAP).find(([, v]) => v === key)?.[0];
    return storeId ? ownedItems.includes(storeId) : true;
  }

  // Load preset from device
  useEffect(() => {
    if (deviceConfig?.avatar_preset) {
      const p = deviceConfig.avatar_preset as Record<string, unknown>;
      if (p.params) setParams(p.params as AvatarParams);
      if (p.accessories) setAccessories(prev => ({ ...prev, ...(p.accessories as Partial<Accessories>) }));
    }
  }, [deviceConfig]);

  const saveMut = useMutation({
    mutationFn: () => updateDeviceConfig(selectedDevice, {
      avatar_preset: { params, accessories, state: activeState },
    }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['config', selectedDevice] }),
  });

  const pushMut = useMutation({
    mutationFn: async () => {
      await updateDeviceConfig(selectedDevice, {
        avatar_preset: { params, accessories, state: activeState },
      });
      await pushConfig(selectedDevice);
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['config', selectedDevice] }),
  });

  function loadPreset(state: AvatarState) {
    setActiveState(state);
    setParams(STATE_PRESETS[state]);
  }

  function updateParam(key: keyof AvatarParams, value: number) {
    setParams(p => ({ ...p, [key]: value }));
  }

  function toggleAccessory(key: keyof Accessories) {
    setAccessories(a => ({ ...a, [key]: !a[key] }));
  }

  // Eye drag handler
  const handleCanvasInteraction = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!dragging && e.type !== 'mousedown') return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;
    // Map to eye range (-1 to 1)
    const eyeX = Math.max(-1, Math.min(1, (x - 0.5) * 3));
    const eyeY = Math.max(-1, Math.min(1, (y - 0.5) * 3));
    setParams(p => ({ ...p, eyeX, eyeY }));
  }, [dragging]);

  // Draw loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const draw = () => {
      if (animating) {
        timeRef.current += 33;
        // Add breathing animation
        const phase = timeRef.current / 1000;
        const breath = Math.sin(phase * 0.5) * 1.2;
        const glance = Math.sin(phase * 0.3);
        const animParams = { ...params };
        animParams.bounce = params.bounce + breath;
        if (glance > 0.85) {
          animParams.eyeX = Math.sin(phase * 1.5) * 0.5;
        }
        // Blink
        if (Math.floor(phase * 0.25) % 4 === 0 && (phase * 0.25 % 1) < 0.05) {
          animParams.eyeOpen = 0;
        }
        drawAvatar(ctx, canvas.width, canvas.height, animParams, accessories, timeRef.current);
      } else {
        drawAvatar(ctx, canvas.width, canvas.height, params, accessories, Date.now());
      }
      animRef.current = requestAnimationFrame(draw);
    };
    draw();
    return () => cancelAnimationFrame(animRef.current);
  }, [params, accessories, animating]);

  const ACCESSORY_LIST: { key: keyof Accessories; label: string; emoji: string }[] = [
    { key: 'topHat', label: 'Top Hat', emoji: '\u{1F3A9}' },
    { key: 'crown', label: 'Crown', emoji: '\u{1F451}' },
    { key: 'horns', label: 'Devil Horns', emoji: '\u{1F608}' },
    { key: 'halo', label: 'Halo', emoji: '\u{1F607}' },
    { key: 'cigar', label: 'Cigar', emoji: '\u{1F6AC}' },
    { key: 'glasses', label: 'Glasses', emoji: '\u{1F453}' },
    { key: 'monocle', label: 'Monocle', emoji: '\u{1F9D0}' },
    { key: 'bowtie', label: 'Bow Tie', emoji: '\u{1F380}' },
  ];

  const SLIDER_GROUPS = [
    {
      label: 'Eyes',
      items: [
        { key: 'eyeX' as const, label: 'Look X', min: -1, max: 1, step: 0.05 },
        { key: 'eyeY' as const, label: 'Look Y', min: -1, max: 1, step: 0.05 },
        { key: 'eyeOpen' as const, label: 'Openness', min: 0, max: 1.2, step: 0.05 },
      ],
    },
    {
      label: 'Brows',
      items: [
        { key: 'browAngle' as const, label: 'Angle', min: -1, max: 1, step: 0.05 },
        { key: 'browY' as const, label: 'Height', min: -2, max: 1, step: 0.1 },
      ],
    },
    {
      label: 'Mouth',
      items: [
        { key: 'mouthCurve' as const, label: 'Curve', min: -1, max: 1, step: 0.05 },
        { key: 'mouthOpen' as const, label: 'Open', min: 0, max: 0.5, step: 0.05 },
      ],
    },
    {
      label: 'Body',
      items: [
        { key: 'bounce' as const, label: 'Bounce', min: -5, max: 5, step: 0.5 },
        { key: 'shake' as const, label: 'Shake', min: -5, max: 5, step: 0.5 },
      ],
    },
  ];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-fg">Avatar Editor</h1>
        <div className="flex items-center gap-2">
          <select
            value={selectedDevice}
            onChange={e => setSelectedDevice(e.target.value)}
            className="px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg"
          >
            <option value="">Select device...</option>
            {devices?.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
          <button
            onClick={() => saveMut.mutate()}
            disabled={!selectedDevice || saveMut.isPending}
            className="px-4 py-1.5 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
          >
            {saveMut.isPending ? 'Saving...' : 'Save'}
          </button>
          <button
            onClick={() => pushMut.mutate()}
            disabled={!selectedDevice || pushMut.isPending}
            className="px-4 py-1.5 text-sm bg-green-600 hover:bg-green-700 rounded-md disabled:opacity-50"
          >
            {pushMut.isPending ? 'Pushing...' : 'Save & Push'}
          </button>
          {saveMut.isSuccess && <span className="text-xs text-green-400">Saved!</span>}
          {pushMut.isSuccess && <span className="text-xs text-green-400">Pushed!</span>}
          {pushMut.isError && <span className="text-xs text-red-400">{pushMut.error.message}</span>}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] gap-6">
        {/* Left: Preview + Accessories */}
        <div className="space-y-4">
          {/* Canvas */}
          <div className="rounded-xl border border-edge bg-black p-6 flex flex-col items-center">
            <div className="flex items-center justify-between w-full mb-3">
              <span className="text-[10px] text-dim font-mono uppercase tracking-wider">128 x 64 OLED Preview (2x)</span>
              <button
                onClick={() => { setAnimating(!animating); timeRef.current = 0; }}
                className={`px-3 py-1 text-xs rounded-full transition-colors ${
                  animating
                    ? 'bg-red-500/20 text-red-400 border border-red-500/30'
                    : 'bg-inset text-muted border border-edge hover:border-edge'
                }`}
              >
                {animating ? 'Stop' : 'Animate'}
              </button>
            </div>
            <canvas
              ref={canvasRef}
              width={384}
              height={192}
              className="rounded-lg border border-edge cursor-crosshair"
              style={{ imageRendering: 'pixelated', width: '100%', maxWidth: 512 }}
              onMouseDown={e => { setDragging(true); handleCanvasInteraction(e); }}
              onMouseMove={handleCanvasInteraction}
              onMouseUp={() => setDragging(false)}
              onMouseLeave={() => setDragging(false)}
            />
            <p className="text-[10px] text-dim mt-2">Click and drag to move eyes</p>
          </div>

          {/* State Presets */}
          <div className="rounded-lg border border-edge bg-surface p-4">
            <p className="text-xs text-subtle mb-3 font-medium uppercase tracking-wider">State Presets</p>
            <div className="grid grid-cols-3 sm:grid-cols-6 gap-2">
              {STATES.map(s => (
                <button
                  key={s}
                  onClick={() => loadPreset(s)}
                  className={`px-3 py-2.5 text-xs rounded-lg capitalize transition-all ${
                    activeState === s
                      ? 'bg-red-500/20 text-red-400 border border-red-500/30 shadow-lg shadow-red-500/10'
                      : 'bg-inset/80 text-muted border border-edge hover:border-edge hover:text-fg'
                  }`}
                >
                  {s}
                </button>
              ))}
            </div>
          </div>

          {/* Accessories */}
          <div className="rounded-lg border border-edge bg-surface p-4">
            <p className="text-xs text-subtle mb-3 font-medium uppercase tracking-wider">Accessories</p>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
              {ACCESSORY_LIST.map(a => {
                const owned = isAccessoryOwned(a.key);
                return (
                  <button
                    key={a.key}
                    onClick={() => owned && toggleAccessory(a.key)}
                    disabled={!owned}
                    className={`flex items-center gap-2 px-3 py-2.5 rounded-lg text-sm transition-all ${
                      !owned
                        ? 'bg-inset/40 text-dim border border-edge opacity-50 cursor-not-allowed'
                        : accessories[a.key]
                          ? 'bg-amber-500/15 text-amber-300 border border-amber-500/30'
                          : 'bg-inset/80 text-subtle border border-edge hover:border-edge'
                    }`}
                    title={owned ? a.label : `Buy from Store to unlock`}
                  >
                    <span className="text-lg">{owned ? a.emoji : '\u{1F512}'}</span>
                    <span className="text-xs">{a.label}</span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Export JSON */}
          <details className="rounded-lg border border-edge bg-surface">
            <summary className="px-4 py-3 text-xs text-subtle cursor-pointer hover:text-fg-2 font-medium uppercase tracking-wider">
              Export JSON
            </summary>
            <pre className="px-4 pb-4 text-[11px] text-muted font-mono overflow-x-auto">
{JSON.stringify({ params, accessories, state: activeState }, null, 2)}
            </pre>
          </details>
        </div>

        {/* Right: Sliders */}
        <div className="space-y-4">
          {SLIDER_GROUPS.map(group => (
            <div key={group.label} className="rounded-lg border border-edge bg-surface p-4 space-y-3">
              <p className="text-xs text-subtle font-medium uppercase tracking-wider">{group.label}</p>
              {group.items.map(s => (
                <div key={s.key}>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-xs text-muted">{s.label}</label>
                    <span className="text-[11px] font-mono text-dim tabular-nums">
                      {params[s.key].toFixed(2)}
                    </span>
                  </div>
                  <input
                    type="range"
                    min={s.min} max={s.max} step={s.step}
                    value={params[s.key]}
                    onChange={e => updateParam(s.key, parseFloat(e.target.value))}
                    className="w-full h-1 bg-raised rounded-full appearance-none cursor-pointer
                      [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3
                      [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-red-500 [&::-webkit-slider-thumb]:cursor-pointer
                      [&::-webkit-slider-thumb]:shadow-[0_0_6px_rgba(239,68,68,0.4)]"
                  />
                </div>
              ))}
            </div>
          ))}

          <button
            onClick={() => { setParams(DEFAULT_PARAMS); setActiveState('idle'); }}
            className="w-full px-3 py-2.5 text-xs bg-inset hover:bg-raised rounded-lg text-muted transition-colors border border-edge"
          >
            Reset All
          </button>
        </div>
      </div>
    </div>
  );
}

// ─── Canvas Drawing ─────────────────────────────────────────────

function drawAvatar(
  ctx: CanvasRenderingContext2D, w: number, h: number,
  p: AvatarParams, acc: Accessories, time: number
) {
  const S = 3; // scale factor (384/128)
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, w, h);

  const cx = w / 2 + p.shake * S;
  const cy = h / 2 + p.bounce * S + 8 * S; // offset down for hat room

  // ─── Accessories: Head items ─────
  if (acc.topHat) drawTopHat(ctx, cx, cy, S, time);
  if (acc.crown) drawCrown(ctx, cx, cy, S, time);
  if (acc.horns) drawHorns(ctx, cx, cy, S, time);
  if (acc.halo) drawHalo(ctx, cx, cy, S, time);

  // ─── Eyebrows ─────
  const eyeSpacing = 18 * S;
  const browBaseY = cy - 18 * S;

  ctx.strokeStyle = '#fff';
  ctx.lineCap = 'round';
  for (const side of [-1, 1]) {
    const bx = cx + side * eyeSpacing;
    const by = browBaseY + p.browY * 4 * S;
    const tilt = p.browAngle * 3 * S;
    const innerX = bx - side * 2 * S;
    const outerX = bx + side * 8 * S;

    ctx.lineWidth = 2.5 * S;
    ctx.beginPath();
    ctx.moveTo(innerX, by - tilt);
    ctx.lineTo(outerX, by + tilt);
    ctx.stroke();
  }

  // ─── Eyes ─────
  const eyeBaseY = cy - 8 * S;
  const eyeW = 10 * S;
  const eyeMaxH = 12 * S;
  const openness = Math.max(0, Math.min(p.eyeOpen, 1.2));
  const eyeH = Math.max(1, eyeMaxH * openness);
  const pupilOffX = p.eyeX * 3 * S;
  const pupilOffY = p.eyeY * 2 * S;

  for (const side of [-1, 1]) {
    const ex = cx + side * eyeSpacing;

    if (eyeH <= 2 * S) {
      // Closed
      ctx.strokeStyle = '#fff';
      ctx.lineWidth = S;
      ctx.beginPath();
      ctx.moveTo(ex - eyeW / 2, eyeBaseY);
      ctx.lineTo(ex + eyeW / 2, eyeBaseY);
      ctx.stroke();
    } else {
      // Open eye
      ctx.fillStyle = '#fff';
      const r = Math.min(eyeW / 2, eyeH / 2);
      roundRect(ctx, ex - eyeW / 2, eyeBaseY - eyeH / 2, eyeW, eyeH, r);
      ctx.fill();

      // Pupil
      if (eyeH > 5 * S) {
        ctx.fillStyle = '#000';
        ctx.beginPath();
        ctx.arc(ex + pupilOffX, eyeBaseY + pupilOffY, 2.5 * S, 0, Math.PI * 2);
        ctx.fill();

        // Pupil highlight
        ctx.fillStyle = '#fff';
        ctx.beginPath();
        ctx.arc(ex + pupilOffX - S, eyeBaseY + pupilOffY - S, 0.8 * S, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }

  // ─── Accessories: Eye items ─────
  if (acc.glasses) drawGlasses(ctx, cx, eyeBaseY, eyeSpacing, S);
  if (acc.monocle) drawMonocle(ctx, cx, eyeBaseY, eyeSpacing, S);

  // ─── Mouth ─────
  const mouthY = cy + 12 * S;
  const mouthW = 16 * S;
  const curve = p.mouthCurve * 6 * S;
  const openH = p.mouthOpen * 6 * S;

  ctx.strokeStyle = '#fff';
  ctx.fillStyle = '#fff';
  ctx.lineWidth = 1.5 * S;

  if (openH > S) {
    // Open mouth
    ctx.beginPath();
    ctx.moveTo(cx - mouthW / 2, mouthY);
    ctx.quadraticCurveTo(cx, mouthY + curve, cx + mouthW / 2, mouthY);
    ctx.quadraticCurveTo(cx, mouthY + curve + openH, cx - mouthW / 2, mouthY);
    ctx.fill();
  } else {
    // Closed mouth line
    ctx.beginPath();
    ctx.moveTo(cx - mouthW / 2, mouthY);
    ctx.quadraticCurveTo(cx, mouthY + curve, cx + mouthW / 2, mouthY);
    ctx.stroke();
  }

  // ─── Accessories: Mouth/body items ─────
  if (acc.cigar) drawCigar(ctx, cx, mouthY, mouthW, S, time);
  if (acc.bowtie) drawBowtie(ctx, cx, cy, S);
}

// ─── Accessory Drawers ──────────────────────────────────────────

function drawTopHat(ctx: CanvasRenderingContext2D, cx: number, cy: number, S: number, _time: number) {
  const brimY = cy - 22 * S;
  ctx.fillStyle = '#fff';
  // Brim
  roundRect(ctx, cx - 20 * S, brimY, 40 * S, 3 * S, S);
  ctx.fill();
  // Body
  ctx.fillRect(cx - 14 * S, brimY - 20 * S, 28 * S, 20 * S);
  // Top rounded
  roundRect(ctx, cx - 14 * S, brimY - 22 * S, 28 * S, 4 * S, 2 * S);
  ctx.fill();
  // Band
  ctx.fillStyle = '#000';
  ctx.fillRect(cx - 14 * S, brimY - 5 * S, 28 * S, 3 * S);
  // Band detail
  ctx.fillStyle = '#888';
  ctx.fillRect(cx - 2 * S, brimY - 5 * S, 4 * S, 3 * S);
}

function drawCrown(ctx: CanvasRenderingContext2D, cx: number, cy: number, S: number, time: number) {
  const baseY = cy - 24 * S;
  ctx.fillStyle = '#FFD700';
  ctx.strokeStyle = '#FFD700';
  ctx.lineWidth = 1.5 * S;

  // Crown base
  ctx.beginPath();
  ctx.moveTo(cx - 16 * S, baseY);
  ctx.lineTo(cx - 16 * S, baseY - 10 * S);
  ctx.lineTo(cx - 10 * S, baseY - 5 * S);
  ctx.lineTo(cx - 4 * S, baseY - 14 * S);
  ctx.lineTo(cx, baseY - 8 * S);
  ctx.lineTo(cx + 4 * S, baseY - 14 * S);
  ctx.lineTo(cx + 10 * S, baseY - 5 * S);
  ctx.lineTo(cx + 16 * S, baseY - 10 * S);
  ctx.lineTo(cx + 16 * S, baseY);
  ctx.closePath();
  ctx.fill();

  // Jewels
  const sparkle = Math.sin(time / 300) * 0.5 + 0.5;
  ctx.fillStyle = `rgba(255, 50, 50, ${0.7 + sparkle * 0.3})`;
  ctx.beginPath(); ctx.arc(cx, baseY - 8 * S, 1.5 * S, 0, Math.PI * 2); ctx.fill();
  ctx.fillStyle = `rgba(50, 50, 255, ${0.7 + sparkle * 0.3})`;
  ctx.beginPath(); ctx.arc(cx - 10 * S, baseY - 5 * S, S, 0, Math.PI * 2); ctx.fill();
  ctx.beginPath(); ctx.arc(cx + 10 * S, baseY - 5 * S, S, 0, Math.PI * 2); ctx.fill();
}

function drawHorns(ctx: CanvasRenderingContext2D, cx: number, cy: number, S: number, _time: number) {
  const baseY = cy - 22 * S;
  ctx.fillStyle = '#ff3333';
  for (const side of [-1, 1]) {
    ctx.beginPath();
    ctx.moveTo(cx + side * 12 * S, baseY);
    ctx.quadraticCurveTo(cx + side * 18 * S, baseY - 8 * S, cx + side * 14 * S, baseY - 16 * S);
    ctx.quadraticCurveTo(cx + side * 16 * S, baseY - 6 * S, cx + side * 10 * S, baseY);
    ctx.fill();
  }
}

function drawHalo(ctx: CanvasRenderingContext2D, cx: number, cy: number, S: number, time: number) {
  const y = cy - 30 * S + Math.sin(time / 500) * S;
  ctx.strokeStyle = '#FFD700';
  ctx.lineWidth = 2 * S;
  ctx.beginPath();
  ctx.ellipse(cx, y, 14 * S, 4 * S, 0, 0, Math.PI * 2);
  ctx.stroke();
  // Glow
  ctx.strokeStyle = 'rgba(255, 215, 0, 0.3)';
  ctx.lineWidth = 4 * S;
  ctx.beginPath();
  ctx.ellipse(cx, y, 14 * S, 4 * S, 0, 0, Math.PI * 2);
  ctx.stroke();
}

function drawGlasses(ctx: CanvasRenderingContext2D, cx: number, eyeY: number, spacing: number, S: number) {
  ctx.strokeStyle = '#fff';
  ctx.lineWidth = 1.5 * S;
  // Left lens
  ctx.beginPath();
  ctx.ellipse(cx - spacing, eyeY, 8 * S, 7 * S, 0, 0, Math.PI * 2);
  ctx.stroke();
  // Right lens
  ctx.beginPath();
  ctx.ellipse(cx + spacing, eyeY, 8 * S, 7 * S, 0, 0, Math.PI * 2);
  ctx.stroke();
  // Bridge
  ctx.beginPath();
  ctx.moveTo(cx - spacing + 8 * S, eyeY);
  ctx.lineTo(cx + spacing - 8 * S, eyeY);
  ctx.stroke();
  // Arms
  ctx.beginPath();
  ctx.moveTo(cx - spacing - 8 * S, eyeY);
  ctx.lineTo(cx - spacing - 12 * S, eyeY - 2 * S);
  ctx.moveTo(cx + spacing + 8 * S, eyeY);
  ctx.lineTo(cx + spacing + 12 * S, eyeY - 2 * S);
  ctx.stroke();
}

function drawMonocle(ctx: CanvasRenderingContext2D, cx: number, eyeY: number, spacing: number, S: number) {
  ctx.strokeStyle = '#FFD700';
  ctx.lineWidth = 1.5 * S;
  // Lens
  ctx.beginPath();
  ctx.arc(cx + spacing, eyeY, 8 * S, 0, Math.PI * 2);
  ctx.stroke();
  // Chain
  ctx.strokeStyle = 'rgba(255, 215, 0, 0.5)';
  ctx.lineWidth = S;
  ctx.beginPath();
  ctx.moveTo(cx + spacing + 5.5 * S, eyeY + 5.5 * S);
  ctx.quadraticCurveTo(cx + spacing + 12 * S, eyeY + 20 * S, cx + spacing - 5 * S, eyeY + 30 * S);
  ctx.stroke();
}

function drawCigar(ctx: CanvasRenderingContext2D, cx: number, mouthY: number, mouthW: number, S: number, time: number) {
  const cigarX = cx + mouthW / 2 + S;
  const cigarY = mouthY + S;

  // Body
  ctx.fillStyle = '#8B4513';
  ctx.beginPath();
  ctx.moveTo(cigarX, cigarY - S);
  ctx.lineTo(cigarX + 12 * S, cigarY - 4 * S);
  ctx.lineTo(cigarX + 12 * S, cigarY - S);
  ctx.lineTo(cigarX, cigarY + 2 * S);
  ctx.closePath();
  ctx.fill();

  // Ash tip
  ctx.fillStyle = '#aaa';
  ctx.fillRect(cigarX + 10 * S, cigarY - 4 * S, 2 * S, 3 * S);

  // Ember
  const flicker = Math.sin(time / 150);
  ctx.fillStyle = flicker > -0.3 ? '#ff4400' : '#aa2200';
  ctx.beginPath();
  ctx.arc(cigarX + 12 * S, cigarY - 2.5 * S, 1.5 * S, 0, Math.PI * 2);
  ctx.fill();

  // Smoke
  ctx.fillStyle = 'rgba(255,255,255,0.15)';
  const smokePhase = time / 1000;
  for (let i = 0; i < 6; i++) {
    const pLife = ((smokePhase * 1.2 + i * 0.7) % 3);
    if (pLife > 2.5) continue;
    const rise = pLife * 6 * S;
    const drift = Math.sin(pLife * 2 + i * 1.5) * (2 + pLife) * S;
    const size = (1 + pLife * 0.8) * S;
    const alpha = Math.max(0, 0.3 - pLife * 0.1);
    ctx.fillStyle = `rgba(255,255,255,${alpha})`;
    ctx.beginPath();
    ctx.arc(cigarX + 12 * S + drift, cigarY - 5 * S - rise, size, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawBowtie(ctx: CanvasRenderingContext2D, cx: number, cy: number, S: number) {
  const y = cy + 22 * S;
  ctx.fillStyle = '#ff3333';
  // Left triangle
  ctx.beginPath();
  ctx.moveTo(cx, y);
  ctx.lineTo(cx - 10 * S, y - 5 * S);
  ctx.lineTo(cx - 10 * S, y + 5 * S);
  ctx.closePath();
  ctx.fill();
  // Right triangle
  ctx.beginPath();
  ctx.moveTo(cx, y);
  ctx.lineTo(cx + 10 * S, y - 5 * S);
  ctx.lineTo(cx + 10 * S, y + 5 * S);
  ctx.closePath();
  ctx.fill();
  // Center knot
  ctx.fillStyle = '#cc0000';
  ctx.beginPath();
  ctx.arc(cx, y, 2 * S, 0, Math.PI * 2);
  ctx.fill();
}

// ─── Helpers ────────────────────────────────────────────────────

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.arcTo(x + w, y, x + w, y + r, r);
  ctx.lineTo(x + w, y + h - r);
  ctx.arcTo(x + w, y + h, x + w - r, y + h, r);
  ctx.lineTo(x + r, y + h);
  ctx.arcTo(x, y + h, x, y + h - r, r);
  ctx.lineTo(x, y + r);
  ctx.arcTo(x, y, x + r, y, r);
  ctx.closePath();
}
