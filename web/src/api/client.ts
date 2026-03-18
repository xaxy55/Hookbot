import type {
  DeviceWithStatus,
  Device,
  DeviceConfig,
  Firmware,
  OtaJob,
  DiscoveredDevice,
  StatusSnapshot,
  GamificationStats,
  ActivityEntry,
  AnalyticsData,
  BadgeDefinition,
  LeaderboardEntry,
} from '../types';

const BASE = import.meta.env.VITE_API_BASE_URL
  ? `${import.meta.env.VITE_API_BASE_URL}/api`
  : '/api';

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...options,
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(body.error || res.statusText);
  }
  return res.json();
}

// Devices
export const getDevices = () => request<DeviceWithStatus[]>('/devices');

export const getDevice = (id: string) => request<DeviceWithStatus>(`/devices/${id}`);

export const createDevice = (data: {
  name: string;
  hostname: string;
  ip_address: string;
  purpose?: string;
  personality?: string;
}) => request<Device>('/devices', { method: 'POST', body: JSON.stringify(data) });

export const updateDevice = (id: string, data: Record<string, unknown>) =>
  request<Device>(`/devices/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteDevice = (id: string) =>
  request<{ ok: boolean }>(`/devices/${id}`, { method: 'DELETE' });

export const sendState = (id: string, state: string) =>
  request<{ ok: boolean }>(`/devices/${id}/state`, {
    method: 'POST',
    body: JSON.stringify({ state }),
  });

export const sendTasks = (id: string, items: { label: string; status: number }[], active?: number) =>
  request<{ ok: boolean }>(`/devices/${id}/tasks`, {
    method: 'POST',
    body: JSON.stringify({ items, active: active ?? 0 }),
  });

export const getDeviceStatus = (id: string) =>
  request<Record<string, unknown>>(`/devices/${id}/status`);

// Servos
export const getServos = (id: string) =>
  request<{ channels: ServoChannel[]; state_maps: Record<string, number[]> }>(`/devices/${id}/servos`);

export const setServoAngles = (id: string, angles: number[]) =>
  request<{ ok: boolean }>(`/devices/${id}/servos`, {
    method: 'POST',
    body: JSON.stringify({ angles }),
  });

export const setServoAngle = (id: string, channel: number, angle: number) =>
  request<{ ok: boolean }>(`/devices/${id}/servos`, {
    method: 'POST',
    body: JSON.stringify({ channel, angle }),
  });

export const restServos = (id: string) =>
  request<{ ok: boolean }>(`/devices/${id}/servos`, {
    method: 'POST',
    body: JSON.stringify({ rest: true }),
  });

export const configureServos = (id: string, config: { channels?: Partial<ServoChannel>[]; state_maps?: Record<string, number[]> }) =>
  request<{ ok: boolean }>(`/devices/${id}/servos/config`, {
    method: 'POST',
    body: JSON.stringify(config),
  });

export interface ServoChannel {
  pin: number;
  min: number;
  max: number;
  rest: number;
  current: number;
  label: string;
  enabled: boolean;
}

export const getDeviceHistory = (id: string) =>
  request<StatusSnapshot[]>(`/devices/${id}/history`);

// Config
export const getDeviceConfig = (id: string) =>
  request<DeviceConfig>(`/devices/${id}/config`);

export const updateDeviceConfig = (id: string, data: Partial<DeviceConfig>) =>
  request<DeviceConfig>(`/devices/${id}/config`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });

export const pushConfig = (id: string) =>
  request<{ ok: boolean }>(`/devices/${id}/config/push`, { method: 'POST' });

// Discovery
export const discoverDevices = () => request<DiscoveredDevice[]>('/discovery');

// Firmware
export const getFirmware = () => request<Firmware[]>('/firmware');

export const uploadFirmware = async (file: File, version: string, notes?: string, deviceType?: string) => {
  const form = new FormData();
  form.append('file', file);
  form.append('version', version);
  if (notes) form.append('notes', notes);
  if (deviceType) form.append('device_type', deviceType);

  const res = await fetch(`${BASE}/firmware`, { method: 'POST', body: form });
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(body.error || res.statusText);
  }
  return res.json() as Promise<Firmware>;
};

// Build firmware from source
export interface BuildStatus {
  status: string;
  message: string;
  firmware?: Firmware;
  build_log?: string;
}

export const buildFirmware = (environment: string, version?: string, notes?: string) =>
  request<BuildStatus>('/firmware/build', {
    method: 'POST',
    body: JSON.stringify({ environment, version, notes }),
  });

// OTA
export const deployOta = (firmwareId: string, deviceIds: string[]) =>
  request<OtaJob[]>('/ota/deploy', {
    method: 'POST',
    body: JSON.stringify({ firmware_id: firmwareId, device_ids: deviceIds }),
  });

export const getOtaJobs = () => request<OtaJob[]>('/ota/jobs');

// Notifications
export const sendNotification = (deviceId: string, source: string, unread: number, active?: boolean) =>
  request<{ ok: boolean }>(`/devices/${deviceId}/notifications`, {
    method: 'POST',
    body: JSON.stringify({ source, unread, active }),
  });

export const sendWebhookNotification = (data: {
  source?: string;
  unread?: number;
  active?: boolean;
  device_id?: string;
}) =>
  request<{ ok: boolean; devices_notified: number }>('/notifications/webhook', {
    method: 'POST',
    body: JSON.stringify(data),
  });

// Notifications (persistent)
export interface NotificationEntry {
  id: number;
  device_id: string;
  source: string;
  unread: number;
  message: string | null;
  delivered: boolean;
  created_at: string;
  delivered_at: string | null;
}

export const getNotifications = (deviceId: string) =>
  request<NotificationEntry[]>(`/devices/${deviceId}/notifications`);

export const deleteNotification = (deviceId: string, notifId: number) =>
  request<{ ok: boolean }>(`/devices/${deviceId}/notifications/${notifId}`, { method: 'DELETE' });

// Sensors
export interface SensorChannelConfig {
  channel: number;
  pin: number;
  sensor_type: string;
  label: string | null;
  poll_interval_ms: number;
  threshold: number;
  last_value?: number;
}

export const getSensors = (deviceId: string) =>
  request<SensorChannelConfig[]>(`/devices/${deviceId}/sensors`);

export const updateSensors = (deviceId: string, channels: Partial<SensorChannelConfig>[]) =>
  request<{ ok: boolean }>(`/devices/${deviceId}/sensors`, {
    method: 'PUT',
    body: JSON.stringify({ channels }),
  });

// Automation Rules
export interface AutomationRule {
  id: string;
  device_id: string;
  name: string;
  enabled: boolean;
  trigger_type: string;
  trigger_config: Record<string, unknown>;
  action_type: string;
  action_config: Record<string, unknown>;
  cooldown_secs: number;
  last_triggered_at: string | null;
  created_at: string;
}

export const getRules = (deviceId: string) =>
  request<AutomationRule[]>(`/devices/${deviceId}/rules`);

export const createRule = (deviceId: string, data: {
  name: string;
  trigger_type: string;
  trigger_config: Record<string, unknown>;
  action_type: string;
  action_config: Record<string, unknown>;
  cooldown_secs?: number;
}) => request<AutomationRule>(`/devices/${deviceId}/rules`, { method: 'POST', body: JSON.stringify(data) });

export const updateRule = (deviceId: string, ruleId: string, data: Partial<AutomationRule>) =>
  request<AutomationRule>(`/devices/${deviceId}/rules/${ruleId}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteRule = (deviceId: string, ruleId: string) =>
  request<{ ok: boolean }>(`/devices/${deviceId}/rules/${ruleId}`, { method: 'DELETE' });

// Context
export interface DeviceContext {
  context: string;
  confidence: number;
  recent_tools: string[];
}

export const getContext = () => request<DeviceContext>('/context');

// AI
export interface AISummary {
  summary: string;
}

export const getAISummary = () => request<AISummary>('/ai/summary');

// Health
export const getHealth = () => request<{ status: string }>('/health');

// Gamification
export const getGamificationStats = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<GamificationStats>(`/gamification/stats${params}`);
};

