import { useState, useRef, useEffect, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getDevices, updateDeviceConfig, getDeviceConfig, pushConfig } from '../api/client';

// ─── Types ──────────────────────────────────────────────────────

interface AvatarParams {
  eyeX: number; eyeY: number; eyeOpen: number;
  mouthCurve: number; mouthOpen: number;
  bounce: number; shake: number;
  browAngle: number; browY: number;
}

interface Keyframe {
  time: number; // ms
  params: AvatarParams;
  easing: 'linear' | 'ease-in' | 'ease-out' | 'ease-in-out' | 'bounce';
}

interface Animation {
  name: string;
  duration: number; // ms
  loop: boolean;
  keyframes: Keyframe[];
}

const DEFAULT_PARAMS: AvatarParams = {
  eyeX: 0, eyeY: 0, eyeOpen: 0.9,
  mouthCurve: 0.15, mouthOpen: 0,
  bounce: 0, shake: 0,
  browAngle: -0.4, browY: 0,
};

const PARAM_KEYS: (keyof AvatarParams)[] = [
  'eyeX', 'eyeY', 'eyeOpen', 'browAngle', 'browY', 'mouthCurve', 'mouthOpen', 'bounce', 'shake',
];

const PARAM_LABELS: Record<keyof AvatarParams, string> = {
  eyeX: 'Eye X', eyeY: 'Eye Y', eyeOpen: 'Eye Open',
  browAngle: 'Brow', browY: 'Brow Y',
  mouthCurve: 'Mouth', mouthOpen: 'Mouth Open',
  bounce: 'Bounce', shake: 'Shake',
};

const PARAM_RANGES: Record<keyof AvatarParams, [number, number]> = {
  eyeX: [-1, 1], eyeY: [-1, 1], eyeOpen: [0, 1.2],
  browAngle: [-1, 1], browY: [-2, 1],
  mouthCurve: [-1, 1], mouthOpen: [0, 0.5],
  bounce: [-5, 5], shake: [-5, 5],
};

