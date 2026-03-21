import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getBuddies, createBuddy, acceptBuddy, deleteBuddy,
  getRaids, createRaid,
  getSharedStreaks, createSharedStreak, deleteSharedStreak,
  getPresence, updatePresence,
  getReactions, sendReaction,
  getDevices,
} from '../api/client';
import type { Buddy, Raid, SharedStreak, CodingPresence, SocialReaction } from '../api/client';

type Tab = 'buddies' | 'raids' | 'streaks' | 'presence' | 'reactions';

const REACTION_EMOJIS: Record<string, string> = {
  fireworks: '\uD83C\uDF86', skull: '\uD83D\uDC80', heart: '\u2764\uFE0F', fire: '\uD83D\uDD25',
  rocket: '\uD83D\uDE80', party: '\uD83C\uDF89', thumbsup: '\uD83D\uDC4D', clap: '\uD83D\uDC4F',
  eyes: '\uD83D\uDC40', '100': '\uD83D\uDCAF', bug: '\uD83D\uDC1B', ship: '\uD83D\uDEA2',
};

export default function SocialPage() {
  const [tab, setTab] = useState<Tab>('buddies');

  const tabs: { key: Tab; label: string }[] = [
    { key: 'buddies', label: 'Buddies' },
    { key: 'raids', label: 'Raids' },
    { key: 'streaks', label: 'Shared Streaks' },
    { key: 'presence', label: 'Live Coding' },
    { key: 'reactions', label: 'Reactions' },
  ];

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-bold text-fg">Social & Multiplayer</h1>

      <div className="flex gap-1 border-b border-edge pb-1">
        {tabs.map(t => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`px-3 py-1.5 text-sm rounded-t-lg transition-colors ${
              tab === t.key ? 'bg-brand/10 text-brand-fg border-b-2 border-brand' : 'text-muted hover:text-fg'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'buddies' && <BuddiesTab />}
      {tab === 'raids' && <RaidsTab />}
      {tab === 'streaks' && <SharedStreaksTab />}
      {tab === 'presence' && <PresenceTab />}
      {tab === 'reactions' && <ReactionsTab />}
    </div>
  );
}

function BuddiesTab() {
  const qc = useQueryClient();
  const { data: buddies } = useQuery({ queryKey: ['buddies'], queryFn: getBuddies, refetchInterval: 5000 });
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [deviceId, setDeviceId] = useState('');
  const [buddyDeviceId, setBuddyDeviceId] = useState('');

  const addMut = useMutation({
    mutationFn: () => createBuddy({ device_id: deviceId, buddy_device_id: buddyDeviceId }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ['buddies'] }); setDeviceId(''); setBuddyDeviceId(''); },
  });
  const acceptMut = useMutation({
    mutationFn: (id: string) => acceptBuddy(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['buddies'] }),
  });
  const removeMut = useMutation({
    mutationFn: (id: string) => deleteBuddy(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['buddies'] }),
  });

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
        <h3 className="text-sm font-medium text-fg">Pair Hookbots</h3>
        <div className="flex gap-2 items-end flex-wrap">
          <select value={deviceId} onChange={e => setDeviceId(e.target.value)}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg">
            <option value="">Your device...</option>
            {devices?.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
          <select value={buddyDeviceId} onChange={e => setBuddyDeviceId(e.target.value)}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg">
            <option value="">Buddy device...</option>
            {devices?.filter(d => d.id !== deviceId).map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
          <button onClick={() => addMut.mutate()} disabled={!deviceId || !buddyDeviceId}
            className="px-4 py-2 rounded-lg bg-brand text-white text-sm font-medium disabled:opacity-50 hover:bg-brand/80">
            Send Request
          </button>
        </div>
      </div>

      <div className="space-y-2">
        {buddies?.map((b: Buddy) => (
          <div key={b.id} className="rounded-lg border border-edge bg-surface p-4 flex items-center justify-between">
            <div>
              <span className="text-sm font-medium text-fg">{b.device_name || b.device_id}</span>
              <span className="text-muted mx-2">&harr;</span>
              <span className="text-sm font-medium text-fg">{b.buddy_device_name || b.buddy_device_id}</span>
              <span className={`ml-3 text-xs px-2 py-0.5 rounded-full ${
                b.status === 'accepted' ? 'bg-green-500/10 text-green-600 dark:text-green-400' : 'bg-yellow-500/10 text-yellow-600 dark:text-yellow-400'
              }`}>
                {b.status}
              </span>
              {b.mirror_mood && <span className="ml-2 text-xs text-muted">Mirror mood</span>}
            </div>
            <div className="flex gap-2">
              {b.status === 'pending' && (
                <button onClick={() => acceptMut.mutate(b.id)}
                  className="px-3 py-1 rounded-lg bg-green-600 text-white text-xs font-medium hover:bg-green-700">
                  Accept
                </button>
              )}
              <button onClick={() => removeMut.mutate(b.id)}
                className="px-3 py-1 rounded-lg bg-red-600/10 text-red-600 dark:text-red-400 text-xs font-medium hover:bg-red-600/20">
                Remove
              </button>
            </div>
          </div>
        ))}
        {buddies?.length === 0 && <p className="text-sm text-muted">No buddy pairings yet.</p>}
      </div>
    </div>
  );
}

function RaidsTab() {
  const qc = useQueryClient();
  const { data: raids } = useQuery({ queryKey: ['raids'], queryFn: getRaids, refetchInterval: 5000 });
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [msg, setMsg] = useState('');
  const [state, setState] = useState('happy');

  const raidMut = useMutation({
    mutationFn: () => createRaid({ from_device_id: from, to_device_id: to, message: msg || undefined, avatar_state: state }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ['raids'] }); setMsg(''); },
  });

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
        <h3 className="text-sm font-medium text-fg">Send a Raid</h3>
        <p className="text-xs text-muted">Send your avatar to visit a friend's hookbot for 30 seconds!</p>
        <div className="flex gap-2 items-end flex-wrap">
          <select value={from} onChange={e => setFrom(e.target.value)}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg">
            <option value="">From...</option>
            {devices?.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
          <select value={to} onChange={e => setTo(e.target.value)}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg">
            <option value="">To...</option>
            {devices?.filter(d => d.id !== from).map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
          <select value={state} onChange={e => setState(e.target.value)}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg">
            {['happy', 'excited', 'coding', 'thinking', 'error', 'sleeping'].map(s =>
              <option key={s} value={s}>{s}</option>
            )}
          </select>
          <input value={msg} onChange={e => setMsg(e.target.value)} placeholder="Message (optional)"
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg flex-1 min-w-[150px]" />
          <button onClick={() => raidMut.mutate()} disabled={!from || !to}
            className="px-4 py-2 rounded-lg bg-purple-600 text-white text-sm font-medium disabled:opacity-50 hover:bg-purple-700">
            Raid!
          </button>
        </div>
      </div>

      <div className="space-y-2">
        {raids?.map((r: Raid) => (
          <div key={r.id} className="rounded-lg border border-edge bg-surface p-4">
            <div className="flex items-center justify-between">
              <div className="text-sm">
                <span className="font-medium text-fg">{r.from_device_name || r.from_device_id}</span>
                <span className="text-muted mx-2">&rarr;</span>
                <span className="font-medium text-fg">{r.to_device_name || r.to_device_id}</span>
              </div>
              <span className={`text-xs px-2 py-0.5 rounded-full ${
                r.status === 'pending' ? 'bg-yellow-500/10 text-yellow-600' : 'bg-green-500/10 text-green-600'
              }`}>{r.status}</span>
            </div>
            {r.message && <p className="text-sm text-muted mt-1">"{r.message}"</p>}
            <div className="flex gap-4 text-xs text-dim mt-2">
              <span>State: {r.avatar_state}</span>
              <span>{r.duration_secs}s</span>
              <span>{new Date(r.created_at).toLocaleString()}</span>
            </div>
          </div>
        ))}
        {raids?.length === 0 && <p className="text-sm text-muted">No raids yet. Send one!</p>}
      </div>
    </div>
  );
}

function SharedStreaksTab() {
  const qc = useQueryClient();
  const { data: streaks } = useQuery({ queryKey: ['shared-streaks'], queryFn: getSharedStreaks, refetchInterval: 10000 });
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [name, setName] = useState('');
  const [selectedIds, setSelectedIds] = useState<string[]>([]);

  const createMut = useMutation({
    mutationFn: () => createSharedStreak({ name, device_ids: selectedIds }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ['shared-streaks'] }); setName(''); setSelectedIds([]); },
  });
  const removeMut = useMutation({
    mutationFn: (id: string) => deleteSharedStreak(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['shared-streaks'] }),
  });

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
        <h3 className="text-sm font-medium text-fg">Create Shared Streak Challenge</h3>
        <p className="text-xs text-muted">All participants must code daily to keep the streak alive!</p>
        <div className="flex gap-2 items-end flex-wrap">
          <input value={name} onChange={e => setName(e.target.value)} placeholder="Challenge name"
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg" />
          <select multiple value={selectedIds} onChange={e => setSelectedIds(Array.from(e.target.selectedOptions, o => o.value))}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg min-h-[60px]">
            {devices?.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
          <button onClick={() => createMut.mutate()} disabled={!name || selectedIds.length < 2}
            className="px-4 py-2 rounded-lg bg-brand text-white text-sm font-medium disabled:opacity-50 hover:bg-brand/80">
            Create
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {streaks?.map((s: SharedStreak) => (
          <div key={s.id} className="rounded-lg border border-edge bg-surface p-4">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-medium text-fg">{s.name}</span>
              <button onClick={() => removeMut.mutate(s.id)}
                className="text-xs text-red-500 hover:text-red-400">&times;</button>
            </div>
            <div className="flex gap-4 text-sm">
              <div className="text-center">
                <div className="text-2xl font-bold text-brand">{s.current_streak}</div>
                <div className="text-xs text-muted">Current</div>
              </div>
              <div className="text-center">
                <div className="text-2xl font-bold text-amber-500">{s.longest_streak}</div>
                <div className="text-xs text-muted">Best</div>
              </div>
            </div>
            <div className="text-xs text-dim mt-2">{s.device_ids.length} participants</div>
          </div>
        ))}
        {streaks?.length === 0 && <p className="text-sm text-muted col-span-2">No shared streaks yet.</p>}
      </div>
    </div>
  );
}

function PresenceTab() {
  const { data: presence } = useQuery({ queryKey: ['presence'], queryFn: getPresence, refetchInterval: 3000 });

  const coding = presence?.filter((p: CodingPresence) => p.is_coding) || [];
  const idle = presence?.filter((p: CodingPresence) => !p.is_coding) || [];

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted">See who's actively coding right now. Hookbots glow when friends are coding!</p>

      {coding.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-sm font-medium text-green-600 dark:text-green-400">Active Now</h3>
          {coding.map((p: CodingPresence) => (
            <div key={p.device_id} className="rounded-lg border border-green-500/30 bg-green-500/5 p-4 flex items-center gap-3">
              <div className="w-3 h-3 rounded-full bg-green-500 animate-pulse" />
              <div>
                <span className="text-sm font-medium text-fg">{p.device_name || p.device_id}</span>
                <span className="ml-2 text-xs text-muted">{p.current_state}</span>
              </div>
              {p.last_activity_at && (
                <span className="ml-auto text-xs text-dim">Last: {new Date(p.last_activity_at).toLocaleTimeString()}</span>
              )}
            </div>
          ))}
        </div>
      )}

      {idle.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-sm font-medium text-muted">Idle</h3>
          {idle.map((p: CodingPresence) => (
            <div key={p.device_id} className="rounded-lg border border-edge bg-surface p-4 flex items-center gap-3">
              <div className="w-3 h-3 rounded-full bg-gray-400" />
              <span className="text-sm text-muted">{p.device_name || p.device_id}</span>
              {p.last_activity_at && (
                <span className="ml-auto text-xs text-dim">Last: {new Date(p.last_activity_at).toLocaleTimeString()}</span>
              )}
            </div>
          ))}
        </div>
      )}

      {(!presence || presence.length === 0) && (
        <p className="text-sm text-muted">No presence data yet. Presence updates when devices send activity.</p>
      )}
    </div>
  );
}

function ReactionsTab() {
  const qc = useQueryClient();
  const { data: reactions } = useQuery({ queryKey: ['reactions'], queryFn: () => getReactions(50), refetchInterval: 5000 });
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [emoji, setEmoji] = useState('fireworks');

  const sendMut = useMutation({
    mutationFn: () => sendReaction({ from_device_id: from, to_device_id: to, reaction: emoji }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['reactions'] }),
  });

  return (
    <div className="space-y-4">
      <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
        <h3 className="text-sm font-medium text-fg">Send a Reaction</h3>
        <div className="flex gap-2 items-end flex-wrap">
          <select value={from} onChange={e => setFrom(e.target.value)}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg">
            <option value="">From...</option>
            {devices?.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
          <select value={to} onChange={e => setTo(e.target.value)}
            className="px-3 py-2 rounded-lg bg-inset border border-edge text-sm text-fg">
            <option value="">To...</option>
            {devices?.filter(d => d.id !== from).map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
          <div className="flex gap-1 flex-wrap">
            {Object.entries(REACTION_EMOJIS).map(([key, emo]) => (
              <button key={key} onClick={() => setEmoji(key)}
                className={`w-8 h-8 rounded-lg text-lg flex items-center justify-center transition-colors ${
                  emoji === key ? 'bg-brand/20 ring-2 ring-brand' : 'bg-inset hover:bg-inset/80'
                }`} title={key}>
                {emo}
              </button>
            ))}
          </div>
          <button onClick={() => sendMut.mutate()} disabled={!from || !to}
            className="px-4 py-2 rounded-lg bg-brand text-white text-sm font-medium disabled:opacity-50 hover:bg-brand/80">
            Send {REACTION_EMOJIS[emoji]}
          </button>
        </div>
      </div>

      <div className="space-y-2">
        {reactions?.map((r: SocialReaction) => (
          <div key={r.id} className="rounded-lg border border-edge bg-surface p-3 flex items-center gap-3">
            <span className="text-2xl">{REACTION_EMOJIS[r.reaction] || r.reaction}</span>
            <div className="text-sm">
              <span className="font-medium text-fg">{r.from_device_name || r.from_device_id}</span>
              <span className="text-muted mx-1">&rarr;</span>
              <span className="text-fg">{r.to_device_id}</span>
            </div>
            <span className="ml-auto text-xs text-dim">{new Date(r.created_at).toLocaleString()}</span>
          </div>
        ))}
        {reactions?.length === 0 && <p className="text-sm text-muted">No reactions yet.</p>}
      </div>
    </div>
  );
}
