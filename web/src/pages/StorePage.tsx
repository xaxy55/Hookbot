import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getStore, buyItem, getDevices, getGamificationStats } from '../api/client';
import type { StoreItemDef } from '../api/client';

const RARITY_COLORS: Record<string, string> = {
  starter:   'border-gray-600 bg-gray-800/30',
  common:    'border-green-700 bg-green-900/20',
  uncommon:  'border-blue-700 bg-blue-900/20',
  rare:      'border-purple-700 bg-purple-900/20',
  epic:      'border-amber-600 bg-amber-900/20',
  legendary: 'border-red-600 bg-red-900/20',
};

const RARITY_BADGE: Record<string, string> = {
  starter:   'bg-gray-700 text-gray-300',
  common:    'bg-green-800 text-green-300',
  uncommon:  'bg-blue-800 text-blue-300',
  rare:      'bg-purple-800 text-purple-300',
  epic:      'bg-amber-800 text-amber-300',
  legendary: 'bg-red-800 text-red-300',
};

const CATEGORIES = [
  { key: 'all', label: 'All' },
  { key: 'accessory', label: 'Accessories' },
  { key: 'title', label: 'Titles' },
  { key: 'animation', label: 'Animations' },
  { key: 'screensaver', label: 'Screensavers' },
];

export default function StorePage() {
  const qc = useQueryClient();
  const [category, setCategory] = useState('all');
  const [buyingId, setBuyingId] = useState<string | null>(null);

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [selectedDevice, setSelectedDevice] = useState<string>('');
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: store, isLoading } = useQuery({
    queryKey: ['store', deviceId],
    queryFn: () => getStore(deviceId),
    enabled: !!deviceId,
  });

  const { data: stats } = useQuery({
    queryKey: ['gamification-stats', deviceId],
    queryFn: () => getGamificationStats(deviceId),
    enabled: !!deviceId,
  });

  const purchase = useMutation({
    mutationFn: (itemId: string) => buyItem(itemId, deviceId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['store', deviceId] });
      qc.invalidateQueries({ queryKey: ['gamification-stats', deviceId] });
      setBuyingId(null);
    },
    onError: () => setBuyingId(null),
  });

  const items = store?.items ?? [];
  const filtered = category === 'all' ? items : items.filter(i => i.category === category);
  const owned = items.filter(i => i.owned);
  const balance = store?.balance ?? 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-fg">Store</h1>
          <p className="text-sm text-subtle">Spend your hard-earned XP on accessories, titles, and more</p>
        </div>
        <select
          value={selectedDevice}
          onChange={(e) => setSelectedDevice(e.target.value)}
          className="px-3 py-1.5 text-sm bg-inset border border-gray-700 rounded-md text-fg"
        >
          <option value="">Default device</option>
          {devices?.map((d) => (
            <option key={d.id} value={d.id}>{d.name}</option>
          ))}
        </select>
      </div>

      {/* Balance + Stats Bar */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <div className="rounded-lg border border-amber-600/30 bg-amber-900/10 p-4 text-center">
          <div className="text-2xl font-bold text-amber-400 font-mono">{balance.toLocaleString()}</div>
          <div className="text-[11px] text-amber-400/60 uppercase tracking-wider mt-1">XP Balance</div>
        </div>
        <div className="rounded-lg border border-edge bg-surface p-4 text-center">
          <div className="text-2xl font-bold text-fg font-mono">{stats?.level ?? 0}</div>
          <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">Level</div>
        </div>
        <div className="rounded-lg border border-edge bg-surface p-4 text-center">
          <div className="text-2xl font-bold text-fg font-mono">{owned.length}</div>
          <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">Owned</div>
        </div>
        <div className="rounded-lg border border-edge bg-surface p-4 text-center">
          <div className="text-2xl font-bold text-fg font-mono">{items.length}</div>
          <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">Total Items</div>
        </div>
      </div>

      {/* Category Filter */}
      <div className="flex gap-1.5">
        {CATEGORIES.map(c => (
          <button
            key={c.key}
            onClick={() => setCategory(c.key)}
            className={`px-3 py-1.5 text-xs rounded-md transition-colors ${
              category === c.key
                ? 'bg-brand/15 text-brand-fg border border-brand/30'
                : 'bg-inset text-subtle hover:text-fg border border-transparent'
            }`}
          >
            {c.label}
          </button>
        ))}
      </div>

      {/* Item Grid */}
      {isLoading ? (
        <p className="text-subtle text-sm">Loading store...</p>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {filtered.map(item => (
            <ItemCard
              key={item.id}
              item={item}
              buying={buyingId === item.id}
              onBuy={() => {
                setBuyingId(item.id);
                purchase.mutate(item.id);
              }}
              error={purchase.isError && buyingId === item.id ? purchase.error.message : undefined}
            />
          ))}
        </div>
      )}

      {filtered.length === 0 && !isLoading && (
        <div className="text-center py-12 rounded-lg border border-edge bg-surface">
          <p className="text-subtle">No items in this category</p>
        </div>
      )}
    </div>
  );
}

function ItemCard({ item, buying, onBuy, error }: {
  item: StoreItemDef;
  buying: boolean;
  onBuy: () => void;
  error?: string;
}) {
  const rarityColor = RARITY_COLORS[item.rarity] ?? RARITY_COLORS.common;
  const rarityBadge = RARITY_BADGE[item.rarity] ?? RARITY_BADGE.common;

  return (
    <div className={`rounded-lg border p-4 space-y-3 transition-all ${rarityColor} ${item.owned ? 'opacity-75' : ''}`}>
      {/* Top row: icon + name + rarity */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <span className="text-3xl">{item.icon}</span>
          <div>
            <h3 className="text-sm font-semibold text-fg">{item.name}</h3>
            <span className={`inline-block px-1.5 py-0.5 text-[10px] rounded font-medium uppercase tracking-wider ${rarityBadge}`}>
              {item.rarity}
            </span>
          </div>
        </div>
        <span className="text-[10px] text-subtle uppercase tracking-wider">{item.category}</span>
      </div>

      {/* Description */}
      <p className="text-xs text-muted leading-relaxed">{item.description}</p>

      {/* Price + action */}
      <div className="flex items-center justify-between pt-1">
        <div className="flex items-center gap-1.5">
          <span className="text-sm font-bold text-amber-400 font-mono">
            {item.price === 0 ? 'Free' : `${item.price.toLocaleString()} XP`}
          </span>
        </div>

        {item.owned ? (
          <span className="px-3 py-1.5 text-xs bg-green-800/40 text-green-400 rounded-md border border-green-700/30">
            Owned
          </span>
        ) : (
          <button
            onClick={onBuy}
            disabled={buying || !item.can_afford}
            className={`px-3 py-1.5 text-xs rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${
              item.can_afford
                ? 'bg-amber-600 hover:bg-amber-500 text-white'
                : 'bg-raised text-subtle'
            }`}
          >
            {buying ? 'Buying...' : item.can_afford ? 'Buy' : 'Not enough XP'}
          </button>
        )}
      </div>

      {error && <p className="text-[11px] text-red-400">{error}</p>}
    </div>
  );
}
