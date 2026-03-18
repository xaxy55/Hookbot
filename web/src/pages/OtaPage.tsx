import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getFirmware, getDevices, getOtaJobs, deployOta, buildFirmware } from '../api/client';
import type { BuildStatus } from '../api/client';
import OtaUpload from '../components/OtaUpload';
import FirmwareFlasher from '../components/FirmwareFlasher';

const DEVICE_TYPE_LABELS: Record<string, string> = {
  esp32_oled: 'ESP32 OLED (128x64)',
  esp32_4848s040c_lcd: 'ESP32-S3 LCD (480x480 Touch)',
};


export default function OtaPage() {
  const qc = useQueryClient();
  const [selectedFw, setSelectedFw] = useState('');
  const [selectedDevices, setSelectedDevices] = useState<string[]>([]);
  const [buildEnv, setBuildEnv] = useState('esp32');
  const [buildVersion, setBuildVersion] = useState('');
  const [buildResult, setBuildResult] = useState<BuildStatus | null>(null);
  const [showBuildLog, setShowBuildLog] = useState(false);

  const { data: firmwares } = useQuery({
    queryKey: ['firmware'],
    queryFn: getFirmware,
  });

  const { data: devices } = useQuery({
    queryKey: ['devices'],
    queryFn: getDevices,
  });

  const { data: jobs } = useQuery({
    queryKey: ['ota-jobs'],
    queryFn: getOtaJobs,
    refetchInterval: 3000,
  });

  const deploy = useMutation({
    mutationFn: () => deployOta(selectedFw, selectedDevices),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['ota-jobs'] });
      setSelectedDevices([]);
    },
  });

  const build = useMutation({
    mutationFn: () => buildFirmware(buildEnv, buildVersion || undefined),
    onSuccess: (data) => {
      setBuildResult(data);
      qc.invalidateQueries({ queryKey: ['firmware'] });
    },
    onError: (err: Error) => {
      setBuildResult({ status: 'error', message: err.message });
    },
  });

  // Get selected firmware info for type validation
  const selectedFirmware = firmwares?.find(f => f.id === selectedFw);

  // Check for device type mismatches
  const typeMismatches = selectedDevices.filter(devId => {
    const dev = devices?.find(d => d.id === devId);
    if (!dev?.device_type || !selectedFirmware?.device_type) return false;
    return dev.device_type !== selectedFirmware.device_type;
  });

  function toggleDevice(id: string) {
    setSelectedDevices((prev) =>
      prev.includes(id) ? prev.filter((d) => d !== id) : [...prev, id]
    );
  }

  function selectCompatible() {
    if (!devices || !selectedFirmware?.device_type) {
      if (devices) setSelectedDevices(devices.map(d => d.id));
      return;
    }
    setSelectedDevices(
      devices
        .filter(d => !d.device_type || d.device_type === selectedFirmware.device_type)
        .map(d => d.id)
    );
  }

  const statusColor: Record<string, string> = {
    pending: 'text-yellow-400',
    in_progress: 'text-blue-400',
    success: 'text-green-400',
    failed: 'text-red-400',
  };

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-bold text-white">OTA Updates</h1>

      <OtaUpload />

      <FirmwareFlasher />

      {/* Build from Source */}
      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-4 space-y-4">
        <h3 className="text-sm font-semibold text-white">Build from Source</h3>
        <p className="text-xs text-gray-500">
          Build firmware using PlatformIO directly from the server. Requires <code className="text-gray-400">pio</code> in PATH.
        </p>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm text-gray-400 mb-1">Target Board</label>
            <select
              value={buildEnv}
              onChange={(e) => setBuildEnv(e.target.value)}
              className="w-full px-3 py-1.5 text-sm bg-gray-800 border border-gray-700 rounded-md text-white"
            >
              <option value="esp32">ESP32 OLED (SSD1306)</option>
              <option value="esp32-4848s040c">ESP32-S3 LCD (480x480 Touch)</option>
            </select>
          </div>
          <div>
            <label className="block text-sm text-gray-400 mb-1">Version (optional)</label>
            <input
              value={buildVersion}
              onChange={(e) => setBuildVersion(e.target.value)}
              placeholder="auto-generated if empty"
              className="w-full px-3 py-1.5 text-sm bg-gray-800 border border-gray-700 rounded-md text-white placeholder-gray-600"
            />
          </div>
        </div>

        <button
          onClick={() => build.mutate()}
          disabled={build.isPending}
          className="px-4 py-2 text-sm bg-purple-600 hover:bg-purple-700 rounded-md disabled:opacity-50"
        >
          {build.isPending ? 'Building...' : 'Build Firmware'}
        </button>

        {buildResult && (
          <div className={`p-3 rounded-md text-sm ${buildResult.status === 'success' ? 'bg-green-900/30 border border-green-800' : 'bg-red-900/30 border border-red-800'}`}>
            <p className={buildResult.status === 'success' ? 'text-green-400' : 'text-red-400'}>
              {buildResult.message}
            </p>
            {buildResult.firmware && (
              <p className="text-gray-400 text-xs mt-1">
                {buildResult.firmware.filename} ({(buildResult.firmware.size_bytes / 1024).toFixed(0)} KB)
                {buildResult.firmware.device_type && (
                  <span className="ml-2 text-purple-400">
                    [{DEVICE_TYPE_LABELS[buildResult.firmware.device_type] || buildResult.firmware.device_type}]
                  </span>
                )}
              </p>
            )}
            {buildResult.build_log && (
              <div className="mt-2">
                <button
                  onClick={() => setShowBuildLog(!showBuildLog)}
                  className="text-xs text-gray-500 hover:text-gray-300"
                >
                  {showBuildLog ? 'Hide' : 'Show'} build log
                </button>
                {showBuildLog && (
                  <pre className="mt-1 text-[10px] text-gray-500 bg-black/50 p-2 rounded max-h-48 overflow-auto whitespace-pre-wrap">
                    {buildResult.build_log}
                  </pre>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Deploy Section */}
      <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-4 space-y-4">
        <h3 className="text-sm font-semibold text-white">Deploy Firmware</h3>

        <div>
          <label className="block text-sm text-gray-400 mb-1">Firmware Version</label>
          <select
            value={selectedFw}
            onChange={(e) => setSelectedFw(e.target.value)}
            className="w-full px-3 py-1.5 text-sm bg-gray-800 border border-gray-700 rounded-md text-white"
          >
            <option value="">Select firmware...</option>
            {firmwares?.map((fw) => (
              <option key={fw.id} value={fw.id}>
                v{fw.version} - {fw.filename} ({(fw.size_bytes / 1024).toFixed(0)} KB)
                {fw.device_type ? ` [${DEVICE_TYPE_LABELS[fw.device_type] || fw.device_type}]` : ''}
              </option>
            ))}
          </select>
          {selectedFirmware?.device_type && (
            <p className="text-xs text-purple-400 mt-1">
              Target: {DEVICE_TYPE_LABELS[selectedFirmware.device_type] || selectedFirmware.device_type}
            </p>
          )}
        </div>

        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="text-sm text-gray-400">Target Devices</label>
            <button onClick={selectCompatible} className="text-xs text-blue-400 hover:text-blue-300">
              {selectedFirmware?.device_type ? 'Select Compatible' : 'Select All'}
            </button>
          </div>
          <div className="space-y-1">
            {devices?.map((d) => {
              const isMismatch = typeMismatches.includes(d.id);
              return (
                <label key={d.id} className={`flex items-center gap-2 text-sm ${isMismatch ? 'text-red-400' : 'text-gray-300'}`}>
                  <input
                    type="checkbox"
                    checked={selectedDevices.includes(d.id)}
                    onChange={() => toggleDevice(d.id)}
                    className="rounded border-gray-600"
                  />
                  <span>{d.name}</span>
                  <span className="text-gray-500 text-xs">({d.ip_address})</span>
                  {d.device_type && (
                    <span className={`text-[10px] px-1.5 py-0.5 rounded-full border ${
                      d.device_type === 'esp32_4848s040c_lcd'
                        ? 'border-cyan-800 text-cyan-400 bg-cyan-900/20'
                        : 'border-amber-800 text-amber-400 bg-amber-900/20'
                    }`}>
                      {d.device_type === 'esp32_4848s040c_lcd' ? 'LCD' : 'OLED'}
                    </span>
                  )}
                  {isMismatch && <span className="text-[10px] text-red-500">type mismatch!</span>}
                </label>
              );
            })}
          </div>
        </div>

        {typeMismatches.length > 0 && (
          <p className="text-xs text-red-400 bg-red-900/20 p-2 rounded">
            {typeMismatches.length} device(s) have a different type than the selected firmware. Deploy will be rejected.
          </p>
        )}

        <button
          onClick={() => deploy.mutate()}
          disabled={!selectedFw || selectedDevices.length === 0 || deploy.isPending}
          className="px-4 py-2 text-sm bg-orange-600 hover:bg-orange-700 rounded-md disabled:opacity-50"
        >
          {deploy.isPending ? 'Deploying...' : `Deploy to ${selectedDevices.length} device(s)`}
        </button>

        {deploy.isError && (
          <p className="text-xs text-red-400">{deploy.error.message}</p>
        )}
      </div>

      {/* Job History */}
      {jobs && jobs.length > 0 && (
        <div className="rounded-lg border border-gray-800 bg-gray-900/50 p-4">
          <h3 className="text-sm font-semibold text-white mb-3">OTA Jobs</h3>
          <div className="space-y-2">
            {jobs.map((job) => (
              <div key={job.id} className="flex items-center justify-between text-sm">
                <span className="text-gray-400">
                  {job.device_id.slice(0, 8)}... &rarr; {job.firmware_id.slice(0, 8)}...
                </span>
                <span className={statusColor[job.status] || 'text-gray-400'}>
                  {job.status}
                  {job.error_msg && ` - ${job.error_msg}`}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
