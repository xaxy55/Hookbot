import { useEffect, useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getMe,
  getApiTokens,
  createApiToken,
  revokeApiToken,
  createPairingToken,
  updateAccount,
  apiOrigin,
} from '../api/client';
import QRCode from '../components/QRCode';

type Tab = 'profile' | 'tokens' | 'pair-phone';

interface ApiToken {
  id: string;
  name: string;
  token_preview: string;
  created_at: string;
  last_used_at: string | null;
}

export default function AccountPage() {
  const queryClient = useQueryClient();
  const [tab, setTab] = useState<Tab>('profile');
  const [newTokenName, setNewTokenName] = useState('');
  const [createdToken, setCreatedToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [editingName, setEditingName] = useState(false);
  const [nameValue, setNameValue] = useState('');

  const { data: me } = useQuery({
    queryKey: ['me'],
    queryFn: getMe,
  });

  const { data: tokens, isLoading: tokensLoading } = useQuery({
    queryKey: ['api-tokens'],
    queryFn: getApiTokens,
    enabled: tab === 'tokens',
  });

  const createTokenMut = useMutation({
    mutationFn: (name: string) => createApiToken({ name }),
    onSuccess: (data) => {
      setCreatedToken(data.token);
      setNewTokenName('');
      queryClient.invalidateQueries({ queryKey: ['api-tokens'] });
    },
  });

  const revokeTokenMut = useMutation({
    mutationFn: (id: string) => revokeApiToken(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-tokens'] });
    },
  });

  const updateAccountMut = useMutation({
    mutationFn: (data: { name?: string }) => updateAccount(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['me'] });
      setEditingName(false);
    },
  });

  // Pairing: the token is only ever held in memory here, and only until it expires.
  const [pairing, setPairing] = useState<{ token: string; server: string; expiresAt: number } | null>(null);
  const [secondsLeft, setSecondsLeft] = useState(0);

  const pairMut = useMutation({
    mutationFn: () => createPairingToken(),
    onSuccess: (data) => {
      setPairing({
        token: data.token,
        server: apiOrigin(),
        // Count down off the server's TTL rather than its clock, so a skewed
        // browser clock cannot show a code as live after the server killed it.
        expiresAt: Date.now() + data.expires_in_secs * 1000,
      });
    },
  });

  useEffect(() => {
    if (!pairing) return;
    const tick = () => setSecondsLeft(Math.max(0, Math.ceil((pairing.expiresAt - Date.now()) / 1000)));
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [pairing]);

  const pairingPayload = pairing
    ? `hookbot://pair?server=${encodeURIComponent(pairing.server)}&token=${encodeURIComponent(pairing.token)}`
    : '';
  const countdown = `${Math.floor(secondsLeft / 60)}:${String(secondsLeft % 60).padStart(2, '0')}`;

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const tabs: { key: Tab; label: string }[] = [
    { key: 'profile', label: 'Profile' },
    { key: 'tokens', label: 'API Tokens' },
    { key: 'pair-phone', label: 'Pair Phone' },
  ];

  return (
    <div>
      <h1 className="text-xl font-bold text-fg mb-6">Account</h1>

      {/* Tabs */}
      <div className="flex gap-1 mb-6 border-b border-edge">
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              tab === t.key
                ? 'border-indigo-500 text-fg'
                : 'border-transparent text-subtle hover:text-fg'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Profile Tab */}
      {tab === 'profile' && me && (
        <div className="space-y-6 max-w-lg">
          <div className="rounded-lg border border-edge bg-surface p-6">
            <h2 className="text-sm font-semibold text-fg mb-4">Account Info</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-xs text-subtle mb-1">Email</label>
                <div className="text-sm text-fg">{me.email}</div>
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Name</label>
                {editingName ? (
                  <div className="flex items-center gap-2">
                    <input
                      type="text"
                      value={nameValue}
                      onChange={(e) => setNameValue(e.target.value)}
                      className="flex-1 px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg focus:outline-none focus:ring-1 focus:ring-indigo-500"
                    />
                    <button
                      onClick={() => updateAccountMut.mutate({ name: nameValue })}
                      disabled={updateAccountMut.isPending}
                      className="px-3 py-1.5 text-xs bg-indigo-600 text-white rounded-md hover:bg-indigo-700 disabled:opacity-50"
                    >
                      Save
                    </button>
                    <button
                      onClick={() => setEditingName(false)}
                      className="px-3 py-1.5 text-xs border border-edge text-subtle rounded-md hover:bg-raised"
                    >
                      Cancel
                    </button>
                  </div>
                ) : (
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-fg">{me.name || '(not set)'}</span>
                    <button
                      onClick={() => {
                        setNameValue(me.name || '');
                        setEditingName(true);
                      }}
                      className="text-xs text-indigo-400 hover:text-indigo-300"
                    >
                      Edit
                    </button>
                  </div>
                )}
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">User ID</label>
                <div className="text-xs text-dim font-mono">{me.id}</div>
              </div>
              <div>
                <label className="block text-xs text-subtle mb-1">Member Since</label>
                <div className="text-sm text-fg">
                  {me.created_at ? new Date(me.created_at + 'Z').toLocaleDateString() : '-'}
                </div>
              </div>
            </div>
          </div>

          <div className="rounded-lg border border-edge bg-surface p-6">
            <h2 className="text-sm font-semibold text-fg mb-2">Default API Key</h2>
            <p className="text-xs text-subtle mb-3">
              This is your account-level API key. For finer control, create dedicated tokens in the API Tokens tab.
            </p>
            <div className="flex items-center gap-2">
              <code className="flex-1 px-3 py-2 text-xs bg-inset border border-edge rounded-md text-fg font-mono overflow-hidden text-ellipsis">
                {me.api_key}
              </code>
              <button
                onClick={() => handleCopy(me.api_key)}
                className="px-3 py-2 text-xs border border-edge text-subtle rounded-md hover:bg-raised transition-colors"
              >
                {copied ? 'Copied!' : 'Copy'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* API Tokens Tab */}
      {tab === 'tokens' && (
        <div className="space-y-6 max-w-2xl">
          {/* Create Token */}
          <div className="rounded-lg border border-edge bg-surface p-6">
            <h2 className="text-sm font-semibold text-fg mb-3">Create API Token</h2>
            <p className="text-xs text-subtle mb-4">
              Tokens authenticate API requests. Give each token a descriptive name so you remember what it's for.
            </p>
            <div className="flex items-end gap-3">
              <div className="flex-1">
                <label className="block text-xs text-subtle mb-1">Token Name</label>
                <input
                  type="text"
                  value={newTokenName}
                  onChange={(e) => setNewTokenName(e.target.value)}
                  placeholder="e.g. CLI tool, CI/CD, Home Assistant"
                  className="w-full px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg placeholder:text-dim focus:outline-none focus:ring-1 focus:ring-indigo-500"
                />
              </div>
              <button
                onClick={() => createTokenMut.mutate(newTokenName)}
                disabled={!newTokenName.trim() || createTokenMut.isPending}
                className="px-4 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-700 disabled:opacity-50 transition-colors"
              >
                {createTokenMut.isPending ? 'Creating...' : 'Create Token'}
              </button>
            </div>
            {createTokenMut.isError && (
              <p className="mt-2 text-xs text-red-400">Failed to create token.</p>
            )}
          </div>

          {/* Newly Created Token (show once) */}
          {createdToken && (
            <div className="rounded-lg border border-green-500/30 bg-green-500/5 p-6">
              <h3 className="text-sm font-semibold text-green-400 mb-2">Token Created</h3>
              <p className="text-xs text-subtle mb-3">
                Copy this token now — you won't be able to see it again.
              </p>
              <div className="flex items-center gap-2">
                <code className="flex-1 px-3 py-2 text-xs bg-inset border border-edge rounded-md text-fg font-mono break-all">
                  {createdToken}
                </code>
                <button
                  onClick={() => handleCopy(createdToken)}
                  className="px-3 py-2 text-xs bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors"
                >
                  {copied ? 'Copied!' : 'Copy'}
                </button>
              </div>
              <button
                onClick={() => setCreatedToken(null)}
                className="mt-3 text-xs text-subtle hover:text-fg"
              >
                Dismiss
              </button>
            </div>
          )}

          {/* Token List */}
          <div className="rounded-lg border border-edge bg-surface p-6">
            <h2 className="text-sm font-semibold text-fg mb-4">Active Tokens</h2>
            {tokensLoading ? (
              <p className="text-sm text-subtle">Loading...</p>
            ) : tokens && tokens.length > 0 ? (
              <div className="space-y-3">
                {tokens.map((token: ApiToken) => (
                  <div
                    key={token.id}
                    className="flex items-center justify-between p-3 rounded-md border border-edge bg-inset"
                  >
                    <div>
                      <div className="text-sm font-medium text-fg">{token.name}</div>
                      <div className="text-xs text-dim font-mono mt-0.5">{token.token_preview}</div>
                      <div className="text-xs text-subtle mt-0.5">
                        Created {new Date(token.created_at + 'Z').toLocaleDateString()}
                        {token.last_used_at && (
                          <span className="ml-2">
                            · Last used {new Date(token.last_used_at + 'Z').toLocaleDateString()}
                          </span>
                        )}
                      </div>
                    </div>
                    <button
                      onClick={() => {
                        if (confirm(`Revoke token "${token.name}"? This cannot be undone.`)) {
                          revokeTokenMut.mutate(token.id);
                        }
                      }}
                      className="px-3 py-1 text-xs text-red-400 border border-red-500/30 rounded-md hover:bg-red-500/10 transition-colors"
                    >
                      Revoke
                    </button>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-subtle">No API tokens yet. Create one above.</p>
            )}
          </div>

          {/* Usage example */}
          <div className="rounded-lg border border-edge bg-surface p-6">
            <h2 className="text-sm font-semibold text-fg mb-3">Usage</h2>
            <p className="text-xs text-subtle mb-2">
              Use your token in the <code className="text-fg">Authorization</code> header or <code className="text-fg">X-API-Key</code> header:
            </p>
            <pre className="px-3 py-2 text-xs bg-inset border border-edge rounded-md text-subtle font-mono overflow-x-auto">
{`curl -H "Authorization: Bearer hb_your_token_here" \\
  ${window.location.origin}/api/devices`}
            </pre>
          </div>
        </div>
      )}

      {/* Pair Phone Tab */}
      {tab === 'pair-phone' && (
        <div className="space-y-6 max-w-lg">
          <div className="rounded-lg border border-edge bg-surface p-6">
            <h2 className="text-sm font-semibold text-fg mb-2">Pair your phone</h2>
            <p className="text-xs text-subtle mb-4">
              Scan this code with the Hookbot iOS app to sign it in — no server address or
              password to type. The code carries this server's address and a one-time key that
              expires after two minutes and works exactly once.
            </p>

            {pairing ? (
              secondsLeft > 0 ? (
                <div className="space-y-4">
                  <div className="flex justify-center p-6 bg-white rounded-lg">
                    <QRCode value={pairingPayload} size={220} fgColor="#000000" bgColor="#ffffff" />
                  </div>
                  <div className="text-center space-y-2">
                    <p className="text-sm font-mono text-fg" aria-live="polite">
                      Expires in {countdown}
                    </p>
                    <p className="text-xs text-subtle break-all">{pairing.server}</p>
                    <button
                      onClick={() => pairMut.mutate()}
                      disabled={pairMut.isPending}
                      className="mt-1 px-4 py-1.5 text-xs border border-edge text-subtle rounded-md hover:bg-raised disabled:opacity-50 transition-colors"
                    >
                      {pairMut.isPending ? 'Generating...' : 'New code'}
                    </button>
                  </div>
                </div>
              ) : (
                <div className="space-y-4 text-center">
                  <div className="rounded-lg border border-edge bg-inset p-8">
                    <p className="text-sm text-fg">This code has expired.</p>
                    <p className="text-xs text-subtle mt-1">
                      Codes are good for two minutes so a photographed screen stays useless.
                    </p>
                  </div>
                  <button
                    onClick={() => pairMut.mutate()}
                    disabled={pairMut.isPending}
                    className="px-5 py-2.5 text-sm bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg disabled:opacity-50 transition-colors"
                  >
                    {pairMut.isPending ? 'Generating...' : 'Generate a new code'}
                  </button>
                </div>
              )
            ) : (
              <button
                onClick={() => pairMut.mutate()}
                disabled={pairMut.isPending}
                className="px-5 py-2.5 text-sm bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg disabled:opacity-50 transition-colors"
              >
                {pairMut.isPending ? 'Generating...' : 'Show pairing code'}
              </button>
            )}

            {pairMut.isError && (
              <p className="mt-3 text-xs text-red-400">
                {(pairMut.error as Error)?.message || 'Failed to generate a pairing code.'}
              </p>
            )}
          </div>

          <div className="rounded-lg border border-edge bg-surface p-6">
            <h3 className="text-sm font-semibold text-fg mb-3">How it works</h3>
            <ol className="text-sm text-subtle space-y-2 list-decimal list-inside">
              <li>Click "Show pairing code" above</li>
              <li>Open the Hookbot iOS app and tap "Scan pairing code"</li>
              <li>Point the camera at the code</li>
              <li>The app stores its own credential and signs in</li>
            </ol>
            <p className="text-xs text-dim mt-3">
              Each phone gets its own token, listed under API Tokens as "Paired phone" — revoke it
              there if you lose the device.
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
