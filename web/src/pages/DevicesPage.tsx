import { useQuery } from '@tanstack/react-query';
import { getDevices } from '../api/client';
import DeviceCard from '../components/DeviceCard';

export default function DevicesPage() {
  const { data: devices, isLoading } = useQuery({
    queryKey: ['devices'],
    queryFn: getDevices,
    refetchInterval: 5000,
  });

  return (
    <div>
      <h1 className="text-xl font-bold text-fg mb-6">Devices</h1>

      {isLoading ? (
        <p className="text-subtle text-sm">Loading devices...</p>
      ) : devices && devices.length > 0 ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {devices.map((d) => (
            <DeviceCard key={d.id} device={d} />
          ))}
        </div>
      ) : (
        <div className="text-center py-12">
          <p className="text-subtle mb-2">No devices registered</p>
          <p className="text-dim text-sm">
            Go to Discovery to scan your network and register devices.
          </p>
        </div>
      )}
    </div>
  );
}
