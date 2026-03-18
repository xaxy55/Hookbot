import { useState, useRef } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { uploadFirmware } from '../api/client';

export default function OtaUpload() {
  const qc = useQueryClient();
  const fileRef = useRef<HTMLInputElement>(null);
  const [version, setVersion] = useState('');
  const [notes, setNotes] = useState('');
  const [deviceType, setDeviceType] = useState('');

  const upload = useMutation({
    mutationFn: () => {
      const file = fileRef.current?.files?.[0];
      if (!file) throw new Error('No file selected');
      if (!version) throw new Error('Version is required');
      return uploadFirmware(file, version, notes || undefined, deviceType || undefined);
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['firmware'] });
      setVersion('');
      setNotes('');
      setDeviceType('');
      if (fileRef.current) fileRef.current.value = '';
    },
  });

  return (
    <div className="rounded-lg border border-edge bg-surface p-4 space-y-3">
      <h3 className="text-sm font-semibold text-fg">Upload Firmware</h3>

      <input
        ref={fileRef}
        type="file"
        accept=".bin"
        className="block w-full text-sm text-muted file:mr-4 file:py-1.5 file:px-3 file:rounded-md file:border-0 file:text-sm file:bg-inset file:text-fg-2 hover:file:bg-raised"
      />

      <div className="grid grid-cols-2 gap-3">
        <input
          type="text"
          placeholder="Version (e.g. 1.0.0)"
          value={version}
          onChange={(e) => setVersion(e.target.value)}
          className="w-full px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg placeholder-subtle"
        />

        <select
          value={deviceType}
          onChange={(e) => setDeviceType(e.target.value)}
          className="w-full px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg"
        >
          <option value="">Device type (optional)</option>
          <option value="esp32_oled">ESP32 OLED</option>
          <option value="esp32_4848s040c_lcd">ESP32-S3 LCD Touch</option>
        </select>
      </div>

      <input
        type="text"
        placeholder="Notes (optional)"
        value={notes}
        onChange={(e) => setNotes(e.target.value)}
        className="w-full px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg placeholder-subtle"
      />

      <button
        onClick={() => upload.mutate()}
        disabled={upload.isPending}
        className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50"
      >
        {upload.isPending ? 'Uploading...' : 'Upload'}
      </button>

      {upload.isError && (
        <p className="text-xs text-red-400">{upload.error.message}</p>
      )}
      {upload.isSuccess && (
        <p className="text-xs text-green-400">Firmware uploaded successfully</p>
      )}
    </div>
  );
}