export const getActivity = (limit?: number, deviceId?: string) => {
  const params = new URLSearchParams();
  if (limit) params.set('limit', String(limit));
  if (deviceId) params.set('device_id', deviceId);
  const qs = params.toString();
  return request<ActivityEntry[]>(`/gamification/activity${qs ? `?${qs}` : ''}`);
};

export const getAnalytics = (days?: number, deviceId?: string) => {
  const params = new URLSearchParams();
  if (days) params.set('days', String(days));
  if (deviceId) params.set('device_id', deviceId);
  const qs = params.toString();
  return request<AnalyticsData>(`/gamification/analytics${qs ? `?${qs}` : ''}`);
};

export const getAchievements = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<BadgeDefinition[]>(`/gamification/achievements${params}`);
};

export const getLeaderboard = () => request<LeaderboardEntry[]>('/gamification/leaderboard');

// Diagnostics
export interface DiagCheck {
  name: string;
  status: 'pass' | 'fail' | 'warn';
  message: string;
  latency_ms: number | null;
}

export interface DiagResult {
  checks: DiagCheck[];
  overall: string;
}

export const runDiagnostics = () => request<DiagResult>('/diagnostics');

// Settings
export interface ServerSettings {
  log_retention_hours: number;
  poll_interval_secs: number;
}

export const getServerSettings = () => request<ServerSettings>('/settings');

