import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getCommunityPlugins, publishPlugin, installPlugin, uninstallPlugin, ratePlugin,
  getDevices,
  type CommunityPlugin,
} from '../api/client';

const CATEGORIES = [
  { key: 'all', label: 'All' },
  { key: 'utility', label: 'Utilities' },
  { key: 'integration', label: 'Integrations' },
  { key: 'theme', label: 'Themes' },
  { key: 'automation', label: 'Automation' },
];

const SORT_OPTIONS = [
  { key: 'newest', label: 'Newest' },
  { key: 'popular', label: 'Most Popular' },
  { key: 'rating', label: 'Top Rated' },
];

export default function CommunityStorePage() {
  const qc = useQueryClient();
  const [category, setCategory] = useState('all');
  const [sort, setSort] = useState('newest');
  const [search, setSearch] = useState('');
  const [showPublish, setShowPublish] = useState(false);

  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [selectedDevice, setSelectedDevice] = useState('');
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: plugins, isLoading } = useQuery({
    queryKey: ['community-plugins', deviceId, category, sort, search],
    queryFn: () => getCommunityPlugins({ deviceId, category, search: search || undefined, sort }),
  });

  const install = useMutation({
    mutationFn: (pluginId: string) => installPlugin(pluginId, deviceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['community-plugins'] }),
  });

  const uninstall = useMutation({
    mutationFn: (pluginId: string) => uninstallPlugin(pluginId, deviceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['community-plugins'] }),
  });

  const rate = useMutation({
    mutationFn: ({ pluginId, stars }: { pluginId: string; stars: number }) => ratePlugin(pluginId, stars, deviceId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['community-plugins'] }),
  });

  const installedCount = plugins?.filter(p => p.installed).length ?? 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-fg">Community Plugins</h1>
          <p className="text-sm text-subtle">Browse and install community-created plugins for your Hookbot</p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setShowPublish(!showPublish)}
            className="px-3 py-1.5 text-sm bg-brand/15 text-brand-fg border border-brand/30 rounded-md hover:bg-brand/25 transition-colors"
          >
            + Publish Plugin
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
        <PublishForm
          onPublish={() => {
            setShowPublish(false);
            qc.invalidateQueries({ queryKey: ['community-plugins'] });
          }}
          onCancel={() => setShowPublish(false)}
        />
      )}

      {/* Stats bar */}
      <div className="grid grid-cols-3 gap-3">
        <div className="rounded-lg border border-edge bg-surface p-4 text-center">
          <div className="text-2xl font-bold text-fg font-mono">{plugins?.length ?? 0}</div>
          <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">Available</div>
        </div>
        <div className="rounded-lg border border-green-700/30 bg-green-900/10 p-4 text-center">
          <div className="text-2xl font-bold text-green-400 font-mono">{installedCount}</div>
          <div className="text-[11px] text-green-400/60 uppercase tracking-wider mt-1">Installed</div>
        </div>
        <div className="rounded-lg border border-edge bg-surface p-4 text-center">
          <div className="text-2xl font-bold text-fg font-mono">
            {plugins?.reduce((sum, p) => sum + p.downloads, 0)?.toLocaleString() ?? 0}
          </div>
          <div className="text-[11px] text-subtle uppercase tracking-wider mt-1">Total Downloads</div>
        </div>
      </div>

      {/* Search + filters */}
      <div className="flex items-center gap-3">
        <input
          type="text"
          placeholder="Search plugins..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg placeholder:text-dim"
        />
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

      {/* Category filter */}
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

      {/* Plugin grid */}
      {isLoading ? (
        <p className="text-subtle text-sm">Loading plugins...</p>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {plugins?.map(plugin => (
            <PluginCard
              key={plugin.id}
              plugin={plugin}
              onInstall={() => install.mutate(plugin.id)}
              onUninstall={() => uninstall.mutate(plugin.id)}
              onRate={(stars) => rate.mutate({ pluginId: plugin.id, stars })}
              installing={install.isPending}
            />
          ))}
        </div>
      )}

      {plugins?.length === 0 && !isLoading && (
        <div className="text-center py-12 rounded-lg border border-edge bg-surface">
          <p className="text-2xl mb-2">📦</p>
          <p className="text-subtle">No plugins yet. Be the first to publish one!</p>
        </div>
      )}
    </div>
  );
}

