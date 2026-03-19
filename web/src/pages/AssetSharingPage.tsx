import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getSharedAssets, publishAsset, installAsset, uninstallAsset, rateAsset,
  getDevices,
  type SharedAsset,
} from '../api/client';

const ASSET_TYPES = [
  { key: 'all', label: 'All', icon: '🎨' },
  { key: 'avatar', label: 'Avatars', icon: '😎' },
  { key: 'animation', label: 'Animations', icon: '🎬' },
  { key: 'screensaver', label: 'Screensavers', icon: '🖥️' },
];

const SORT_OPTIONS = [
  { key: 'newest', label: 'Newest' },
  { key: 'popular', label: 'Most Downloaded' },
  { key: 'rating', label: 'Top Rated' },
  { key: 'verified', label: 'Verified First' },
];

const TYPE_COLORS: Record<string, string> = {
  avatar: 'border-purple-700/40 bg-purple-900/10',
  animation: 'border-cyan-700/40 bg-cyan-900/10',
  screensaver: 'border-amber-700/40 bg-amber-900/10',
};

const TYPE_BADGES: Record<string, string> = {
  avatar: 'bg-purple-800 text-purple-300',
  animation: 'bg-cyan-800 text-cyan-300',
  screensaver: 'bg-amber-800 text-amber-300',
};

export default function AssetSharingPage() {
  const qc = useQueryClient();
  const [assetType, setAssetType] = useState('all');
  const [sort, setSort] = useState('newest');
  const [search, setSearch] = useState('');
  const [showPublish, setShowPublish] = useState(false);
  const [verifiedOnly, setVerifiedOnly] = useState(false);

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [selectedDevice, setSelectedDevice] = useState('');
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: assets, isLoading } = useQuery({
    queryKey: ['shared-assets', deviceId, assetType, sort, search],
    queryFn: () => getSharedAssets({ deviceId, assetType, search: search || undefined, sort }),
  });

  const install = useMutation({
    mutationFn: (assetId: string) => installAsset(assetId, deviceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['shared-assets'] }),
  });

  const uninstallMut = useMutation({
    mutationFn: (assetId: string) => uninstallAsset(assetId, deviceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['shared-assets'] }),
  });

  const rate = useMutation({
    mutationFn: ({ assetId, stars }: { assetId: string; stars: number }) => rateAsset(assetId, stars, deviceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['shared-assets'] }),
  });

  const filteredAssets = verifiedOnly ? assets?.filter(a => a.verified) : assets;
  const verifiedCount = assets?.filter(a => a.verified).length ?? 0;
  const avatarCount = assets?.filter(a => a.asset_type === 'avatar').length ?? 0;
  const animCount = assets?.filter(a => a.asset_type === 'animation').length ?? 0;
  const ssCount = assets?.filter(a => a.asset_type === 'screensaver').length ?? 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-fg">Shared Assets</h1>
          <p className="text-sm text-subtle">Browse and share community avatars, animations, and screensavers</p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setShowPublish(!showPublish)}
            className="px-3 py-1.5 text-sm bg-brand/15 text-brand-fg border border-brand/30 rounded-md hover:bg-brand/25 transition-colors"
          >
            + Share Asset
          </button>
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
      </div>

      {/* Publish form */}
      {showPublish && (
        <PublishAssetForm
          onPublish={() => {
            setShowPublish(false);
            qc.invalidateQueries({ queryKey: ['shared-assets'] });
          }}
          onCancel={() => setShowPublish(false)}
        />
      )}

      {/* Stats */}
      <div className="grid grid-cols-4 gap-3">
        <div className="rounded-lg border border-edge bg-surface p-4 text-center">
          <div className="text-2xl font-bold text-fg font-mono">{assets?.length ?? 0}</div>
          <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">Total</div>
        </div>
        <div className="rounded-lg border border-purple-700/30 bg-purple-900/10 p-4 text-center">
          <div className="text-2xl font-bold text-purple-400 font-mono">{avatarCount}</div>
          <div className="text-[11px] text-purple-400/60 uppercase tracking-wider mt-1">Avatars</div>
        </div>
        <div className="rounded-lg border border-cyan-700/30 bg-cyan-900/10 p-4 text-center">
          <div className="text-2xl font-bold text-cyan-400 font-mono">{animCount}</div>
          <div className="text-[11px] text-cyan-400/60 uppercase tracking-wider mt-1">Animations</div>
        </div>
        <div className="rounded-lg border border-amber-700/30 bg-amber-900/10 p-4 text-center">
          <div className="text-2xl font-bold text-amber-400 font-mono">{ssCount}</div>
          <div className="text-[11px] text-amber-400/60 uppercase tracking-wider mt-1">Screensavers</div>
        </div>
      </div>

      {/* Search + sort */}
      <div className="flex items-center gap-3">
        <input
          type="text"
          placeholder="Search assets..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg placeholder:text-dim"
        />
        <button
          onClick={() => setVerifiedOnly(!verifiedOnly)}
          className={`px-3 py-1.5 text-sm rounded-md border transition-colors flex items-center gap-1.5 ${
            verifiedOnly
              ? 'bg-blue-600/15 text-blue-400 border-blue-600/30'
              : 'bg-inset text-subtle border-edge hover:text-fg'
          }`}
        >
          <svg className="w-3.5 h-3.5" viewBox="0 0 20 20" fill="currentColor">
            <path fillRule="evenodd" d="M6.267 3.455a3.066 3.066 0 001.745-.723 3.066 3.066 0 013.976 0 3.066 3.066 0 001.745.723 3.066 3.066 0 012.812 2.812c.051.643.304 1.254.723 1.745a3.066 3.066 0 010 3.976 3.066 3.066 0 00-.723 1.745 3.066 3.066 0 01-2.812 2.812 3.066 3.066 0 00-1.745.723 3.066 3.066 0 01-3.976 0 3.066 3.066 0 00-1.745-.723 3.066 3.066 0 01-2.812-2.812 3.066 3.066 0 00-.723-1.745 3.066 3.066 0 010-3.976 3.066 3.066 0 00.723-1.745 3.066 3.066 0 012.812-2.812zm7.44 5.252a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
          </svg>
          Verified ({verifiedCount})
        </button>
        <select
          value={sort}
          onChange={(e) => setSort(e.target.value)}
          className="px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg"
        >
          {SORT_OPTIONS.map(s => (
            <option key={s.key} value={s.key}>{s.label}</option>
          ))}
        </select>
      </div>

      {/* Type filter */}
      <div className="flex gap-1.5">
        {ASSET_TYPES.map(t => (
          <button
            key={t.key}
            onClick={() => setAssetType(t.key)}
            className={`px-3 py-1.5 text-xs rounded-md transition-colors ${
              assetType === t.key
                ? 'bg-brand/15 text-brand-fg border border-brand/30'
                : 'bg-inset text-subtle hover:text-fg border border-transparent'
            }`}
          >
            {t.icon} {t.label}
          </button>
        ))}
      </div>

      {/* Asset grid */}
      {isLoading ? (
        <p className="text-subtle text-sm">Loading assets...</p>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredAssets?.map(asset => (
            <AssetCard
              key={asset.id}
              asset={asset}
              onInstall={() => install.mutate(asset.id)}
              onUninstall={() => uninstallMut.mutate(asset.id)}
              onRate={(stars) => rate.mutate({ assetId: asset.id, stars })}
              installing={install.isPending}
            />
          ))}
        </div>
      )}

      {filteredAssets?.length === 0 && !isLoading && (
        <div className="text-center py-12 rounded-lg border border-edge bg-surface">
          <p className="text-2xl mb-2">🎨</p>
          <p className="text-subtle">No shared assets yet. Share your creations with the community!</p>
        </div>
      )}
    </div>
  );
}