const PRESET_ANIMATIONS: Animation[] = [
  {
    name: 'Nod',
    duration: 800,
    loop: false,
    keyframes: [
      { time: 0, params: { ...DEFAULT_PARAMS }, easing: 'ease-out' },
      { time: 300, params: { ...DEFAULT_PARAMS, bounce: 3, eyeOpen: 0.7 }, easing: 'ease-in-out' },
      { time: 600, params: { ...DEFAULT_PARAMS, bounce: -1 }, easing: 'ease-out' },
      { time: 800, params: { ...DEFAULT_PARAMS }, easing: 'linear' },
    ],
  },
  {
    name: 'Head Shake',
    duration: 1000,
    loop: false,
    keyframes: [
      { time: 0, params: { ...DEFAULT_PARAMS }, easing: 'ease-in-out' },
      { time: 200, params: { ...DEFAULT_PARAMS, shake: 3, browAngle: -0.6 }, easing: 'ease-in-out' },
      { time: 400, params: { ...DEFAULT_PARAMS, shake: -3, browAngle: -0.6 }, easing: 'ease-in-out' },
      { time: 600, params: { ...DEFAULT_PARAMS, shake: 2 }, easing: 'ease-in-out' },
      { time: 800, params: { ...DEFAULT_PARAMS, shake: -1 }, easing: 'ease-out' },
      { time: 1000, params: { ...DEFAULT_PARAMS }, easing: 'linear' },
    ],
  },
  {
    name: 'Laugh',
    duration: 2000,
    loop: false,
    keyframes: [
      { time: 0, params: { ...DEFAULT_PARAMS }, easing: 'ease-in' },
      { time: 200, params: { ...DEFAULT_PARAMS, eyeOpen: 0.5, mouthCurve: 0.8, mouthOpen: 0.3, browAngle: -0.3, browY: 0.3 }, easing: 'ease-in-out' },
      { time: 500, params: { ...DEFAULT_PARAMS, eyeOpen: 0.4, mouthCurve: 1, mouthOpen: 0.4, bounce: -2, browAngle: -0.2, browY: 0.5 }, easing: 'bounce' },
      { time: 800, params: { ...DEFAULT_PARAMS, eyeOpen: 0.5, mouthCurve: 0.9, mouthOpen: 0.3, bounce: 2 }, easing: 'bounce' },
      { time: 1100, params: { ...DEFAULT_PARAMS, eyeOpen: 0.4, mouthCurve: 1, mouthOpen: 0.4, bounce: -2 }, easing: 'bounce' },
      { time: 1500, params: { ...DEFAULT_PARAMS, eyeOpen: 0.6, mouthCurve: 0.5, mouthOpen: 0.1 }, easing: 'ease-out' },
      { time: 2000, params: { ...DEFAULT_PARAMS }, easing: 'linear' },
    ],
  },
  {
    name: 'Rage',
    duration: 2500,
    loop: false,
    keyframes: [
      { time: 0, params: { ...DEFAULT_PARAMS }, easing: 'ease-in' },
      { time: 300, params: { ...DEFAULT_PARAMS, eyeOpen: 1.2, browAngle: -1, browY: -2, mouthCurve: -0.8, mouthOpen: 0.3 }, easing: 'ease-in-out' },
      { time: 600, params: { ...DEFAULT_PARAMS, eyeOpen: 1.2, browAngle: -1, browY: -2, mouthCurve: -1, mouthOpen: 0.5, shake: 4 }, easing: 'bounce' },
      { time: 900, params: { ...DEFAULT_PARAMS, eyeOpen: 1.2, browAngle: -1, browY: -2, mouthCurve: -1, mouthOpen: 0.5, shake: -4 }, easing: 'bounce' },
      { time: 1200, params: { ...DEFAULT_PARAMS, eyeOpen: 1.2, browAngle: -1, browY: -2, mouthCurve: -1, mouthOpen: 0.5, shake: 3, bounce: -3 }, easing: 'bounce' },
      { time: 1700, params: { ...DEFAULT_PARAMS, eyeOpen: 0.6, browAngle: -0.8, mouthCurve: -0.9, shake: 0.5 }, easing: 'ease-out' },
      { time: 2500, params: { ...DEFAULT_PARAMS }, easing: 'linear' },
    ],
  },
  {
    name: 'Blink',
    duration: 300,
    loop: false,
    keyframes: [
      { time: 0, params: { ...DEFAULT_PARAMS }, easing: 'ease-in' },
      { time: 80, params: { ...DEFAULT_PARAMS, eyeOpen: 0 }, easing: 'ease-out' },
      { time: 160, params: { ...DEFAULT_PARAMS, eyeOpen: 0 }, easing: 'ease-in' },
      { time: 300, params: { ...DEFAULT_PARAMS }, easing: 'linear' },
    ],
  },
  {
    name: 'Look Around',
    duration: 3000,
    loop: true,
    keyframes: [
      { time: 0, params: { ...DEFAULT_PARAMS, eyeX: 0, eyeY: 0 }, easing: 'ease-in-out' },
      { time: 500, params: { ...DEFAULT_PARAMS, eyeX: 0.8, eyeY: -0.2 }, easing: 'ease-in-out' },
      { time: 1000, params: { ...DEFAULT_PARAMS, eyeX: 0.8, eyeY: 0.3 }, easing: 'ease-in-out' },
      { time: 1500, params: { ...DEFAULT_PARAMS, eyeX: -0.7, eyeY: -0.1 }, easing: 'ease-in-out' },
      { time: 2200, params: { ...DEFAULT_PARAMS, eyeX: -0.5, eyeY: 0.2 }, easing: 'ease-in-out' },
      { time: 3000, params: { ...DEFAULT_PARAMS, eyeX: 0, eyeY: 0 }, easing: 'ease-in-out' },
    ],
  },
  {
    name: 'Sleep',
    duration: 4000,
    loop: true,
    keyframes: [
      { time: 0, params: { ...DEFAULT_PARAMS, eyeOpen: 0.1, mouthCurve: 0, browAngle: -0.1, browY: 0.5, bounce: 0 }, easing: 'ease-in-out' },
      { time: 2000, params: { ...DEFAULT_PARAMS, eyeOpen: 0, mouthCurve: 0, mouthOpen: 0.1, browAngle: -0.1, browY: 0.5, bounce: 2 }, easing: 'ease-in-out' },
      { time: 4000, params: { ...DEFAULT_PARAMS, eyeOpen: 0.1, mouthCurve: 0, browAngle: -0.1, browY: 0.5, bounce: 0 }, easing: 'ease-in-out' },
    ],
  },
];

