import { useState, useRef, useCallback } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import { getDevices, sendState } from '../api/client';

// ─── Types ──────────────────────────────────────────────────────

interface Note {
  freq: number;
  duration: number;
  name: string;
  color: string;
}

interface SavedPreset {
  name: string;
  notes: Note[];
}

// ─── Constants ──────────────────────────────────────────────────

const MUSICAL_NOTES = [
  { name: 'C4', freq: 262, color: '#ef4444' },
  { name: 'D4', freq: 294, color: '#f97316' },
  { name: 'E4', freq: 330, color: '#eab308' },
  { name: 'F4', freq: 349, color: '#22c55e' },
  { name: 'G4', freq: 392, color: '#14b8a6' },
  { name: 'A4', freq: 440, color: '#3b82f6' },
  { name: 'B4', freq: 494, color: '#6366f1' },
  { name: 'C5', freq: 523, color: '#8b5cf6' },
  { name: 'D5', freq: 587, color: '#ec4899' },
  { name: 'E5', freq: 659, color: '#f43f5e' },
];

const DURATION_OPTIONS = [
  { label: '100ms', value: 100 },
  { label: '200ms', value: 200 },
  { label: '400ms', value: 400 },
];

const QUICK_SOUNDS = [
  { label: 'Ding!', state: 'success', color: '#22c55e', bgClass: 'bg-green-500/15 hover:bg-green-500/25 border-green-500/30', textClass: 'text-green-400' },
  { label: 'Error Buzz', state: 'error', color: '#ef4444', bgClass: 'bg-red-500/15 hover:bg-red-500/25 border-red-500/30', textClass: 'text-red-400' },
  { label: 'Think...', state: 'thinking', color: '#8b5cf6', bgClass: 'bg-purple-500/15 hover:bg-purple-500/25 border-purple-500/30', textClass: 'text-purple-400' },
  { label: 'Alert!', state: 'waiting', color: '#f97316', bgClass: 'bg-orange-500/15 hover:bg-orange-500/25 border-orange-500/30', textClass: 'text-orange-400' },
  { label: 'Check!', state: 'taskcheck', color: '#14b8a6', bgClass: 'bg-teal-500/15 hover:bg-teal-500/25 border-teal-500/30', textClass: 'text-teal-400' },
  { label: 'Chill', state: 'idle', color: '#3b82f6', bgClass: 'bg-blue-500/15 hover:bg-blue-500/25 border-blue-500/30', textClass: 'text-blue-400' },
];

const PRESET_MELODIES: SavedPreset[] = [
  {
    name: 'Victory Fanfare',
    notes: [
      { freq: 262, duration: 200, name: 'C4', color: '#ef4444' },
      { freq: 330, duration: 200, name: 'E4', color: '#eab308' },
      { freq: 392, duration: 200, name: 'G4', color: '#14b8a6' },
      { freq: 523, duration: 400, name: 'C5', color: '#8b5cf6' },
    ],
  },
  {
    name: 'Sad Trombone',
    notes: [
      { freq: 392, duration: 400, name: 'G4', color: '#14b8a6' },
      { freq: 349, duration: 400, name: 'F4', color: '#22c55e' },
      { freq: 330, duration: 400, name: 'E4', color: '#eab308' },
      { freq: 294, duration: 400, name: 'D4', color: '#f97316' },
    ],
  },
  {
    name: 'Alert Beep',
    notes: [
      { freq: 440, duration: 100, name: 'A4', color: '#3b82f6' },
      { freq: 440, duration: 100, name: 'A4', color: '#3b82f6' },
      { freq: 440, duration: 100, name: 'A4', color: '#3b82f6' },
    ],
  },
  {
    name: 'Mario Coin',
    notes: [
      { freq: 988, duration: 100, name: 'B5', color: '#6366f1' },
      { freq: 1319, duration: 200, name: 'E6', color: '#eab308' },
    ],
  },
  {
    name: 'Imperial March',
    notes: [
      { freq: 392, duration: 400, name: 'G4', color: '#14b8a6' },
      { freq: 392, duration: 400, name: 'G4', color: '#14b8a6' },
      { freq: 392, duration: 400, name: 'G4', color: '#14b8a6' },
      { freq: 311, duration: 200, name: 'Eb4', color: '#f97316' },
      { freq: 466, duration: 100, name: 'Bb4', color: '#ec4899' },
      { freq: 392, duration: 400, name: 'G4', color: '#14b8a6' },
      { freq: 311, duration: 200, name: 'Eb4', color: '#f97316' },
      { freq: 466, duration: 100, name: 'Bb4', color: '#ec4899' },
      { freq: 392, duration: 400, name: 'G4', color: '#14b8a6' },
    ],
  },
  {
    name: 'Nokia Tune',
    notes: [
      { freq: 659, duration: 100, name: 'E5', color: '#f43f5e' },
      { freq: 587, duration: 100, name: 'D5', color: '#ec4899' },
      { freq: 370, duration: 200, name: 'F#4', color: '#22c55e' },
      { freq: 415, duration: 200, name: 'G#4', color: '#14b8a6' },
      { freq: 554, duration: 100, name: 'C#5', color: '#8b5cf6' },
      { freq: 494, duration: 100, name: 'B4', color: '#6366f1' },
      { freq: 294, duration: 200, name: 'D4', color: '#f97316' },
      { freq: 330, duration: 200, name: 'E4', color: '#eab308' },
      { freq: 494, duration: 100, name: 'B4', color: '#6366f1' },
      { freq: 440, duration: 100, name: 'A4', color: '#3b82f6' },
      { freq: 277, duration: 200, name: 'C#4', color: '#ef4444' },
      { freq: 330, duration: 200, name: 'E4', color: '#eab308' },
      { freq: 440, duration: 400, name: 'A4', color: '#3b82f6' },
    ],
  },
];