function AssetCard({ asset, onInstall, onUninstall, onRate, installing }: {
  asset: SharedAsset;
  onInstall: () => void;
  onUninstall: () => void;
  onRate: (stars: number) => void;
  installing: boolean;
}) {
  const [hoverStar, setHoverStar] = useState(0);
  const typeColor = TYPE_COLORS[asset.asset_type] ?? 'border-edge bg-surface';
  const typeBadge = TYPE_BADGES[asset.asset_type] ?? 'bg-gray-700 text-gray-300';

  const typeIcon = asset.asset_type === 'avatar' ? '😎' : asset.asset_type === 'animation' ? '🎬' : '🖥️';

  return (
    <div className={`rounded-lg border p-4 space-y-3 transition-all ${typeColor} ${
      asset.installed ? 'ring-1 ring-green-600/40' : ''
    }`}>
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <span className="text-2xl">{typeIcon}</span>
          <div>
            <h3 className="text-sm font-semibold text-fg">{asset.name}</h3>
            <p className="text-[11px] text-subtle flex items-center gap-1">
              by {asset.author}
              {asset.verified && (
                <svg className="w-3.5 h-3.5 text-blue-400 inline-block" viewBox="0 0 20 20" fill="currentColor" title="Verified Publisher">
                  <path fillRule="evenodd" d="M6.267 3.455a3.066 3.066 0 001.745-.723 3.066 3.066 0 013.976 0 3.066 3.066 0 001.745.723 3.066 3.066 0 012.812 2.812c.051.643.304 1.254.723 1.745a3.066 3.066 0 010 3.976 3.066 3.066 0 00-.723 1.745 3.066 3.066 0 01-2.812 2.812 3.066 3.066 0 00-1.745.723 3.066 3.066 0 01-3.976 0 3.066 3.066 0 00-1.745-.723 3.066 3.066 0 01-2.812-2.812 3.066 3.066 0 00-.723-1.745 3.066 3.066 0 010-3.976 3.066 3.066 0 00.723-1.745 3.066 3.066 0 012.812-2.812zm7.44 5.252a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                </svg>
              )}
            </p>
          </div>
        </div>
        <span className={`px-1.5 py-0.5 text-[10px] rounded font-medium uppercase tracking-wider ${typeBadge}`}>
          {asset.asset_type}
        </span>
      </div>

      {/* Description */}
      <p className="text-xs text-muted leading-relaxed line-clamp-2">{asset.description}</p>

      {/* Payload preview */}
      {asset.payload && Object.keys(asset.payload).length > 0 && (
        <div className="px-2 py-1.5 rounded bg-black/20 border border-edge">
          <p className="text-[10px] text-dim font-mono truncate">
            {JSON.stringify(asset.payload).slice(0, 80)}...
          </p>
        </div>
      )}

      {/* Rating */}
      <div className="flex items-center gap-2">
        <div className="flex gap-0.5">
          {[1, 2, 3, 4, 5].map(star => (
            <button
              key={star}
              onMouseEnter={() => setHoverStar(star)}
              onMouseLeave={() => setHoverStar(0)}
              onClick={() => onRate(star)}
              className="text-sm transition-colors"
            >
              <span className={
                star <= (hoverStar || Math.round(asset.rating_avg))
                  ? 'text-amber-400'
                  : 'text-gray-600'
              }>
                ★
              </span>
            </button>
          ))}
        </div>
        <span className="text-[11px] text-subtle">
          {asset.rating_avg > 0 ? asset.rating_avg.toFixed(1) : '—'} ({asset.rating_count})
        </span>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between pt-1">
        <span className="text-[11px] text-dim">{asset.downloads.toLocaleString()} installs</span>

        {asset.installed ? (
          <button
            onClick={onUninstall}
            className="px-3 py-1.5 text-xs rounded-md bg-red-900/30 text-red-400 border border-red-700/30 hover:bg-red-900/50 transition-colors"
          >
            Remove
          </button>
        ) : (
          <button
            onClick={onInstall}
            disabled={installing}
            className="px-3 py-1.5 text-xs rounded-md bg-green-700 hover:bg-green-600 text-white transition-colors disabled:opacity-50"
          >
            {installing ? 'Installing...' : 'Install'}
          </button>
        )}
      </div>
    </div>
  );
}

