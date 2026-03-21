import { useState, useCallback } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { claimDevice } from '../api/client';

const BLE_SERVICE_UUID = '4fafc201-1fb5-459e-8fcc-c5c9c331914b';
const WIFI_CONFIG_UUID = 'beb5483e-36e1-4688-b7f5-ea07361b26a8';
const STATUS_UUID = 'beb5483e-36e1-4688-b7f5-ea07361b26a9';
const CLAIM_INFO_UUID = 'beb5483e-36e1-4688-b7f5-ea07361b26aa';

type BleState =
  | 'idle'
  | 'scanning'
  | 'connected'
  | 'sending'
  | 'success'
  | 'error';

interface ClaimInfo {
  claim_code: string;
  claimed: boolean;
  cloud: boolean;
  wifi: boolean;
  name?: string;
}

export default function BleSetupPage() {
  const queryClient = useQueryClient();
  const [bleState, setBleState] = useState<BleState>('idle');
  const [deviceName, setDeviceName] = useState('');
  const [ssid, setSsid] = useState('');
  const [password, setPassword] = useState('');
  const [statusMsg, setStatusMsg] = useState('');
  const [error, setError] = useState('');
  const [device, setDevice] = useState<any>(null);
  const [server, setServer] = useState<any>(null);
  const [claimInfo, setClaimInfo] = useState<ClaimInfo | null>(null);
  const [claimDeviceName, setClaimDeviceName] = useState('');

  const isWebBluetoothSupported = typeof navigator !== 'undefined' && 'bluetooth' in navigator;

  const claimDeviceMut = useMutation({
    mutationFn: (data: { claim_code: string; name?: string }) => claimDevice(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['devices'] });
      setClaimInfo(prev => prev ? { ...prev, claimed: true } : null);
      setStatusMsg('Device claimed successfully! It will appear in your devices list.');
    },
    onError: () => {
      setError('Failed to claim device. Make sure you are logged in and the code is valid.');
    },
  });

  const readClaimInfo = useCallback(async (gattServer: any) => {
    try {
      const service = await gattServer.getPrimaryService(BLE_SERVICE_UUID);
      const claimChar = await service.getCharacteristic(CLAIM_INFO_UUID);
      const value = await claimChar.readValue();
      const decoder = new TextDecoder();
      const json = decoder.decode(value);
      const info: ClaimInfo = JSON.parse(json);
      setClaimInfo(info);
      if (info.name) {
        setClaimDeviceName(info.name);
      }
      return info;
    } catch {
      // Claim info characteristic not available (older firmware)
      return null;
    }
  }, []);

  const handleConnect = useCallback(async () => {
    if (!isWebBluetoothSupported) {
      setError('Web Bluetooth is not supported in this browser. Use Chrome or Edge.');
      return;
    }

    setBleState('scanning');
    setError('');
    setStatusMsg('');
    setClaimInfo(null);

    try {
      const bleDevice = await (navigator as any).bluetooth.requestDevice({
        filters: [{ namePrefix: 'Hookbot' }],
        optionalServices: [BLE_SERVICE_UUID],
      });

      setDeviceName(bleDevice.name || 'Unknown');
      setStatusMsg(`Connecting to ${bleDevice.name}...`);

      const gattServer = await bleDevice.gatt!.connect();
      setDevice(bleDevice);
      setServer(gattServer);
      setBleState('connected');
      setStatusMsg(`Connected to ${bleDevice.name}`);

      // Read claim info if available
      await readClaimInfo(gattServer);

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
  }, [isWebBluetoothSupported, readClaimInfo]);

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

  const handleClaimViaBle = useCallback(() => {
    if (!claimInfo?.claim_code) return;
    claimDeviceMut.mutate({
      claim_code: claimInfo.claim_code,
      ...(claimDeviceName.trim() ? { name: claimDeviceName.trim() } : {}),
    });
  }, [claimInfo, claimDeviceName, claimDeviceMut]);

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
    setClaimInfo(null);
    setClaimDeviceName('');
  }, [device]);

  // Determine which steps to show
  const deviceHasWifi = claimInfo?.wifi === true;
  const deviceIsCloud = claimInfo?.cloud === true;
  const deviceIsClaimed = claimInfo?.claimed === true;
  const hasClaimCode = claimInfo?.claim_code && claimInfo.claim_code.length > 0;
  const isConnected = bleState === 'connected' || bleState === 'sending' || bleState === 'success';

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-bold text-fg">Bluetooth Setup</h1>
      </div>

      <p className="text-sm text-subtle mb-6">
        Set up WiFi and pair your Hookbot device via Bluetooth. Works for first-time
        setup and for claiming cloud-connected devices to your account.
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
            isConnected
              ? 'bg-green-600/20 text-green-400'
              : 'bg-indigo-600 text-white'
          }`}>
            {isConnected ? '\u2713' : '1'}
          </div>
          <h2 className="text-base font-semibold text-fg">Connect to Device</h2>
        </div>

        {!isConnected ? (
          <button
            onClick={handleConnect}
            disabled={bleState === 'scanning' || !isWebBluetoothSupported}
            className="px-5 py-2.5 text-sm bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg disabled:opacity-50 transition-colors"
          >
            {bleState === 'scanning' ? (
              <span className="flex items-center gap-2">
                <span className="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                Scanning...
              </span>
            ) : (
              'Scan for Hookbot'
            )}
          </button>
        ) : (
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <BluetoothIcon />
              <div>
                <div className="text-sm font-medium text-fg">{deviceName}</div>
                <div className="text-xs text-subtle flex items-center gap-2">
                  Connected via BLE
                  {claimInfo && (
                    <>
                      <span className="text-dim">·</span>
                      {deviceHasWifi ? (
                        <span className="text-green-400">WiFi OK</span>
                      ) : (
                        <span className="text-yellow-400">No WiFi</span>
                      )}
                      {deviceIsCloud && (
                        <>
                          <span className="text-dim">·</span>
                          {deviceIsClaimed ? (
                            <span className="text-green-400">Claimed</span>
                          ) : (
                            <span className="text-yellow-400">Unclaimed</span>
                          )}
                        </>
                      )}
                    </>
                  )}
                </div>
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

      {/* Step 2: WiFi Credentials (only if device doesn't have WiFi) */}
      {isConnected && !deviceHasWifi && (
        <div className="rounded-lg border border-edge bg-surface p-6 mb-4">
          <div className="flex items-center gap-3 mb-4">
            <div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold ${
              bleState === 'success'
                ? 'bg-green-600/20 text-green-400'
                : 'bg-indigo-600 text-white'
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
                className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-lg text-fg placeholder:text-dim focus:outline-none focus:ring-1 focus:ring-indigo-500 disabled:opacity-50"
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
                className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-lg text-fg placeholder:text-dim focus:outline-none focus:ring-1 focus:ring-indigo-500 disabled:opacity-50"
              />
            </div>
            <button
              onClick={handleSendWifi}
              disabled={!ssid || bleState === 'sending' || bleState === 'success'}
              className="px-5 py-2.5 text-sm bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg disabled:opacity-50 transition-colors"
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

      {/* Step 3: Claim Device via Bluetooth (cloud devices with claim code) */}
      {isConnected && deviceIsCloud && hasClaimCode && !deviceIsClaimed && (
        <div className="rounded-lg border border-green-500/30 bg-green-500/5 p-6 mb-4">
          <div className="flex items-center gap-3 mb-4">
            <div className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold ${
              claimDeviceMut.isSuccess
                ? 'bg-green-600/20 text-green-400'
                : 'bg-green-600 text-white'
            }`}>
              {claimDeviceMut.isSuccess ? '\u2713' : deviceHasWifi ? '2' : '3'}
            </div>
            <h2 className="text-base font-semibold text-fg">Claim Device</h2>
          </div>

          <p className="text-sm text-subtle mb-4">
            This device is broadcasting claim code <span className="font-mono font-bold text-fg tracking-wider">{claimInfo?.claim_code}</span> via Bluetooth.
            Claim it to add it to your account.
          </p>

          <div className="space-y-4 max-w-sm">
            <div>
              <label className="block text-xs font-medium text-subtle mb-1.5">
                Device Name (optional)
              </label>
              <input
                type="text"
                value={claimDeviceName}
                onChange={(e) => setClaimDeviceName(e.target.value)}
                placeholder="e.g. Office Bot"
                disabled={claimDeviceMut.isPending || claimDeviceMut.isSuccess}
                className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-lg text-fg placeholder:text-dim focus:outline-none focus:ring-1 focus:ring-green-500 disabled:opacity-50"
              />
            </div>
            <button
              onClick={handleClaimViaBle}
              disabled={claimDeviceMut.isPending || claimDeviceMut.isSuccess}
              className="px-5 py-2.5 text-sm bg-green-600 hover:bg-green-700 text-white rounded-lg disabled:opacity-50 transition-colors"
            >
              {claimDeviceMut.isPending ? (
                <span className="flex items-center gap-2">
                  <span className="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Claiming...
                </span>
              ) : claimDeviceMut.isSuccess ? (
                'Claimed!'
              ) : (
                'Claim This Device'
              )}
            </button>
          </div>
        </div>
      )}

      {/* Already claimed indicator */}
      {isConnected && deviceIsCloud && deviceIsClaimed && (
        <div className="rounded-lg border border-green-500/30 bg-green-500/5 p-6 mb-4">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold bg-green-600/20 text-green-400">
              {'\u2713'}
            </div>
            <div>
              <h2 className="text-base font-semibold text-fg">Device Already Claimed</h2>
              <p className="text-sm text-subtle">This device is already linked to an account.</p>
            </div>
          </div>
        </div>
      )}

      {/* Status / Error messages */}
      {statusMsg && (
        <div className={`p-4 rounded-lg text-sm ${
          bleState === 'success' || claimDeviceMut.isSuccess
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
          <li>Power on your Hookbot — BLE starts automatically when WiFi is not configured, or while the device is unclaimed</li>
          <li>Look for the blinking Bluetooth icon on the device screen</li>
          <li>Click "Scan for Hookbot" above — your browser will show nearby devices</li>
          <li><strong>WiFi setup:</strong> If the device needs WiFi, enter your network credentials and send</li>
          <li><strong>Claim device:</strong> If the device has a claim code, click "Claim This Device" to add it to your account</li>
        </ol>
        <p className="text-xs text-dim mt-3">
          Requires Chrome or Edge browser with Bluetooth enabled. You can also claim devices manually
          on the Devices page by entering the 6-character code shown on the device screen.
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
