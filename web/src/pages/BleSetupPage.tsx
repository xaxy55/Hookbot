import { useState, useCallback } from 'react';

const BLE_SERVICE_UUID = '4fafc201-1fb5-459e-8fcc-c5c9c331914b';
const WIFI_CONFIG_UUID = 'beb5483e-36e1-4688-b7f5-ea07361b26a8';
const STATUS_UUID = 'beb5483e-36e1-4688-b7f5-ea07361b26a9';

type BleState =
  | 'idle'
  | 'scanning'
  | 'connected'
  | 'sending'
  | 'success'
  | 'error';

export default function BleSetupPage() {
  const [bleState, setBleState] = useState<BleState>('idle');
  const [deviceName, setDeviceName] = useState('');
  const [ssid, setSsid] = useState('');
  const [password, setPassword] = useState('');
  const [statusMsg, setStatusMsg] = useState('');
  const [error, setError] = useState('');
  const [device, setDevice] = useState<BluetoothDevice | null>(null);
  const [server, setServer] = useState<BluetoothRemoteGATTServer | null>(null);

  const isWebBluetoothSupported = typeof navigator !== 'undefined' && 'bluetooth' in navigator;

  const handleConnect = useCallback(async () => {
    if (!isWebBluetoothSupported) {
      setError('Web Bluetooth is not supported in this browser. Use Chrome or Edge.');
      return;
    }

    setBleState('scanning');
    setError('');
    setStatusMsg('');

    try {
      const bleDevice = await navigator.bluetooth.requestDevice({
        filters: [{ namePrefix: 'DeskBot' }],
        optionalServices: [BLE_SERVICE_UUID],
      });

      setDeviceName(bleDevice.name || 'Unknown');
      setStatusMsg(`Connecting to ${bleDevice.name}...`);

      const gattServer = await bleDevice.gatt!.connect();
      setDevice(bleDevice);
      setServer(gattServer);
      setBleState('connected');
      setStatusMsg(`Connected to ${bleDevice.name}`);

      // Try to read current status
      try {
        const service = await gattServer.getPrimaryService(BLE_SERVICE_UUID);
        const statusChar = await service.getCharacteristic(STATUS_UUID);
        const value = await statusChar.readValue();
        const decoder = new TextDecoder();
        setStatusMsg(decoder.decode(value));
      } catch {
        // Status read is optional
      }
    } catch (e: unknown) {
      if (e instanceof Error && e.name === 'NotFoundError') {
        setBleState('idle');
        setStatusMsg('No device selected');
      } else {
        setBleState('error');
        setError(e instanceof Error ? e.message : 'BLE connection failed');
      }
    }
  }, [isWebBluetoothSupported]);

  const handleSendWifi = useCallback(async () => {
    if (!server || !ssid) return;

    setBleState('sending');
    setError('');
    setStatusMsg('Sending WiFi credentials...');

    try {
      const service = await server.getPrimaryService(BLE_SERVICE_UUID);
      const wifiChar = await service.getCharacteristic(WIFI_CONFIG_UUID);

      // Format: SSID\nPASSWORD
      const payload = `${ssid}\n${password}`;
      const encoder = new TextEncoder();
      await wifiChar.writeValue(encoder.encode(payload));

      setBleState('success');
      setStatusMsg('WiFi credentials sent! Device will reboot and connect.');

      // Try to read status update
      try {
        const statusChar = await service.getCharacteristic(STATUS_UUID);
        const value = await statusChar.readValue();
        const decoder = new TextDecoder();
        setStatusMsg(decoder.decode(value));
      } catch {
        // Device may have already rebooted
      }
    } catch (e: unknown) {
      setBleState('error');
      setError(e instanceof Error ? e.message : 'Failed to send credentials');
    }
  }, [server, ssid, password]);

  const handleDisconnect = useCallback(() => {
    if (device?.gatt?.connected) {
      device.gatt.disconnect();
    }
    setDevice(null);
    setServer(null);
    setBleState('idle');
    setDeviceName('');
    setStatusMsg('');
    setError('');
  }, [device]);

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-fg">BLE WiFi Setup</h1>
      </div>

      <p className="text-sm text-subtle mb-6">
        Configure WiFi on a DeskBot device via Bluetooth. The device must be powered on
        and not connected to WiFi (BLE icon blinking on screen).
      </p>

      {!isWebBluetoothSupported && (
        <div className="mb-6 p-4 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
          Web Bluetooth is not supported in this browser. Please use Chrome or Edge.
        </div>
      )}

      {/* Step 1: Connect */}
      <div className="rounded-lg border border-edge bg-surface p-6 mb-4">
        <div className="flex items-center gap-3 mb-4">
          <div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold ${
            bleState === 'idle' || bleState === 'scanning'
              ? 'bg-red-600 text-white'
              : 'bg-green-600/20 text-green-400'
          }`}>
            {bleState !== 'idle' && bleState !== 'scanning' ? '\u2713' : '1'}
          </div>
          <h2 className="text-base font-semibold text-fg">Connect to Device</h2>
        </div>

        {bleState === 'idle' || bleState === 'scanning' ? (
          <button
            onClick={handleConnect}
            disabled={bleState === 'scanning' || !isWebBluetoothSupported}
            className="px-5 py-2.5 text-sm bg-red-600 hover:bg-red-700 text-white rounded-lg disabled:opacity-50 transition-colors"
          >
            {bleState === 'scanning' ? (
              <span className="flex items-center gap-2">
                <span className="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                Scanning...
              </span>
            ) : (
              'Scan for DeskBot'
            )}
          </button>
        ) : (
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <BluetoothIcon />
              <div>
                <div className="text-sm font-medium text-fg">{deviceName}</div>
                <div className="text-xs text-subtle">Connected via BLE</div>
              </div>
            </div>
            <button
              onClick={handleDisconnect}
              className="px-3 py-1.5 text-xs text-subtle hover:text-fg border border-edge rounded-md hover:bg-raised transition-colors"
            >
              Disconnect
            </button>
          </div>
        )}
      </div>

      {/* Step 2: Enter WiFi credentials */}
      {(bleState === 'connected' || bleState === 'sending' || bleState === 'success') && (
        <div className="rounded-lg border border-edge bg-surface p-6 mb-4">
          <div className="flex items-center gap-3 mb-4">
            <div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold ${
              bleState === 'success'
                ? 'bg-green-600/20 text-green-400'
                : 'bg-red-600 text-white'
            }`}>
              {bleState === 'success' ? '\u2713' : '2'}
            </div>
            <h2 className="text-base font-semibold text-fg">WiFi Credentials</h2>
          </div>

          <div className="space-y-4 max-w-sm">
            <div>
              <label className="block text-xs font-medium text-subtle mb-1.5">
                Network Name (SSID)
              </label>
              <input
                type="text"
                value={ssid}
                onChange={(e) => setSsid(e.target.value)}
                placeholder="Your WiFi network"
                disabled={bleState === 'sending' || bleState === 'success'}
                className="w-full px-3 py-2 text-sm bg-canvas border border-edge rounded-lg text-fg placeholder:text-muted focus:outline-none focus:border-brand disabled:opacity-50"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-subtle mb-1.5">
                Password
              </label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="WiFi password"
                disabled={bleState === 'sending' || bleState === 'success'}
                className="w-full px-3 py-2 text-sm bg-canvas border border-edge rounded-lg text-fg placeholder:text-muted focus:outline-none focus:border-brand disabled:opacity-50"
              />
            </div>
            <button
              onClick={handleSendWifi}
              disabled={!ssid || bleState === 'sending' || bleState === 'success'}
              className="px-5 py-2.5 text-sm bg-red-600 hover:bg-red-700 text-white rounded-lg disabled:opacity-50 transition-colors"
            >
              {bleState === 'sending' ? (
                <span className="flex items-center gap-2">
                  <span className="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Sending...
                </span>
              ) : bleState === 'success' ? (
                'Sent!'
              ) : (
                'Send to Device'
              )}
            </button>
          </div>
        </div>
      )}

      {/* Status / Error messages */}
      {statusMsg && (
        <div className={`p-4 rounded-lg text-sm ${
          bleState === 'success'
            ? 'bg-green-500/10 border border-green-500/20 text-green-400'
            : 'bg-surface border border-edge text-subtle'
        }`}>
          {statusMsg}
        </div>
      )}

      {error && (
        <div className="mt-4 p-4 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
          {error}
        </div>
      )}

      {/* Instructions */}
      <div className="mt-8 p-5 rounded-lg border border-edge bg-surface">
        <h3 className="text-sm font-semibold text-fg mb-3">How it works</h3>
        <ol className="text-sm text-subtle space-y-2 list-decimal list-inside">
          <li>Power on your DeskBot - if WiFi is not configured, BLE will start automatically</li>
          <li>Look for the blinking Bluetooth icon on the device screen</li>
          <li>Click "Scan for DeskBot" above - your browser will show nearby devices</li>
          <li>Select your DeskBot, enter WiFi credentials, and send</li>
          <li>The device reboots and connects to WiFi - then use Discovery to register it</li>
        </ol>
        <p className="text-xs text-muted mt-3">
          Requires Chrome or Edge browser with Bluetooth enabled. Credentials are stored securely in the device's NVS flash memory.
        </p>
      </div>
    </div>
  );
}

function BluetoothIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-blue-400">
      <polyline points="6.5 6.5 17.5 17.5 12 23 12 1 17.5 6.5 6.5 17.5" />
    </svg>
  );
}
