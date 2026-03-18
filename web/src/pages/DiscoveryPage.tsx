import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { discoverDevices, createDevice } from '../api/client';
import type { DiscoveredDevice } from '../types';

export default function DiscoveryPage() {
  const qc = useQueryClient();
  const [discovering, setDiscovering] = useState(false);
  const [discovered, setDiscovered] = useState<DiscoveredDevice[]>([]);
  const [hasScanned, setHasScanned] = useState(false);

  const registerMut = useMutation({
    mutationFn: (d: DiscoveredDevice) =>
      createDevice({
        name: d.hostname,
        hostname: d.hostname,
        ip_address: d.ip_address,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['devices'] });
      // Re-run scan to update registered status
      handleDiscover();
    },
  });

  async function handleDiscover() {
    setDiscovering(true);
    try {
      const result = await discoverDevices();
      setDiscovered(result);
    } catch {
      // ignore
    }
    setDiscovering(false);
    setHasScanned(true);
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-fg">Discovery</h1>
        <button
          onClick={handleDiscover}
          disabled={discovering}
          className="px-4 py-2 text-sm bg-red-600 hover:bg-red-700 text-fg rounded-lg disabled:opacity-50 transition-colors"
        >
          {discovering ? 'Scanning...' : 'Scan Network'}
        </button>
      </div>

      <p className="text-sm text-subtle mb-6">
        Scan your local network for hookbot devices via mDNS. Discovered devices can be registered for management.
      </p>

      {discovered.length > 0 ? (
        <div className="rounded-lg border border-edge bg-surface overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-edge text-subtle text-left">
                <th className="px-4 py-3 font-medium">Hostname</th>
                <th className="px-4 py-3 font-medium">IP Address</th>
                <th className="px-4 py-3 font-medium">Port</th>
                <th className="px-4 py-3 font-medium text-right">Action</th>
              </tr>
            </thead>
            <tbody>
              {discovered.map((d) => (
                <tr key={d.hostname} className="border-b border-edge/50 last:border-0">
                  <td className="px-4 py-3 text-fg font-medium">{d.hostname}</td>
                  <td className="px-4 py-3 text-muted font-mono">{d.ip_address}</td>
                  <td className="px-4 py-3 text-muted">{d.port}</td>
                  <td className="px-4 py-3 text-right">
                    {d.already_registered ? (
                      <span className="text-xs text-dim">Registered</span>
                    ) : (
                      <button
                        onClick={() => registerMut.mutate(d)}
                        disabled={registerMut.isPending}
                        className="px-3 py-1 text-xs bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
                      >
                        Register
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : hasScanned ? (
        <div className="text-center py-12 rounded-lg border border-edge bg-surface">
          <p className="text-subtle">No devices found on the network</p>
          <p className="text-dim text-xs mt-1">Make sure your hookbot devices are powered on and connected to WiFi</p>
        </div>
      ) : (
        <div className="text-center py-12 rounded-lg border border-dashed border-edge">
          <p className="text-dim">Click "Scan Network" to discover hookbot devices</p>
        </div>
      )}
    </div>
  );
}