function PluginCard({ plugin, onInstall, onUninstall, onRate, installing }: {
  plugin: CommunityPlugin;
  onInstall: () => void;
  onUninstall: () => void;
  onRate: (stars: number) => void;
  installing: boolean;
}) {
  const [hoverStar, setHoverStar] = useState(0);

  return (
    <div className={`rounded-lg border p-4 space-y-3 transition-all ${
      plugin.installed
        ? 'border-green-700/40 bg-green-900/10'
        : 'border-edge bg-surface hover:border-edge'
    }`}>
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h3 className="text-sm font-semibold text-fg">{plugin.name}</h3>
          <p className="text-[11px] text-subtle">
            by {plugin.author} &middot; v{plugin.version}
          </p>
        </div>
        <span className="px-1.5 py-0.5 text-[10px] rounded bg-inset text-subtle uppercase tracking-wider">
          {plugin.category}
        </span>
      </div>

      {/* Description */}
      <p className="text-xs text-muted leading-relaxed line-clamp-2">{plugin.description}</p>

      {/* Tags */}
      {plugin.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {plugin.tags.slice(0, 3).map(tag => (
            <span key={tag} className="px-1.5 py-0.5 text-[10px] rounded bg-inset text-dim">
              {tag}
            </span>
          ))}
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
                star <= (hoverStar || Math.round(plugin.rating_avg))
                  ? 'text-amber-400'
                  : 'text-gray-600'
              }>
                ★
              </span>
            </button>
          ))}
        </div>
        <span className="text-[11px] text-subtle">
          {plugin.rating_avg > 0 ? plugin.rating_avg.toFixed(1) : '—'} ({plugin.rating_count})
        </span>
      </div>

      {/* Footer: downloads + action */}
      <div className="flex items-center justify-between pt-1">
        <span className="text-[11px] text-dim">{plugin.downloads.toLocaleString()} downloads</span>

        {plugin.installed ? (
          <button
            onClick={onUninstall}
            className="px-3 py-1.5 text-xs rounded-md bg-red-900/30 text-red-400 border border-red-700/30 hover:bg-red-900/50 transition-colors"
          >
            Uninstall
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

function PublishForm({ onPublish, onCancel }: { onPublish: () => void; onCancel: () => void }) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [author, setAuthor] = useState('');
  const [category, setCategory] = useState('utility');
  const [tags, setTags] = useState('');
  const [error, setError] = useState('');

  const publish = useMutation({
    mutationFn: () => publishPlugin({
      name,
      description,
      author: author || undefined,
      category,
      tags: tags ? tags.split(',').map(t => t.trim()).filter(Boolean) : undefined,
    }),
    onSuccess: onPublish,
    onError: (e) => setError(e.message),
  });

  return (
    <div className="rounded-lg border border-brand/30 bg-brand/5 p-5 space-y-4">
      <h2 className="text-sm font-semibold text-fg">Publish a Plugin</h2>

      <div className="grid grid-cols-2 gap-3">
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
      </div>

      <div>
        <label className="text-[11px] text-subtle uppercase tracking-wider">Description *</label>
        <textarea value={description} onChange={e => setDescription(e.target.value)} rows={2}
          className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg resize-none" />
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="text-[11px] text-subtle uppercase tracking-wider">Category</label>
          <select value={category} onChange={e => setCategory(e.target.value)}
            className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg">
            <option value="utility">Utility</option>
            <option value="integration">Integration</option>
            <option value="theme">Theme</option>
            <option value="automation">Automation</option>
          </select>
        </div>
        <div>
          <label className="text-[11px] text-subtle uppercase tracking-wider">Tags (comma-separated)</label>
          <input value={tags} onChange={e => setTags(e.target.value)} placeholder="slack, webhook"
            className="w-full mt-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg placeholder:text-dim" />
        </div>
      </div>

      {error && <p className="text-[11px] text-red-400">{error}</p>}

      <div className="flex gap-2 pt-1">
        <button
          onClick={() => publish.mutate()}
          disabled={!name || !description || publish.isPending}
          className="px-4 py-1.5 text-sm bg-brand/15 text-brand-fg border border-brand/30 rounded-md hover:bg-brand/25 transition-colors disabled:opacity-50"
        >
          {publish.isPending ? 'Publishing...' : 'Publish'}
        </button>
        <button onClick={onCancel} className="px-4 py-1.5 text-sm text-subtle hover:text-fg transition-colors">
          Cancel
        </button>
      </div>
    </div>
  );
}