export const updateServerSettings = (data: { log_retention_hours?: number }) =>
  request<{ ok: boolean }>('/settings', { method: 'PUT', body: JSON.stringify(data) });

// Log management
export interface LogStats {
  total_entries: number;
  expired_entries: number;
  oldest_entry: string | null;
  retention_hours: number;
}

export const getLogStats = () => request<LogStats>('/logs/stats');

export const pruneLogs = () =>
  request<{ ok: boolean; deleted: number }>('/logs/prune', { method: 'DELETE' });

// Store
export interface StoreItemDef {
  id: string;
  name: string;
  description: string;
  category: string;
  price: number;
  icon: string;
  rarity: string;
  owned: boolean;
  can_afford: boolean;
}

export interface StoreResponse {
  items: StoreItemDef[];
  balance: number;
}

export const getStore = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<StoreResponse>(`/store${params}`);
};

export const buyItem = (itemId: string, deviceId?: string) =>
  request<{ ok: boolean; item_id: string; xp_spent: number; new_balance: number }>(
    '/store/buy',
    { method: 'POST', body: JSON.stringify({ item_id: itemId, device_id: deviceId }) },
  );

export const getOwnedItems = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<string[]>(`/store/owned${params}`);
};

// Community Plugin Store
export interface CommunityPlugin {
  id: string;
  name: string;
  description: string;
  author: string;
  version: string;
  category: string;
  tags: string[];
  payload: Record<string, unknown>;
  downloads: number;
  rating_avg: number;
  rating_count: number;
  installed: boolean;
  created_at: string;
  updated_at: string;
}

export const getCommunityPlugins = (opts?: { deviceId?: string; category?: string; search?: string; sort?: string }) => {
  const params = new URLSearchParams();
  if (opts?.deviceId) params.set('device_id', opts.deviceId);
  if (opts?.category) params.set('category', opts.category);
  if (opts?.search) params.set('search', opts.search);
  if (opts?.sort) params.set('sort', opts.sort);
  const qs = params.toString();
  return request<CommunityPlugin[]>(`/community/plugins${qs ? `?${qs}` : ''}`);
};

export const publishPlugin = (data: {
  name: string;
  description: string;
  author?: string;
  version?: string;
  category?: string;
  tags?: string[];
  payload?: Record<string, unknown>;
}) => request<CommunityPlugin>('/community/plugins', { method: 'POST', body: JSON.stringify(data) });

export const installPlugin = (pluginId: string, deviceId?: string) =>
  request<{ ok: boolean }>(`/community/plugins/${pluginId}/install`, {
    method: 'POST',
    body: JSON.stringify({ device_id: deviceId }),
  });

export const uninstallPlugin = (pluginId: string, deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<{ ok: boolean }>(`/community/plugins/${pluginId}/install${params}`, { method: 'DELETE' });
};

export const ratePlugin = (pluginId: string, stars: number, deviceId?: string) =>
  request<{ ok: boolean }>(`/community/plugins/${pluginId}/rate`, {
    method: 'POST',
    body: JSON.stringify({ stars, device_id: deviceId }),
  });

// Shared Assets
export interface SharedAsset {
  id: string;
  name: string;
  description: string;
  author: string;
  asset_type: string;
  payload: Record<string, unknown>;
  downloads: number;
  rating_avg: number;
  rating_count: number;
  installed: boolean;
  created_at: string;
}

export const getSharedAssets = (opts?: { deviceId?: string; assetType?: string; search?: string; sort?: string }) => {
  const params = new URLSearchParams();
  if (opts?.deviceId) params.set('device_id', opts.deviceId);
  if (opts?.assetType) params.set('asset_type', opts.assetType);
  if (opts?.search) params.set('search', opts.search);
  if (opts?.sort) params.set('sort', opts.sort);
  const qs = params.toString();
  return request<SharedAsset[]>(`/community/assets${qs ? `?${qs}` : ''}`);
};

export const publishAsset = (data: {
  name: string;
  description?: string;
  author?: string;
  asset_type: string;
  payload: Record<string, unknown>;
}) => request<SharedAsset>('/community/assets', { method: 'POST', body: JSON.stringify(data) });

export const installAsset = (assetId: string, deviceId?: string) =>
  request<{ ok: boolean }>(`/community/assets/${assetId}/install`, {
    method: 'POST',
    body: JSON.stringify({ device_id: deviceId }),
  });

export const uninstallAsset = (assetId: string, deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<{ ok: boolean }>(`/community/assets/${assetId}/install${params}`, { method: 'DELETE' });
};

export const rateAsset = (assetId: string, stars: number, deviceId?: string) =>
  request<{ ok: boolean }>(`/community/assets/${assetId}/rate`, {
    method: 'POST',
    body: JSON.stringify({ stars, device_id: deviceId }),
  });
