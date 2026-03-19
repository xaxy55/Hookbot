import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getMoodEntries,
  createMoodEntry,
  getMoodStats,
  type MoodEntry,
} from '../api/client';

const MOODS = [
  { emoji: '\u{1F929}', label: 'Ecstatic' },
  { emoji: '\u{1F60A}', label: 'Happy' },
  { emoji: '\u{1F642}', label: 'Content' },
  { emoji: '\u{1F610}', label: 'Neutral' },
  { emoji: '\u{1F634}', label: 'Tired' },
  { emoji: '\u{1F630}', label: 'Stressed' },
  { emoji: '\u{1F624}', label: 'Frustrated' },
  { emoji: '\u{1F622}', label: 'Sad' },
  { emoji: '\u{1F621}', label: 'Angry' },
  { emoji: '\u{1F3A8}', label: 'Creative' },
  { emoji: '\u{1F9D0}', label: 'Focused' },
  { emoji: '\u{1F389}', label: 'Celebrating' },
];

const ENERGY_LABELS = ['Drained', 'Low', 'Moderate', 'Good', 'Supercharged'];
const ENERGY_COLORS = [
  'bg-red-500/80',
  'bg-orange-500/80',
  'bg-yellow-500/80',
  'bg-lime-500/80',
  'bg-green-500/80',
];

function moodColor(emoji: string): string {
  const map: Record<string, string> = {
    '\u{1F929}': 'bg-fuchsia-500/20 border-fuchsia-500/40',
    '\u{1F60A}': 'bg-yellow-500/20 border-yellow-500/40',
    '\u{1F642}': 'bg-emerald-500/20 border-emerald-500/40',
    '\u{1F610}': 'bg-slate-400/20 border-slate-400/40',
    '\u{1F634}': 'bg-indigo-500/20 border-indigo-500/40',
    '\u{1F630}': 'bg-orange-500/20 border-orange-500/40',
    '\u{1F624}': 'bg-red-400/20 border-red-400/40',
    '\u{1F622}': 'bg-blue-500/20 border-blue-500/40',
    '\u{1F621}': 'bg-red-600/20 border-red-600/40',
    '\u{1F3A8}': 'bg-violet-500/20 border-violet-500/40',
    '\u{1F9D0}': 'bg-cyan-500/20 border-cyan-500/40',
    '\u{1F389}': 'bg-amber-500/20 border-amber-500/40',
  };
  return map[emoji] || 'bg-surface border-edge';
}

function moodDotColor(emoji: string): string {
  const map: Record<string, string> = {
    '\u{1F929}': 'bg-fuchsia-500',
    '\u{1F60A}': 'bg-yellow-500',
    '\u{1F642}': 'bg-emerald-500',
    '\u{1F610}': 'bg-slate-400',
    '\u{1F634}': 'bg-indigo-500',
    '\u{1F630}': 'bg-orange-500',
    '\u{1F624}': 'bg-red-400',
    '\u{1F622}': 'bg-blue-500',
    '\u{1F621}': 'bg-red-600',
    '\u{1F3A8}': 'bg-violet-500',
    '\u{1F9D0}': 'bg-cyan-500',
    '\u{1F389}': 'bg-amber-500',
  };
  return map[emoji] || 'bg-slate-500';
}