function PublishAssetForm({ onPublish, onCancel }: { onPublish: () => void; onCancel: () => void }) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [author, setAuthor] = useState('');
  const [assetType, setAssetType] = useState('avatar');
  const [payloadJson, setPayloadJson] = useState('{}');
  const [error, setError] = useState('');

  const publish = useMutation({
    mutationFn: () => {
      let payload: Record<string, unknown>;
      try {
        payload = JSON.parse(payloadJson);
      } catch {
        throw new Error('Invalid JSON in payload');
      }
      return publishAsset({
        name,
        description: description || undefined,
        author: author || undefined,
        asset_type: assetType,
        payload,
      });
    },
    onSuccess: onPublish,
    onError: (e) => setError(e.message),
  });

  return (
    <div className="rounded-lg border border-brand/30 bg-brand/5 p-5 space-y-4">
      <h2 className="text-sm font-semibold text-fg">Share an Asset</h2>

      <div className="grid grid-cols-3 gap-3">
        <div>
          <label className="text-[11px] text-subtle uppercase tracking-wider">Name *</label>
          <input value={name} onChange={e => setName(e.target.value)}
            className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg" />
        </div>
        <div>
          <label className="text-[11px] text-subtle uppercase tracking-wider">Author</label>
          <input value={author} onChange={e => setAuthor(e.target.value)} placeholder="anonymous"
            className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg placeholder:text-dim" />
        </div>
        <div>
          <label className="text-[11px] text-subtle uppercase tracking-wider">Type *</label>
          <select value={assetType} onChange={e => setAssetType(e.target.value)}
            className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg">
            <option value="avatar">Avatar</option>
            <option value="animation">Animation</option>
            <option value="screensaver">Screensaver</option>
          </select>
        </div>
      </div>

      <div>
        <label className="text-[11px] text-subtle uppercase tracking-wider">Description</label>
        <textarea value={description} onChange={e => setDescription(e.target.value)} rows={2}
          className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg resize-none" />
      </div>

      <div>
        <label className="text-[11px] text-subtle uppercase tracking-wider">
          Payload JSON * <span className="normal-case text-dim">(avatar params, keyframes, or config)</span>
        </label>
        <textarea value={payloadJson} onChange={e => setPayloadJson(e.target.value)} rows={4}
          className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg font-mono resize-none"
          placeholder='{"eyeX": 0, "eyeY": 0, "mouthCurve": 0.3}' />
      </div>

      {error && <p className="text-[11px] text-red-400">{error}</p>}

      <div className="flex gap-2 pt-1">
        <button
          onClick={() => publish.mutate()}
          disabled={!name || publish.isPending}
          className="px-4 py-1.5 text-sm bg-brand/15 text-brand-fg border border-brand/30 rounded-md hover:bg-brand/25 transition-colors disabled:opacity-50"
        >
          {publish.isPending ? 'Sharing...' : 'Share'}
        </button>
        <button onClick={onCancel} className="px-4 py-1.5 text-sm text-subtle hover:text-fg transition-colors">
          Cancel
        </button>
      </div>
    </div>
  );
}