// ─── Web Audio Preview ──────────────────────────────────────────

function playMelodyInBrowser(notes: Note[]): Promise<void> {
  return new Promise((resolve) => {
    if (notes.length === 0) { resolve(); return; }
    const ctx = new AudioContext();
    let offset = 0;
    for (const note of notes) {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'square';
      osc.frequency.value = note.freq;
      gain.gain.setValueAtTime(0.15, ctx.currentTime + offset / 1000);
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + (offset + note.duration) / 1000);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(ctx.currentTime + offset / 1000);
      osc.stop(ctx.currentTime + (offset + note.duration) / 1000);
      offset += note.duration + 30; // 30ms gap between notes
    }
    setTimeout(() => {
      ctx.close();
      resolve();
    }, offset);
  });
}

// ─── Component ──────────────────────────────────────────────────

export default function SoundBoardPage() {
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [selectedDevice, setSelectedDevice] = useState('');
  const deviceId = selectedDevice || devices?.[0]?.id;

  // Sequencer state
  const [sequence, setSequence] = useState<Note[]>([]);
  const [selectedDuration, setSelectedDuration] = useState(200);
  const [isPlaying, setIsPlaying] = useState(false);
  const [presetName, setPresetName] = useState('');
  const [savedPresets, setSavedPresets] = useState<SavedPreset[]>(() => {
    try {
      const stored = localStorage.getItem('hookbot-sound-presets');
      return stored ? JSON.parse(stored) : [];
    } catch { return []; }
  });
  const [activeQuickSound, setActiveQuickSound] = useState<string | null>(null);
  const playingRef = useRef(false);

  // Send state to device
  const stateMut = useMutation({
    mutationFn: ({ id, state }: { id: string; state: string }) => sendState(id, state),
  });

  const handleQuickSound = useCallback(async (state: string) => {
    setActiveQuickSound(state);
    if (deviceId) {
      stateMut.mutate({ id: deviceId, state });
    }
    setTimeout(() => setActiveQuickSound(null), 600);
  }, [deviceId, stateMut]);

  const addNote = useCallback((noteInfo: typeof MUSICAL_NOTES[0]) => {
    setSequence(prev => [...prev, {
      freq: noteInfo.freq,
      duration: selectedDuration,
      name: noteInfo.name,
      color: noteInfo.color,
    }]);
  }, [selectedDuration]);

  const removeNote = useCallback((index: number) => {
    setSequence(prev => prev.filter((_, i) => i !== index));
  }, []);

  const playSequence = useCallback(async (notes: Note[]) => {
    if (playingRef.current || notes.length === 0) return;
    playingRef.current = true;
    setIsPlaying(true);
    await playMelodyInBrowser(notes);
    playingRef.current = false;
    setIsPlaying(false);
  }, []);

  const savePreset = useCallback(() => {
    if (!presetName.trim() || sequence.length === 0) return;
    const preset: SavedPreset = { name: presetName.trim(), notes: [...sequence] };
    setSavedPresets(prev => {
      const updated = [...prev.filter(p => p.name !== preset.name), preset];
      localStorage.setItem('hookbot-sound-presets', JSON.stringify(updated));
      return updated;
    });
    setPresetName('');
  }, [presetName, sequence]);

  const deletePreset = useCallback((name: string) => {
    setSavedPresets(prev => {
      const updated = prev.filter(p => p.name !== name);
      localStorage.setItem('hookbot-sound-presets', JSON.stringify(updated));
      return updated;
    });
  }, []);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-fg">Sound Board</h1>
        <select
          value={selectedDevice}
          onChange={e => setSelectedDevice(e.target.value)}
          className="px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg"
        >
          <option value="">Select device...</option>
          {devices?.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
        </select>
      </div>

      {/* Quick Reaction Sounds */}
      <div className="rounded-xl border border-edge bg-surface p-5">
        <h2 className="text-xs font-semibold text-subtle uppercase tracking-wider mb-4">Quick Reaction Sounds</h2>
        {!deviceId && (
          <p className="text-xs text-dim mb-3">Select a device to send sounds</p>
        )}
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
          {QUICK_SOUNDS.map(sound => (
            <button
              key={sound.state}
              onClick={() => handleQuickSound(sound.state)}
              disabled={!deviceId || stateMut.isPending}
              className={`relative p-5 rounded-xl border-2 transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed ${sound.bgClass} ${
                activeQuickSound === sound.state ? 'scale-95 ring-2 ring-white/20' : 'hover:scale-[1.02]'
              }`}
            >
              <div className={`text-lg font-bold ${sound.textClass}`}>{sound.label}</div>
              <div className="text-[10px] text-dim mt-1 font-mono">{sound.state}</div>
              {activeQuickSound === sound.state && (
                <div className="absolute inset-0 rounded-xl animate-ping opacity-20" style={{ backgroundColor: sound.color }} />
              )}
            </button>
          ))}
        </div>
      </div>

      {/* Custom Melody Composer */}
      <div className="rounded-xl border border-edge bg-surface p-5 space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-xs font-semibold text-subtle uppercase tracking-wider">Custom Melody Composer</h2>
          <span className="text-[10px] text-dim bg-brand/15 text-brand-fg px-2 py-0.5 rounded-full">
            Browser preview only -- device sound packs coming soon
          </span>
        </div>

        {/* Duration selector */}
        <div className="flex items-center gap-2">
          <span className="text-xs text-subtle">Note duration:</span>
          {DURATION_OPTIONS.map(opt => (
            <button
              key={opt.value}
              onClick={() => setSelectedDuration(opt.value)}
              className={`px-3 py-1.5 text-xs rounded-md border transition-colors ${
                selectedDuration === opt.value
                  ? 'bg-indigo-500/20 text-indigo-400 border-indigo-500/30'
                  : 'bg-inset text-muted border-edge hover:text-fg'
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>

        {/* Piano keys */}
        <div className="flex gap-1.5 flex-wrap">
          {MUSICAL_NOTES.map(note => (
            <button
              key={note.name}
              onClick={() => addNote(note)}
              className="flex flex-col items-center px-3 py-3 rounded-lg border border-edge hover:scale-105 transition-all min-w-[52px]"
              style={{
                backgroundColor: `${note.color}15`,
                borderColor: `${note.color}40`,
              }}
            >
              <span className="text-sm font-bold" style={{ color: note.color }}>{note.name}</span>
              <span className="text-[9px] text-dim font-mono mt-0.5">{note.freq}Hz</span>
            </button>
          ))}
        </div>

        {/* Sequence display */}
        <div className="min-h-[56px] rounded-lg bg-inset border border-edge p-3 flex flex-wrap gap-1.5 items-center">
          {sequence.length === 0 ? (
            <span className="text-xs text-dim">Tap notes above to build a melody...</span>
          ) : (
            sequence.map((note, i) => (
              <button
                key={i}
                onClick={() => removeNote(i)}
                className="group relative flex flex-col items-center px-2 py-1.5 rounded-md border transition-all hover:scale-105 hover:opacity-80"
                style={{
                  backgroundColor: `${note.color}20`,
                  borderColor: `${note.color}50`,
                  minWidth: Math.max(36, note.duration / 8),
                }}
                title={`${note.name} (${note.duration}ms) -- click to remove`}
              >
                <span className="text-[11px] font-bold" style={{ color: note.color }}>{note.name}</span>
                <span className="text-[8px] text-dim">{note.duration}ms</span>
                <span className="absolute -top-1 -right-1 w-3 h-3 rounded-full bg-red-500 text-white text-[8px] flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">x</span>
              </button>
            ))
          )}
        </div>

        {/* Controls */}
        <div className="flex flex-wrap items-center gap-2">
          <button
            onClick={() => playSequence(sequence)}
            disabled={sequence.length === 0 || isPlaying}
            className="px-5 py-2 text-sm font-medium rounded-lg bg-green-600 hover:bg-green-700 text-white disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            {isPlaying ? 'Playing...' : 'Preview'}
          </button>
          <button
            onClick={() => setSequence([])}
            disabled={sequence.length === 0}
            className="px-4 py-2 text-sm rounded-lg border border-edge bg-inset text-muted hover:text-fg disabled:opacity-40 transition-colors"
          >
            Clear
          </button>

          <div className="flex-1" />

          {/* Save preset */}
          <input
            type="text"
            value={presetName}
            onChange={e => setPresetName(e.target.value)}
            placeholder="Preset name..."
            className="px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg placeholder:text-dim focus:outline-none focus:ring-1 focus:ring-indigo-500 w-40"
          />
          <button
            onClick={savePreset}
            disabled={!presetName.trim() || sequence.length === 0}
            className="px-4 py-2 text-sm rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white disabled:opacity-40 transition-colors"
          >
            Save
          </button>
        </div>
      </div>

      {/* Preset Melodies */}
      <div className="rounded-xl border border-edge bg-surface p-5">
        <h2 className="text-xs font-semibold text-subtle uppercase tracking-wider mb-4">Preset Melodies</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {PRESET_MELODIES.map(preset => (
            <div
              key={preset.name}
              className="rounded-lg border border-edge bg-inset p-3 space-y-2"
            >
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-fg">{preset.name}</span>
                <button
                  onClick={() => playSequence(preset.notes)}
                  disabled={isPlaying}
                  className="px-3 py-1 text-xs rounded-md bg-green-600 hover:bg-green-700 text-white disabled:opacity-40 transition-colors"
                >
                  Preview
                </button>
              </div>
              <div className="flex gap-1 flex-wrap">
                {preset.notes.map((note, i) => (
                  <span
                    key={i}
                    className="text-[10px] font-mono px-1.5 py-0.5 rounded"
                    style={{ backgroundColor: `${note.color}20`, color: note.color }}
                  >
                    {note.name}
                  </span>
                ))}
              </div>
              <button
                onClick={() => setSequence([...preset.notes])}
                className="text-[10px] text-subtle hover:text-fg transition-colors"
              >
                Load into composer
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* Saved Custom Presets */}
      {savedPresets.length > 0 && (
        <div className="rounded-xl border border-edge bg-surface p-5">
          <h2 className="text-xs font-semibold text-subtle uppercase tracking-wider mb-4">Your Saved Melodies</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {savedPresets.map(preset => (
              <div
                key={preset.name}
                className="rounded-lg border border-edge bg-inset p-3 space-y-2"
              >
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-fg">{preset.name}</span>
                  <div className="flex gap-1">
                    <button
                      onClick={() => playSequence(preset.notes)}
                      disabled={isPlaying}
                      className="px-2 py-1 text-xs rounded-md bg-green-600 hover:bg-green-700 text-white disabled:opacity-40 transition-colors"
                    >
                      Preview
                    </button>
                    <button
                      onClick={() => deletePreset(preset.name)}
                      className="px-2 py-1 text-xs rounded-md border border-edge text-red-400 hover:bg-red-500/10 transition-colors"
                    >
                      Delete
                    </button>
                  </div>
                </div>
                <div className="flex gap-1 flex-wrap">
                  {preset.notes.map((note, i) => (
                    <span
                      key={i}
                      className="text-[10px] font-mono px-1.5 py-0.5 rounded"
                      style={{ backgroundColor: `${note.color}20`, color: note.color }}
                    >
                      {note.name}
                    </span>
                  ))}
                </div>
                <button
                  onClick={() => setSequence([...preset.notes])}
                  className="text-[10px] text-subtle hover:text-fg transition-colors"
                >
                  Load into composer
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
