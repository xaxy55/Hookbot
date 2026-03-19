import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getPetState, feedPet, petPet, getTokenUsage, recordTokenUsage,
  getDevices,
} from '../api/client';

const FOOD_TYPES = [
  { key: 'snack', label: 'Snack', emoji: '\u{1F36A}', desc: '+15 hunger', color: 'bg-green-700' },
  { key: 'meal', label: 'Meal', emoji: '\u{1F35C}', desc: '+35 hunger', color: 'bg-blue-700' },
  { key: 'feast', label: 'Feast', emoji: '\u{1F969}', desc: '+60 hunger', color: 'bg-purple-700' },
];

const MOOD_EMOJI: Record<string, string> = {
  ecstatic: '\u{1F929}',
  happy: '\u{1F60A}',
  content: '\u{1F642}',
  grumpy: '\u{1F612}',
  sad: '\u{1F622}',
  miserable: '\u{1F62D}',
};

export default function PetCarePage() {
  const qc = useQueryClient();
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [selectedDevice, setSelectedDevice] = useState('');
  const deviceId = selectedDevice || devices?.[0]?.id;
  const [feedMsg, setFeedMsg] = useState('');
  const [tab, setTab] = useState<'care' | 'tokens'>('care');

  // Manual token recording
  const [manualInput, setManualInput] = useState('');
  const [manualOutput, setManualOutput] = useState('');
  const [manualModel, setManualModel] = useState('claude-opus-4-6');

  const { data: pet } = useQuery({
    queryKey: ['pet', deviceId],
    queryFn: () => getPetState(deviceId),
    enabled: !!deviceId,
    refetchInterval: 30000,
  });

  const { data: tokens } = useQuery({
    queryKey: ['tokens', deviceId],
    queryFn: () => getTokenUsage(deviceId, 30),
    enabled: !!deviceId && tab === 'tokens',
  });

  const feed = useMutation({
    mutationFn: (foodType: string) => feedPet(foodType, deviceId),
    onSuccess: (data) => {
      setFeedMsg(data.message);
      qc.invalidateQueries({ queryKey: ['pet', deviceId] });
      setTimeout(() => setFeedMsg(''), 3000);
    },
  });

  const petMut = useMutation({
    mutationFn: () => petPet(deviceId),
    onSuccess: (data) => {
      setFeedMsg(data.message);
      qc.invalidateQueries({ queryKey: ['pet', deviceId] });
      setTimeout(() => setFeedMsg(''), 3000);
    },
  });

  const recordMut = useMutation({
    mutationFn: () => recordTokenUsage({
      device_id: deviceId,
      input_tokens: Number(manualInput) || 0,
      output_tokens: Number(manualOutput) || 0,
      model: manualModel,
    }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['tokens', deviceId] });
      setManualInput('');
      setManualOutput('');
    },
  });

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-fg">Pet Care</h1>
          <p className="text-sm text-subtle">Feed your bot, track token usage</p>
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

      {/* Tabs */}
      <div className="flex gap-1.5">
        <button
          onClick={() => setTab('care')}
          className={`px-4 py-1.5 text-xs rounded-md transition-colors ${
            tab === 'care' ? 'bg-brand/15 text-brand-fg border border-brand/30' : 'bg-inset text-subtle border border-transparent'
          }`}
        >
          Pet Care
        </button>
        <button
          onClick={() => setTab('tokens')}
          className={`px-4 py-1.5 text-xs rounded-md transition-colors ${
            tab === 'tokens' ? 'bg-brand/15 text-brand-fg border border-brand/30' : 'bg-inset text-subtle border border-transparent'
          }`}
        >
          Token Usage
        </button>
      </div>

      {tab === 'care' && pet && (
        <>
          {/* Mood + Stats */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <div className="rounded-lg border border-edge bg-surface p-4 text-center">
              <div className="text-4xl mb-1">{MOOD_EMOJI[pet.mood] ?? '\u{1F610}'}</div>
              <div className="text-sm font-medium text-fg capitalize">{pet.mood}</div>
              <div className="text-[11px] text-subtle uppercase tracking-wider mt-0.5">Mood</div>
            </div>
            <div className="rounded-lg border border-amber-700/30 bg-amber-900/10 p-4 text-center">
              <div className="text-2xl font-bold text-amber-400 font-mono">{pet.hunger}</div>
              <div className="text-[11px] text-amber-400/60 uppercase tracking-wider mt-1">Hunger</div>
              <div className="w-full h-1.5 bg-raised rounded-full mt-2 overflow-hidden">
                <div className="h-full bg-amber-500 rounded-full transition-all" style={{ width: `${pet.hunger}%` }} />
              </div>
            </div>
            <div className="rounded-lg border border-pink-700/30 bg-pink-900/10 p-4 text-center">
              <div className="text-2xl font-bold text-pink-400 font-mono">{pet.happiness}</div>
              <div className="text-[11px] text-pink-400/60 uppercase tracking-wider mt-1">Happiness</div>
              <div className="w-full h-1.5 bg-raised rounded-full mt-2 overflow-hidden">
                <div className="h-full bg-pink-500 rounded-full transition-all" style={{ width: `${pet.happiness}%` }} />
              </div>
            </div>
            <div className="rounded-lg border border-edge bg-surface p-4 text-center">
              <div className="text-2xl font-bold text-fg font-mono">{pet.total_feeds}</div>
              <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">Total Feeds</div>
            </div>
          </div>

          {/* Feed message */}
          {feedMsg && (
            <div className="rounded-lg border border-green-700/30 bg-green-900/10 p-3 text-center text-sm text-green-400 animate-pulse">
              {feedMsg}
            </div>
          )}

          {/* Feed buttons */}
          <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
            <p className="text-xs text-subtle font-medium uppercase tracking-wider">Feed your bot</p>
            <div className="grid grid-cols-3 gap-3">
              {FOOD_TYPES.map(f => (
                <button
                  key={f.key}
                  onClick={() => feed.mutate(f.key)}
                  disabled={feed.isPending}
                  className={`flex flex-col items-center gap-2 p-4 rounded-lg ${f.color} hover:opacity-90 transition-all text-white disabled:opacity-50`}
                >
                  <span className="text-3xl">{f.emoji}</span>
                  <span className="text-sm font-medium">{f.label}</span>
                  <span className="text-[11px] opacity-70">{f.desc}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Pet button */}
          <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
            <p className="text-xs text-subtle font-medium uppercase tracking-wider">Show affection</p>
            <button
              onClick={() => petMut.mutate()}
              disabled={petMut.isPending}
              className="w-full p-4 rounded-lg bg-pink-700 hover:bg-pink-600 text-white transition-all disabled:opacity-50 flex items-center justify-center gap-3"
            >
              <span className="text-3xl">{'\u{1F49C}'}</span>
              <div>
                <div className="text-sm font-medium">Pet your bot</div>
                <div className="text-[11px] opacity-70">+20 happiness</div>
              </div>
            </button>
          </div>

          {/* Last interactions */}
          <div className="rounded-lg border border-edge bg-surface p-4 space-y-2">
            <p className="text-xs text-subtle font-medium uppercase tracking-wider">History</p>
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <span className="text-muted">Last fed:</span>{' '}
                <span className="text-fg font-mono text-xs">{pet.last_fed_at ?? 'Never'}</span>
              </div>
              <div>
                <span className="text-muted">Last pet:</span>{' '}
                <span className="text-fg font-mono text-xs">{pet.last_pet_at ?? 'Never'}</span>
              </div>
              <div>
                <span className="text-muted">Total feeds:</span>{' '}
                <span className="text-fg font-mono">{pet.total_feeds}</span>
              </div>
              <div>
                <span className="text-muted">Total pets:</span>{' '}
                <span className="text-fg font-mono">{pet.total_pets}</span>
              </div>
            </div>
          </div>
        </>
      )}

      {tab === 'tokens' && (
        <>
          {/* Token stats */}
          {tokens && (
            <>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                <div className="rounded-lg border border-cyan-700/30 bg-cyan-900/10 p-4 text-center">
                  <div className="text-2xl font-bold text-cyan-400 font-mono">
                    {tokens.today_total.toLocaleString()}
                  </div>
                  <div className="text-[11px] text-cyan-400/60 uppercase tracking-wider mt-1">Today</div>
                </div>
                <div className="rounded-lg border border-edge bg-surface p-4 text-center">
                  <div className="text-2xl font-bold text-fg font-mono">
                    {tokens.total_tokens.toLocaleString()}
                  </div>
                  <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">All Time</div>
                </div>
                <div className="rounded-lg border border-edge bg-surface p-4 text-center">
                  <div className="text-lg font-bold text-fg font-mono">
                    {tokens.total_input.toLocaleString()}
                  </div>
                  <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">Input Tokens</div>
                </div>
                <div className="rounded-lg border border-edge bg-surface p-4 text-center">
                  <div className="text-lg font-bold text-fg font-mono">
                    {tokens.total_output.toLocaleString()}
                  </div>
                  <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">Output Tokens</div>
                </div>
              </div>

              {/* Daily chart */}
              {tokens.daily.length > 0 && (
                <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
                  <p className="text-xs text-subtle font-medium uppercase tracking-wider">Daily Usage (30 days)</p>
                  <div className="flex items-end gap-1 h-32">
                    {tokens.daily.map(d => {
                      const total = d.input_tokens + d.output_tokens;
                      const maxDay = Math.max(...tokens.daily.map(x => x.input_tokens + x.output_tokens), 1);
                      const height = (total / maxDay) * 100;
                      const inputPct = total > 0 ? (d.input_tokens / total) * 100 : 50;
                      return (
                        <div key={d.date} className="flex-1 flex flex-col justify-end" title={`${d.date}: ${total.toLocaleString()} tokens`}>
                          <div className="rounded-t overflow-hidden" style={{ height: `${height}%` }}>
                            <div className="bg-cyan-500 w-full" style={{ height: `${inputPct}%` }} />
                            <div className="bg-purple-500 w-full" style={{ height: `${100 - inputPct}%` }} />
                          </div>
                        </div>
                      );
                    })}
                  </div>
                  <div className="flex items-center gap-4 text-[10px] text-subtle">
                    <span className="flex items-center gap-1"><span className="w-2 h-2 rounded bg-cyan-500" />Input</span>
                    <span className="flex items-center gap-1"><span className="w-2 h-2 rounded bg-purple-500" />Output</span>
                  </div>
                </div>
              )}

              {/* Manual token recording */}
              <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
                <p className="text-xs text-subtle font-medium uppercase tracking-wider">Record Token Usage</p>
                <div className="grid grid-cols-3 gap-3">
                  <div>
                    <label className="text-[11px] text-dim">Input Tokens</label>
                    <input
                      type="number"
                      value={manualInput}
                      onChange={e => setManualInput(e.target.value)}
                      placeholder="0"
                      className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg font-mono"
                    />
                  </div>
                  <div>
                    <label className="text-[11px] text-dim">Output Tokens</label>
                    <input
                      type="number"
                      value={manualOutput}
                      onChange={e => setManualOutput(e.target.value)}
                      placeholder="0"
                      className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg font-mono"
                    />
                  </div>
                  <div>
                    <label className="text-[11px] text-dim">Model</label>
                    <select
                      value={manualModel}
                      onChange={e => setManualModel(e.target.value)}
                      className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg"
                    >
                      <option value="claude-opus-4-6">Opus 4.6</option>
                      <option value="claude-sonnet-4-6">Sonnet 4.6</option>
                      <option value="claude-haiku-4-5">Haiku 4.5</option>
                      <option value="other">Other</option>
                    </select>
                  </div>
                </div>
                <button
                  onClick={() => recordMut.mutate()}
                  disabled={recordMut.isPending || (!manualInput && !manualOutput)}
                  className="px-4 py-1.5 text-sm bg-cyan-700 hover:bg-cyan-600 text-white rounded-md disabled:opacity-50"
                >
                  {recordMut.isPending ? 'Recording...' : 'Record'}
                </button>
              </div>

              {/* Recent entries */}
              {tokens.recent.length > 0 && (
                <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
                  <p className="text-xs text-subtle font-medium uppercase tracking-wider">Recent Usage</p>
                  <div className="space-y-1">
                    {tokens.recent.map(e => (
                      <div key={e.id} className="flex items-center justify-between text-xs py-1.5 border-b border-edge last:border-0">
                        <div className="flex items-center gap-3">
                          <span className="font-mono text-fg">{(e.input_tokens + e.output_tokens).toLocaleString()}</span>
                          <span className="text-subtle">({e.input_tokens.toLocaleString()} in / {e.output_tokens.toLocaleString()} out)</span>
                        </div>
                        <div className="flex items-center gap-3">
                          <span className="text-dim">{e.model}</span>
                          <span className="font-mono text-dim">{e.recorded_at}</span>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}
