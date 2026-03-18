import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { DeviceConfig } from '../types';
import { updateDeviceConfig, pushConfig } from '../api/client';

export default function PersonalityForm({ config }: { config: DeviceConfig }) {
  const qc = useQueryClient();
  const [brightness, setBrightness] = useState(config.led_brightness);
  const [soundEnabled, setSoundEnabled] = useState(config.sound_enabled);
  const [volume, setVolume] = useState(config.sound_volume);

  const updateMut = useMutation({
    mutationFn: (data: Partial<DeviceConfig>) => updateDeviceConfig(config.device_id, data),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['config', config.device_id] }),
  });

  const pushMut = useMutation({
    mutationFn: () => pushConfig(config.device_id),
  });

  function handleSave() {
    updateMut.mutate({
      led_brightness: brightness,
      sound_enabled: soundEnabled,
      sound_volume: volume,
    });
  }

  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm text-muted mb-1">LED Brightness</label>
        <input
          type="range"
          min={0}
          max={255}
          value={brightness}
          onChange={(e) => setBrightness(Number(e.target.value))}
          className="w-full"
        />
        <span className="text-xs text-subtle">{brightness}</span>
      </div>

      <div className="flex items-center gap-3">
        <label className="text-sm text-muted">Sound</label>
        <button
          onClick={() => setSoundEnabled(!soundEnabled)}
          className={`px-3 py-1 text-xs rounded-full ${
            soundEnabled ? 'bg-green-900/50 text-green-400' : 'bg-inset text-subtle'
          }`}
        >
          {soundEnabled ? 'On' : 'Off'}
        </button>
      </div>

      {soundEnabled && (
        <div>
          <label className="block text-sm text-muted mb-1">Volume</label>
          <input
            type="range"
            min={0}
            max={100}
            value={volume}
            onChange={(e) => setVolume(Number(e.target.value))}
            className="w-full"
          />
          <span className="text-xs text-subtle">{volume}%</span>
        </div>
      )}

      <div className="flex gap-2">
        <button
          onClick={handleSave}
          disabled={updateMut.isPending}
          className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
        >
          {updateMut.isPending ? 'Saving...' : 'Save'}
        </button>
        <button
          onClick={() => pushMut.mutate()}
          disabled={pushMut.isPending}
          className="px-4 py-2 text-sm bg-raised hover:bg-raised rounded-md disabled:opacity-50"
        >
          {pushMut.isPending ? 'Pushing...' : 'Push to Device'}
        </button>
      </div>

      {updateMut.isSuccess && <p className="text-xs text-green-400">Saved</p>}
      {pushMut.isSuccess && <p className="text-xs text-green-400">Config pushed to device</p>}
      {(updateMut.isError || pushMut.isError) && (
        <p className="text-xs text-red-400">
          {(updateMut.error || pushMut.error)?.message}
        </p>
      )}
    </div>
  );
}
