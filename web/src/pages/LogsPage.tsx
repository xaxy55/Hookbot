import { useQuery } from '@tanstack/react-query';
import { getDevices } from '../api/client';
import { getDeviceHistory } from '../api/client';
import { useState } from 'react';
import StateIndicator from '../components/StateIndicator';

export default function LogsPage() {
  const { data: devices } = useQuery({
    queryKey: ['devices'],
    queryFn: getDevices,
  });

  const [selectedDevice, setSelectedDevice] = useState<string>('');
  const deviceId = selectedDevice || devices?.[0]?.id || '';

  const { data: history } = useQuery({
    queryKey: ['history', deviceId],
    queryFn: () => getDeviceHistory(deviceId),
    enabled: !!deviceId,
    refetchInterval: 5000,
  });

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-fg">Logs</h1>
        <select
          value={selectedDevice}
          onChange={(e) => setSelectedDevice(e.target.value)}
          className="px-3 py-1.5 text-sm bg-inset border border-gray-700 rounded-md text-fg"
        >
          <option value="">All devices</option>
          {devices?.map((d) => (
            <option key={d.id} value={d.id}>{d.name}</option>
          ))}
        </select>
      </div>

      {history && history.length > 0 ? (
        <div className="rounded-lg border border-gray-800 bg-gray-900/50 overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-800 text-gray-500 text-left">
                <th className="px-4 py-3 font-medium">Time</th>
                <th className="px-4 py-3 font-medium">State</th>
                <th className="px-4 py-3 font-medium text-right">Uptime</th>
                <th className="px-4 py-3 font-medium text-right">Free Heap</th>
              </tr>
            </thead>
            <tbody>
              {history.map((s, i) => (
                <tr key={i} className="border-b border-gray-800/50 last:border-0">
                  <td className="px-4 py-2.5 text-gray-400 font-mono text-xs">{s.recorded_at}</td>
                  <td className="px-4 py-2.5"><StateIndicator state={s.state} /></td>
                  <td className="px-4 py-2.5 text-gray-400 text-right font-mono text-xs">{Math.floor(s.uptime_ms / 1000)}s</td>
                  <td className="px-4 py-2.5 text-gray-400 text-right font-mono text-xs">{(s.free_heap / 1024).toFixed(1)} KB</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <div className="text-center py-12 rounded-lg border border-gray-800 bg-gray-900/50">
          <p className="text-gray-500">No status logs yet</p>
          <p className="text-gray-600 text-xs mt-1">Logs are recorded when the poller reaches a device</p>
        </div>
      )}
    </div>
  );
}
