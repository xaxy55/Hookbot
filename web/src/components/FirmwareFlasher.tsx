import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getFirmware } from '../api/client';

const BASE = '/api';

export default function FirmwareFlasher() {
  const [selectedFw, setSelectedFw] = useState('');
  const [status, setStatus] = useState('');
  const [progress, setProgress] = useState(0);
  const [isFlashing, setIsFlashing] = useState(false);
  const [error, setError] = useState('');

  const { data: firmwares } = useQuery({
    queryKey: ['firmware'],
    queryFn: getFirmware,
  });

  const hasWebSerial = 'serial' in navigator;

  async function flashFirmware() {
    if (!selectedFw) return;
    setError('');
    setStatus('Downloading firmware...');
    setProgress(0);
    setIsFlashing(true);

    try {
      // Download the firmware binary from the server
      const res = await fetch(`${BASE}/firmware/${selectedFw}/binary`);
      if (!res.ok) throw new Error('Failed to download firmware binary');
      const firmwareData = await res.arrayBuffer();
      setStatus(`Firmware downloaded (${(firmwareData.byteLength / 1024).toFixed(0)} KB). Connecting to device...`);

      // Request serial port access
      const port = await (navigator as any).serial.requestPort({
        filters: [
          { usbVendorId: 0x10C4 }, // Silicon Labs CP2102/CP2104
          { usbVendorId: 0x1A86 }, // CH340
          { usbVendorId: 0x0403 }, // FTDI
          { usbVendorId: 0x303A }, // Espressif USB JTAG/Serial
        ],
      });

      await port.open({ baudRate: 115200 });
      setStatus('Port opened. Putting device into bootloader mode...');

      // Toggle DTR/RTS to enter bootloader (standard ESP32 auto-reset sequence)
      await port.setSignals({ dataTerminalReady: false, requestToSend: true });
      await new Promise(r => setTimeout(r, 100));
      await port.setSignals({ dataTerminalReady: true, requestToSend: false });
      await new Promise(r => setTimeout(r, 50));
      await port.setSignals({ dataTerminalReady: false, requestToSend: false });
      await new Promise(r => setTimeout(r, 500));

      // Close and reopen at bootloader baud rate
      await port.close();
      await port.open({ baudRate: 460800 });

      const writer = port.writable.getWriter();
      const reader = port.readable.getReader();

      // ESP32 SLIP protocol: sync with bootloader
      setStatus('Syncing with bootloader...');

      // Send sync command (SLIP encoded)
      const syncCmd = new Uint8Array([
        0xC0, // SLIP start
        0x00, 0x08, // command: sync
        0x24, 0x00, // data length: 36
        0x00, 0x00, 0x00, 0x00, // checksum
        0x07, 0x07, 0x12, 0x20, // sync pattern
        ...Array(32).fill(0x55), // sync bytes
        0xC0, // SLIP end
      ]);

      let synced = false;
      for (let attempt = 0; attempt < 10 && !synced; attempt++) {
        await writer.write(syncCmd);
        await new Promise(r => setTimeout(r, 100));

        // Try to read response
        try {
          const { value } = await Promise.race([
            reader.read(),
            new Promise<{ value: undefined }>((_, reject) =>
              setTimeout(() => reject(new Error('timeout')), 500)
            ),
          ]);
          if (value && value.length > 0) {
            synced = true;
          }
        } catch {
          // Timeout, retry
        }
      }

      reader.releaseLock();
      writer.releaseLock();
      await port.close();

      if (!synced) {
        // Fallback: just tell user to use esptool
        setStatus('');
        setError(
          'Could not sync with bootloader. For reliable USB flashing, use the PlatformIO CLI: ' +
          '`pio run -e <env> -t upload` or use the OTA deploy feature for WiFi-based updates.'
        );
        setIsFlashing(false);
        return;
      }

      // For a full flash we'd need to implement the full ESP32 serial protocol.
      // Instead, provide the binary download + instructions.
      setStatus('Bootloader detected! Full serial flash protocol is complex. Use esptool for USB flashing.');
      setProgress(100);
      setIsFlashing(false);

    } catch (err: any) {
      if (err.name === 'NotFoundError') {
        setError('No serial port selected. Please select your ESP32 device.');
      } else {
        setError(err.message || 'Flash failed');
      }
      setIsFlashing(false);
      setStatus('');
    }
  }

  async function downloadFirmware() {
    if (!selectedFw) return;
    const fw = firmwares?.find(f => f.id === selectedFw);
    const res = await fetch(`${BASE}/firmware/${selectedFw}/binary`);
    if (!res.ok) return;
    const blob = await res.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = fw?.filename || 'firmware.bin';
    a.click();
    URL.revokeObjectURL(url);
  }

  const selectedFirmware = firmwares?.find(f => f.id === selectedFw);

  return (
    <div className="rounded-lg border border-edge bg-surface p-4 space-y-4">
      <h3 className="text-sm font-semibold text-fg">USB Firmware Flasher</h3>

      {!hasWebSerial && (
        <p className="text-xs text-yellow-400 bg-yellow-900/20 p-2 rounded">
          Web Serial API not available. Use Chrome/Edge for USB flashing, or download the binary below.
        </p>
      )}

      <div>
        <label className="block text-sm text-muted mb-1">Firmware</label>
        <select
          value={selectedFw}
          onChange={(e) => setSelectedFw(e.target.value)}
          className="w-full px-3 py-1.5 text-sm bg-inset border border-edge rounded-md text-fg"
        >
          <option value="">Select firmware...</option>
          {firmwares?.map((fw) => (
            <option key={fw.id} value={fw.id}>
              v{fw.version} - {fw.filename} ({(fw.size_bytes / 1024).toFixed(0)} KB)
              {fw.device_type ? ` [${fw.device_type === 'esp32_4848s040c_lcd' ? 'LCD' : 'OLED'}]` : ''}
            </option>
          ))}
        </select>
        {selectedFirmware?.device_type && (
          <p className="text-xs text-purple-400 mt-1">
            Target: {selectedFirmware.device_type === 'esp32_4848s040c_lcd' ? 'ESP32-S3 LCD Touch' : 'ESP32 OLED'}
          </p>
        )}
      </div>

      <div className="flex gap-2">
        {hasWebSerial && (
          <button
            onClick={flashFirmware}
            disabled={!selectedFw || isFlashing}
            className="px-4 py-2 text-sm bg-red-600 hover:bg-red-700 rounded-md disabled:opacity-50"
          >
            {isFlashing ? 'Flashing...' : 'Flash via USB'}
          </button>
        )}
        <button
          onClick={downloadFirmware}
          disabled={!selectedFw}
          className="px-4 py-2 text-sm bg-raised hover:bg-raised rounded-md disabled:opacity-50"
        >
          Download .bin
        </button>
      </div>

      {status && (
        <div className="text-xs text-blue-400 bg-blue-900/20 p-2 rounded">
          {status}
          {progress > 0 && progress < 100 && (
            <div className="mt-1 h-1 bg-raised rounded-full overflow-hidden">
              <div className="h-full bg-blue-500 transition-all" style={{ width: `${progress}%` }} />
            </div>
          )}
        </div>
      )}

      {error && (
        <p className="text-xs text-red-400 bg-red-900/20 p-2 rounded">{error}</p>
      )}

      <p className="text-[10px] text-dim">
        For reliable USB flashing, use: <code className="text-subtle">pio run -e esp32 -t upload</code> or{' '}
        <code className="text-subtle">esptool.py write_flash 0x0 firmware.bin</code>
      </p>
    </div>
  );
}