function timeAgo(dateStr: string): string {
  const now = new Date();
  const d = new Date(dateStr + 'Z');
  const diffMs = now.getTime() - d.getTime();
  const mins = Math.floor(diffMs / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  if (days < 7) return `${days}d ago`;
  return d.toLocaleDateString();
}

export default function MoodJournalPage() {
  const qc = useQueryClient();

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [selectedDevice, setSelectedDevice] = useState('');
  const deviceId = selectedDevice || devices?.[0]?.id;

  const [selectedMood, setSelectedMood] = useState<string | null>(null);
  const [energy, setEnergy] = useState(3);
  const [note, setNote] = useState('');

  const { data: entries } = useQuery({
    queryKey: ['mood-entries', deviceId],
    queryFn: () => getMoodEntries(deviceId, 30),
    enabled: !!deviceId,
  });

  const { data: stats } = useQuery({
    queryKey: ['mood-stats', deviceId],
    queryFn: () => getMoodStats(deviceId),
    enabled: !!deviceId,
  });

  const addEntry = useMutation({
    mutationFn: (data: { mood: string; note?: string; energy: number }) =>
      createMoodEntry({ device_id: deviceId, ...data }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['mood-entries'] });
      qc.invalidateQueries({ queryKey: ['mood-stats'] });
      setSelectedMood(null);
      setNote('');
      setEnergy(3);
    },
  });

  const handleSubmit = () => {
    if (!selectedMood) return;
    addEntry.mutate({ mood: selectedMood, note: note || undefined, energy });
  };

  const maxDistCount = stats?.mood_distribution?.reduce((m, d) => Math.max(m, d.count), 0) || 1;

  // Timeline entries (most recent 20, reversed for left-to-right chronological)
  const timelineEntries = (entries ?? []).slice(0, 20).reverse();

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-fg">Mood Journal</h1>
          <p className="text-sm text-subtle">Track how your bot is feeling over time</p>
        </div>
        <select
          value={selectedDevice}
          onChange={(e) => setSelectedDevice(e.target.value)}
          className="px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg"
        >
          <option value="">Default device</option>
          {devices?.map((d) => (
            <option key={d.id} value={d.id}>{d.name}</option>
          ))}
        </select>
      </div>

      {/* Quick Mood Picker */}
      <div className="bg-surface border border-edge rounded-lg p-5">
        <h2 className="text-sm font-semibold text-fg mb-3">How are you feeling?</h2>
        <div className="grid grid-cols-4 sm:grid-cols-6 gap-2">
          {MOODS.map((m) => (
            <button
              key={m.emoji}
              onClick={() => setSelectedMood(selectedMood === m.emoji ? null : m.emoji)}
              className={`flex flex-col items-center gap-1 p-3 rounded-lg border transition-all ${
                selectedMood === m.emoji
                  ? `${moodColor(m.emoji)} ring-2 ring-brand/50 scale-105`
                  : 'bg-inset border-edge hover:border-brand/30 hover:bg-brand/5'
              }`}
            >
              <span className="text-2xl">{m.emoji}</span>
              <span className="text-[10px] text-muted">{m.label}</span>
            </button>
          ))}
        </div>

        {/* Energy Slider */}
        {selectedMood && (
          <div className="mt-4 space-y-3">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-medium text-fg">Energy Level</h3>
              <span className="text-xs text-subtle">{ENERGY_LABELS[energy - 1]}</span>
            </div>
            <div className="flex gap-1.5">
              {[1, 2, 3, 4, 5].map((level) => (
                <button
                  key={level}
                  onClick={() => setEnergy(level)}
                  className={`flex-1 h-8 rounded-md transition-all ${
                    level <= energy ? ENERGY_COLORS[energy - 1] : 'bg-inset border border-edge'
                  } ${level <= energy ? 'shadow-sm' : ''}`}
                  title={ENERGY_LABELS[level - 1]}
                />
              ))}
            </div>

            {/* Note Field */}
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder="What's going on? (optional)"
              className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg placeholder:text-dim resize-none h-20"
            />

            <button
              onClick={handleSubmit}
              disabled={addEntry.isPending}
              className="w-full px-4 py-2 text-sm font-medium bg-brand/15 text-brand-fg border border-brand/30 rounded-md hover:bg-brand/25 transition-colors disabled:opacity-50"
            >
              {addEntry.isPending ? 'Saving...' : `Log ${selectedMood} Mood`}
            </button>
          </div>
        )}
      </div>

      {/* Mood Timeline */}
      {timelineEntries.length > 0 && (
        <div className="bg-surface border border-edge rounded-lg p-5">
          <h2 className="text-sm font-semibold text-fg mb-4">Recent Mood Timeline</h2>
          <div className="flex items-end gap-1 overflow-x-auto pb-2">
            {timelineEntries.map((entry) => (
              <div key={entry.id} className="flex flex-col items-center gap-1 min-w-[3rem] group">
                <span className="text-lg opacity-0 group-hover:opacity-100 transition-opacity">
                  {entry.mood}
                </span>
                <div
                  className={`w-5 h-5 rounded-full ${moodDotColor(entry.mood)} transition-transform group-hover:scale-125`}
                  title={`${entry.mood} - ${timeAgo(entry.created_at)}`}
                />
                <span className="text-[9px] text-dim opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap">
                  {timeAgo(entry.created_at)}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Stats Dashboard */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        <StatCard label="This Week" value={stats?.this_week ?? 0} />
        <StatCard
          label="Avg Energy"
          value={stats?.avg_energy ? stats.avg_energy.toFixed(1) : '--'}
        />
        <StatCard
          label="Most Common"
          value={stats?.most_common_mood ?? '--'}
          large
        />
        <StatCard label="Total Entries" value={stats?.total_entries ?? 0} />
      </div>

      {/* Mood Distribution */}
      {stats?.mood_distribution && stats.mood_distribution.length > 0 && (
        <div className="bg-surface border border-edge rounded-lg p-5">
          <h2 className="text-sm font-semibold text-fg mb-3">Mood Distribution</h2>
          <div className="space-y-2">
            {stats.mood_distribution.map((d) => (
              <div key={d.mood} className="flex items-center gap-3">
                <span className="text-lg w-8 text-center">{d.mood}</span>
                <div className="flex-1 bg-inset rounded-full h-5 overflow-hidden">
                  <div
                    className={`h-full rounded-full ${moodDotColor(d.mood)} transition-all`}
                    style={{ width: `${(d.count / maxDistCount) * 100}%` }}
                  />
                </div>
                <span className="text-xs text-muted w-8 text-right">{d.count}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Mood History List */}
      {entries && entries.length > 0 && (
        <div className="bg-surface border border-edge rounded-lg p-5">
          <h2 className="text-sm font-semibold text-fg mb-3">Mood History</h2>
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {entries.map((entry) => (
              <HistoryRow key={entry.id} entry={entry} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function StatCard({ label, value, large }: { label: string; value: string | number; large?: boolean }) {
  return (
    <div className="bg-surface border border-edge rounded-lg p-4">
      <p className="text-xs text-muted mb-1">{label}</p>
      <p className={`font-bold text-fg ${large ? 'text-2xl' : 'text-lg'}`}>{value}</p>
    </div>
  );
}

function HistoryRow({ entry }: { entry: MoodEntry }) {
  return (
    <div className={`flex items-center gap-3 p-3 rounded-lg border ${moodColor(entry.mood)}`}>
      <span className="text-2xl">{entry.mood}</span>
      <div className="flex-1 min-w-0">
        {entry.note && (
          <p className="text-sm text-fg truncate">{entry.note}</p>
        )}
        <p className="text-xs text-muted">{timeAgo(entry.created_at)}</p>
      </div>
      <div className="flex gap-0.5">
        {[1, 2, 3, 4, 5].map((level) => (
          <div
            key={level}
            className={`w-1.5 rounded-full ${
              level <= entry.energy
                ? `${ENERGY_COLORS[entry.energy - 1]} h-${2 + level}`
                : 'bg-inset h-2'
            }`}
            style={{ height: level <= entry.energy ? `${8 + level * 3}px` : '8px' }}
          />
        ))}
      </div>
    </div>
  );
}
