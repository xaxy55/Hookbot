import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import type { DeviceWithStatus } from '../types';
import { getOtaJobs, getFirmware } from '../api/client';
import StateIndicator from './StateIndicator';

function formatUptime(ms: number): string {
  const secs = Math.floor(ms / 1000);
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

export default function DeviceCard({ device }: { device: DeviceWithStatus }) {
  const { data: otaJobs } = useQuery({ queryKey: ['ota-jobs'], queryFn: getOtaJobs });
  const { data: firmwares } = useQuery({ queryKey: ['firmware'], queryFn: getFirmware });

  const lastSuccessJob = otaJobs
    ?.filter(j => j.device_id === device.id && j.status === 'success')
    .sort((a, b) => b.created_at.localeCompare(a.created_at))[0];
  const fwVersion = lastSuccessJob
    ? firmwares?.find(f => f.id === lastSuccessJob.firmware_id)?.version
    : undefined;

  return (
    <Link
      to={`/devices/${device.id}`}
      className="block rounded-lg border border-edge bg-surface p-4 hover:border-edge hover:bg-raised transition-all"
    >
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold text-fg">{device.name}</h3>
          <p className="text-xs text-subtle mt-0.5">{device.hostname}.local</p>
        </div>
        <div className={`w-2 h-2 rounded-full mt-1 ${device.online ? 'bg-green-500' : 'bg-dim'}`} />
      </div>

      {device.purpose && (
        <p className="text-xs text-muted mt-2">{device.purpose}</p>
      )}

      <div className="mt-3 flex items-center justify-between">
        {device.latest_status ? (
          <>
            <StateIndicator state={device.latest_status.state} />
            <div className="flex items-center gap-2">
              {fwVersion && (
                <span className="text-[10px] px-1.5 py-0.5 rounded bg-teal-500/10 text-teal-400 border border-teal-500/20 font-mono">
                  v{fwVersion}
                </span>
              )}
              <span className="text-xs text-dim">
                {formatUptime(device.latest_status.uptime_ms)}
              </span>
            </div>
          </>
        ) : (
          <span className="text-xs text-dim">No status</span>
        )}
      </div>
    </Link>
  );
}