// ─── Component ──────────────────────────────────────────────────

export default function AnimationEditorPage() {
  const qc = useQueryClient();
  const [animation, setAnimation] = useState<Animation>(PRESET_ANIMATIONS[0]);
  const [selectedKf, setSelectedKf] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [playTime, setPlayTime] = useState(0);
  const [selectedDevice, setSelectedDevice] = useState('');
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const playRef = useRef<number>(0);
  const lastFrameRef = useRef(0);

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });

  // ─── Playback ─────────────────────────────────────────────────

  useEffect(() => {
    if (!playing) return;
    lastFrameRef.current = performance.now();

    const tick = (now: number) => {
      const dt = now - lastFrameRef.current;
      lastFrameRef.current = now;

      setPlayTime(prev => {
        let next = prev + dt;
        if (next >= animation.duration) {
          if (animation.loop) {
            next = next % animation.duration;
          } else {
            setPlaying(false);
            return animation.duration;
          }
        }
        return next;
      });

      playRef.current = requestAnimationFrame(tick);
    };

    playRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(playRef.current);
  }, [playing, animation.duration, animation.loop]);

  // ─── Canvas Draw ──────────────────────────────────────────────

  const currentParams = playing
    ? interpolateAtTime(animation.keyframes, playTime)
    : animation.keyframes[selectedKf]?.params ?? DEFAULT_PARAMS;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    drawPreview(ctx, canvas.width, canvas.height, currentParams);
  }, [currentParams]);

  // ─── Keyframe Editing ─────────────────────────────────────────

  function updateKfParam(key: keyof AvatarParams, value: number) {
    setAnimation(prev => {
      const kfs = [...prev.keyframes];
      kfs[selectedKf] = { ...kfs[selectedKf], params: { ...kfs[selectedKf].params, [key]: value } };
      return { ...prev, keyframes: kfs };
    });
  }

  function updateKfTime(time: number) {
    setAnimation(prev => {
      const kfs = [...prev.keyframes];
      kfs[selectedKf] = { ...kfs[selectedKf], time };
      kfs.sort((a, b) => a.time - b.time);
      return { ...prev, keyframes: kfs };
    });
  }

  function updateKfEasing(easing: Keyframe['easing']) {
    setAnimation(prev => {
      const kfs = [...prev.keyframes];
      kfs[selectedKf] = { ...kfs[selectedKf], easing };
      return { ...prev, keyframes: kfs };
    });
  }

  function addKeyframe() {
    const time = Math.min(animation.duration, playTime || animation.duration / 2);
    const params = interpolateAtTime(animation.keyframes, time);
    const kf: Keyframe = { time: Math.round(time), params, easing: 'ease-in-out' };
    setAnimation(prev => {
      const kfs = [...prev.keyframes, kf].sort((a, b) => a.time - b.time);
      return { ...prev, keyframes: kfs };
    });
    setSelectedKf(animation.keyframes.length);
  }

  function deleteKeyframe() {
    if (animation.keyframes.length <= 2) return;
    setAnimation(prev => {
      const kfs = prev.keyframes.filter((_, i) => i !== selectedKf);
      return { ...prev, keyframes: kfs };
    });
    setSelectedKf(Math.max(0, selectedKf - 1));
  }

  function duplicateKeyframe() {
    const src = animation.keyframes[selectedKf];
    const newTime = Math.min(animation.duration, src.time + 200);
    const kf: Keyframe = { time: newTime, params: { ...src.params }, easing: src.easing };
    setAnimation(prev => {
      const kfs = [...prev.keyframes, kf].sort((a, b) => a.time - b.time);
      return { ...prev, keyframes: kfs };
    });
  }

  // ─── Save ─────────────────────────────────────────────────────

  const saveMut = useMutation({
    mutationFn: async () => {
      const cfg = await getDeviceConfig(selectedDevice);
      const existing = (cfg.avatar_preset as Record<string, unknown>) || {};
      const animations = ((existing.animations as Animation[]) || []).filter(a => a.name !== animation.name);
      animations.push(animation);
      await updateDeviceConfig(selectedDevice, {
        avatar_preset: { ...existing, animations },
      });
      await pushConfig(selectedDevice);
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: ['config', selectedDevice] }),
  });

  // ─── Timeline click ──────────────────────────────────────────

  const timelineRef = useRef<HTMLDivElement>(null);
  const handleTimelineClick = useCallback((e: React.MouseEvent) => {
    const el = timelineRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const time = x * animation.duration;
    setPlayTime(time);
    // Select nearest keyframe
    let nearest = 0;
    let nearestDist = Infinity;
    animation.keyframes.forEach((kf, i) => {
      const dist = Math.abs(kf.time - time);
      if (dist < nearestDist) { nearestDist = dist; nearest = i; }
    });
    if (nearestDist < animation.duration * 0.05) setSelectedKf(nearest);
  }, [animation]);

  const EASINGS: Keyframe['easing'][] = ['linear', 'ease-in', 'ease-out', 'ease-in-out', 'bounce'];

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-fg">Animation Editor</h1>
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
            className="px-4 py-1.5 text-sm bg-green-600 hover:bg-green-700 rounded-md disabled:opacity-50"
          >
            {saveMut.isPending ? 'Saving...' : 'Save & Push'}
          </button>
          {saveMut.isSuccess && <span className="text-xs text-green-400">Saved!</span>}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[1fr_300px] gap-5">
        {/* Left column */}
        <div className="space-y-4">
          {/* Preview */}
          <div className="rounded-xl border border-edge bg-black p-4 flex flex-col items-center">
            <canvas
              ref={canvasRef}
              width={384}
              height={192}
              className="rounded-lg border border-edge"
              style={{ imageRendering: 'pixelated', width: '100%', maxWidth: 480 }}
            />
          </div>

          {/* Transport controls */}
          <div className="flex items-center gap-3">
            <button
              onClick={() => { setPlayTime(0); setPlaying(false); }}
              className="p-2 rounded-md bg-inset text-muted hover:text-fg"
              title="Rewind"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M2 3h2v10H2V3zm4 5l6-5v10L6 8z"/></svg>
            </button>
            <button
              onClick={() => { setPlaying(!playing); if (!playing) lastFrameRef.current = performance.now(); }}
              className={`p-2.5 rounded-full transition-colors ${playing ? 'bg-red-600 text-white' : 'bg-green-600 text-white hover:bg-green-700'}`}
            >
              {playing ? (
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><rect x="3" y="2" width="4" height="12"/><rect x="9" y="2" width="4" height="12"/></svg>
              ) : (
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M4 2l10 6-10 6V2z"/></svg>
              )}
            </button>
            <div className="flex-1 text-xs font-mono text-subtle">
              {Math.round(playTime)}ms / {animation.duration}ms
            </div>
            <label className="flex items-center gap-1.5 text-xs text-subtle">
              <input
                type="checkbox"
                checked={animation.loop}
                onChange={e => setAnimation(prev => ({ ...prev, loop: e.target.checked }))}
                className="rounded border-edge"
              />
              Loop
            </label>
          </div>

          {/* Timeline */}
          <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs text-subtle font-medium uppercase tracking-wider">Timeline</span>
              <div className="flex gap-1">
                <button onClick={addKeyframe} className="px-2 py-1 text-[10px] bg-blue-600 hover:bg-blue-700 rounded text-white">+ Keyframe</button>
                <button onClick={duplicateKeyframe} className="px-2 py-1 text-[10px] bg-raised hover:bg-raised rounded text-fg-2">Duplicate</button>
                <button onClick={deleteKeyframe} disabled={animation.keyframes.length <= 2} className="px-2 py-1 text-[10px] bg-raised hover:bg-raised rounded text-fg-2 disabled:opacity-30">Delete</button>
              </div>
            </div>

            {/* Timeline bar */}
            <div
              ref={timelineRef}
              onClick={handleTimelineClick}
              className="relative h-12 bg-inset rounded-lg cursor-pointer overflow-hidden"
            >
              {/* Playhead */}
              <div
                className="absolute top-0 bottom-0 w-0.5 bg-red-500 z-20 pointer-events-none"
                style={{ left: `${(playTime / animation.duration) * 100}%` }}
              />

              {/* Keyframe markers */}
              {animation.keyframes.map((kf, i) => (
                <button
                  key={i}
                  onClick={e => { e.stopPropagation(); setSelectedKf(i); setPlayTime(kf.time); }}
                  className={`absolute top-1 z-10 flex flex-col items-center`}
                  style={{ left: `calc(${(kf.time / animation.duration) * 100}% - 6px)` }}
                >
                  <div className={`w-3 h-3 rounded-full border-2 transition-all ${
                    i === selectedKf
                      ? 'bg-amber-400 border-amber-300 scale-125 shadow-lg shadow-amber-400/30'
                      : 'bg-raised border-edge hover:bg-raised'
                  }`} />
                  <span className="text-[8px] text-dim mt-0.5 font-mono">{kf.time}</span>
                </button>
              ))}

              {/* Interpolation curves visualization */}
              <svg className="absolute inset-0 w-full h-full pointer-events-none" preserveAspectRatio="none">
                {animation.keyframes.slice(0, -1).map((kf, i) => {
                  const next = animation.keyframes[i + 1];
                  const x1 = (kf.time / animation.duration) * 100;
                  const x2 = (next.time / animation.duration) * 100;
                  // Show a param curve (mouthCurve as example)
                  const y1 = 70 - (kf.params.mouthCurve + 1) * 20;
                  const y2 = 70 - (next.params.mouthCurve + 1) * 20;
                  return (
                    <line key={i} x1={`${x1}%`} y1={`${y1}%`} x2={`${x2}%`} y2={`${y2}%`}
                      stroke="rgba(239,68,68,0.3)" strokeWidth="1" />
                  );
                })}
              </svg>
            </div>

            {/* Duration control */}
            <div className="flex items-center gap-3">
              <label className="text-xs text-subtle">Duration</label>
              <input
                type="number"
                value={animation.duration}
                onChange={e => setAnimation(prev => ({ ...prev, duration: Math.max(100, Number(e.target.value)) }))}
                className="w-24 px-2 py-1 text-xs bg-inset border border-edge rounded text-fg font-mono"
                step={100}
                min={100}
              />
              <span className="text-[10px] text-dim">ms</span>
            </div>
          </div>

          {/* Preset animations */}
          <div className="rounded-lg border border-edge bg-surface p-4">
            <span className="text-xs text-subtle font-medium uppercase tracking-wider">Presets</span>
            <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-7 gap-2 mt-3">
              {PRESET_ANIMATIONS.map(preset => (
                <button
                  key={preset.name}
                  onClick={() => { setAnimation(preset); setSelectedKf(0); setPlayTime(0); setPlaying(false); }}
                  className={`px-3 py-2 text-xs rounded-lg border transition-all ${
                    animation.name === preset.name
                      ? 'bg-red-500/20 text-red-400 border-red-500/30'
                      : 'bg-inset/80 text-muted border-edge hover:border-edge'
                  }`}
                >
                  {preset.name}
                </button>
              ))}
            </div>
          </div>

          {/* Animation name */}
          <div className="flex items-center gap-3">
            <label className="text-xs text-subtle">Name</label>
            <input
              type="text"
              value={animation.name}
              onChange={e => setAnimation(prev => ({ ...prev, name: e.target.value }))}
              className="flex-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg"
            />
          </div>
        </div>

        {/* Right column: Keyframe editor */}
        <div className="space-y-4">
          <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs text-subtle font-medium uppercase tracking-wider">
                Keyframe {selectedKf + 1}/{animation.keyframes.length}
              </span>
              <span className="text-[10px] font-mono text-dim">
                {animation.keyframes[selectedKf]?.time ?? 0}ms
              </span>
            </div>

            {/* Time */}
            <div>
              <label className="text-[10px] text-dim">Time (ms)</label>
              <input
                type="range"
                min={0} max={animation.duration} step={10}
                value={animation.keyframes[selectedKf]?.time ?? 0}
                onChange={e => updateKfTime(Number(e.target.value))}
                className="w-full h-1 bg-raised rounded-full appearance-none cursor-pointer
                  [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3
                  [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-amber-400 [&::-webkit-slider-thumb]:cursor-pointer"
              />
            </div>

            {/* Easing */}
            <div>
              <label className="text-[10px] text-dim mb-1 block">Easing</label>
              <div className="flex flex-wrap gap-1">
                {EASINGS.map(e => (
                  <button
                    key={e}
                    onClick={() => updateKfEasing(e)}
                    className={`px-2 py-1 text-[10px] rounded transition-colors ${
                      animation.keyframes[selectedKf]?.easing === e
                        ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30'
                        : 'bg-inset text-subtle border border-edge'
                    }`}
                  >
                    {e}
                  </button>
                ))}
              </div>
            </div>
          </div>

          {/* Param sliders */}
          <div className="rounded-lg border border-edge bg-surface p-4 space-y-2.5">
            <span className="text-xs text-subtle font-medium uppercase tracking-wider">Parameters</span>
            {PARAM_KEYS.map(key => {
              const [min, max] = PARAM_RANGES[key];
              const val = animation.keyframes[selectedKf]?.params[key] ?? 0;
              return (
                <div key={key}>
                  <div className="flex items-center justify-between mb-0.5">
                    <label className="text-[10px] text-subtle">{PARAM_LABELS[key]}</label>
                    <span className="text-[10px] font-mono text-dim tabular-nums">{val.toFixed(2)}</span>
                  </div>
                  <input
                    type="range"
                    min={min} max={max} step={0.05}
                    value={val}
                    onChange={e => updateKfParam(key, parseFloat(e.target.value))}
                    className="w-full h-1 bg-raised rounded-full appearance-none cursor-pointer
                      [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-2.5 [&::-webkit-slider-thumb]:h-2.5
                      [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-red-500 [&::-webkit-slider-thumb]:cursor-pointer"
                  />
                </div>
              );
            })}
          </div>

          {/* Quick param buttons */}
          <div className="rounded-lg border border-edge bg-surface p-4 space-y-2">
            <span className="text-xs text-subtle font-medium uppercase tracking-wider">Quick Set</span>
            <div className="grid grid-cols-2 gap-1.5">
              {[
                { label: 'Neutral', p: DEFAULT_PARAMS },
                { label: 'Wide Eyes', p: { ...DEFAULT_PARAMS, eyeOpen: 1.2, browAngle: 0.5, browY: -0.5 } },
                { label: 'Angry', p: { ...DEFAULT_PARAMS, browAngle: -1, browY: -2, mouthCurve: -0.8, eyeOpen: 0.7 } },
                { label: 'Happy', p: { ...DEFAULT_PARAMS, eyeOpen: 0.6, mouthCurve: 1, mouthOpen: 0.2, browAngle: -0.2, browY: 0.3 } },
                { label: 'Sad', p: { ...DEFAULT_PARAMS, eyeOpen: 0.8, mouthCurve: -0.6, browAngle: 0.5, browY: 0.5 } },
                { label: 'Wink', p: { ...DEFAULT_PARAMS, eyeOpen: 0.5, eyeX: 0.3, mouthCurve: 0.4 } },
              ].map(({ label, p }) => (
                <button
                  key={label}
                  onClick={() => setAnimation(prev => {
                    const kfs = [...prev.keyframes];
                    kfs[selectedKf] = { ...kfs[selectedKf], params: p };
                    return { ...prev, keyframes: kfs };
                  })}
                  className="px-2 py-1.5 text-[10px] bg-inset hover:bg-raised rounded text-muted border border-edge"
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─── Interpolation ──────────────────────────────────────────────

function applyEasing(t: number, easing: Keyframe['easing']): number {
  switch (easing) {
    case 'ease-in': return t * t;
    case 'ease-out': return 1 - (1 - t) * (1 - t);
    case 'ease-in-out': return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
    case 'bounce': {
      const n = 7.5625;
      const d = 2.75;
      if (t < 1 / d) return n * t * t;
      if (t < 2 / d) return n * (t -= 1.5 / d) * t + 0.75;
      if (t < 2.5 / d) return n * (t -= 2.25 / d) * t + 0.9375;
      return n * (t -= 2.625 / d) * t + 0.984375;
    }
    default: return t;
  }
}

function interpolateAtTime(keyframes: Keyframe[], time: number): AvatarParams {
  if (keyframes.length === 0) return DEFAULT_PARAMS;
  if (keyframes.length === 1) return keyframes[0].params;
  if (time <= keyframes[0].time) return keyframes[0].params;
  if (time >= keyframes[keyframes.length - 1].time) return keyframes[keyframes.length - 1].params;

  let i = 0;
  while (i < keyframes.length - 1 && keyframes[i + 1].time <= time) i++;

  const a = keyframes[i];
  const b = keyframes[i + 1];
  const rawT = (time - a.time) / (b.time - a.time);
  const t = applyEasing(rawT, b.easing);

  const result: AvatarParams = { ...DEFAULT_PARAMS };
  for (const key of PARAM_KEYS) {
    result[key] = a.params[key] + (b.params[key] - a.params[key]) * t;
  }
  return result;
}

// ─── Preview Renderer ───────────────────────────────────────────

function drawPreview(ctx: CanvasRenderingContext2D, w: number, h: number, p: AvatarParams) {
  const S = 3;
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, w, h);

  const cx = w / 2 + p.shake * S;
  const cy = h / 2 + p.bounce * S + 4 * S;

  // Brows
  const eyeSpacing = 18 * S;
  const browBaseY = cy - 18 * S;
  ctx.strokeStyle = '#fff';
  ctx.lineCap = 'round';
  ctx.lineWidth = 2.5 * S;
  for (const side of [-1, 1]) {
    const bx = cx + side * eyeSpacing;
    const by = browBaseY + p.browY * 4 * S;
    const tilt = p.browAngle * 3 * S;
    ctx.beginPath();
    ctx.moveTo(bx - side * 2 * S, by - tilt);
    ctx.lineTo(bx + side * 8 * S, by + tilt);
    ctx.stroke();
  }

  // Eyes
  const eyeBaseY = cy - 8 * S;
  const eyeW = 10 * S;
  const eyeMaxH = 12 * S;
  const eyeH = Math.max(1, eyeMaxH * Math.max(0, Math.min(p.eyeOpen, 1.2)));

  for (const side of [-1, 1]) {
    const ex = cx + side * eyeSpacing;
    if (eyeH <= 2 * S) {
      ctx.strokeStyle = '#fff';
      ctx.lineWidth = S;
      ctx.beginPath();
      ctx.moveTo(ex - eyeW / 2, eyeBaseY);
      ctx.lineTo(ex + eyeW / 2, eyeBaseY);
      ctx.stroke();
    } else {
      ctx.fillStyle = '#fff';
      const r = Math.min(eyeW / 2, eyeH / 2);
      ctx.beginPath();
      ctx.moveTo(ex - eyeW / 2 + r, eyeBaseY - eyeH / 2);
      ctx.arcTo(ex + eyeW / 2, eyeBaseY - eyeH / 2, ex + eyeW / 2, eyeBaseY + eyeH / 2, r);
      ctx.arcTo(ex + eyeW / 2, eyeBaseY + eyeH / 2, ex - eyeW / 2, eyeBaseY + eyeH / 2, r);
      ctx.arcTo(ex - eyeW / 2, eyeBaseY + eyeH / 2, ex - eyeW / 2, eyeBaseY - eyeH / 2, r);
      ctx.arcTo(ex - eyeW / 2, eyeBaseY - eyeH / 2, ex + eyeW / 2, eyeBaseY - eyeH / 2, r);
      ctx.fill();
      if (eyeH > 5 * S) {
        ctx.fillStyle = '#000';
        ctx.beginPath();
        ctx.arc(ex + p.eyeX * 3 * S, eyeBaseY + p.eyeY * 2 * S, 2.5 * S, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = '#fff';
        ctx.beginPath();
        ctx.arc(ex + p.eyeX * 3 * S - S, eyeBaseY + p.eyeY * 2 * S - S, 0.8 * S, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }

  // Mouth
  const mouthY = cy + 12 * S;
  const mouthW = 16 * S;
  const curve = p.mouthCurve * 6 * S;
  const openH = p.mouthOpen * 6 * S;
  ctx.strokeStyle = '#fff';
  ctx.fillStyle = '#fff';
  ctx.lineWidth = 1.5 * S;
  if (openH > S) {
    ctx.beginPath();
    ctx.moveTo(cx - mouthW / 2, mouthY);
    ctx.quadraticCurveTo(cx, mouthY + curve, cx + mouthW / 2, mouthY);
    ctx.quadraticCurveTo(cx, mouthY + curve + openH, cx - mouthW / 2, mouthY);
    ctx.fill();
  } else {
    ctx.beginPath();
    ctx.moveTo(cx - mouthW / 2, mouthY);
    ctx.quadraticCurveTo(cx, mouthY + curve, cx + mouthW / 2, mouthY);
    ctx.stroke();
  }
}
