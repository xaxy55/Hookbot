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
    credentials: 'include',
    ...options,
  });
  if (res.status === 401 && !path.startsWith('/auth/')) {
    window.location.href = '/login';
    throw new Error('Unauthorized');
  }
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(body.error || res.statusText);
  }
  return res.json();
}

// Auth
export const login = (password: string) =>
  request<{ ok: boolean }>('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ password }),
  });

export const logout = () =>
  request<{ ok: boolean }>('/auth/logout', { method: 'POST' });

export const getAuthStatus = () =>
  request<{ authenticated: boolean; workos_enabled?: boolean }>('/auth/status');

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

// Config export/import
export interface ConfigExportData {
  metadata: { export_date: string; firmware_version: string | null };
  device_info: {
    name: string;
    hostname: string;
    purpose: string | null;
    personality: string | null;
    device_type: string | null;
  };
  device_config: DeviceConfig;
  servo_config: Record<string, unknown> | null;
  sensor_configs: SensorChannelConfig[];
  automation_rules: {
    name: string;
    enabled: boolean;
    trigger_type: string;
    trigger_config: Record<string, unknown>;
    action_type: string;
    action_config: Record<string, unknown>;
    cooldown_secs: number;
  }[];
}

export const exportDeviceConfig = (id: string) =>
  request<ConfigExportData>(`/devices/${id}/config/export`);

export const importDeviceConfig = (id: string, data: ConfigExportData) =>
  request<{ ok: boolean; imported: Record<string, unknown> }>(`/devices/${id}/config/import`, {
    method: 'POST',
    body: JSON.stringify(data),
  });

// Animation
export interface AnimationKeyframe {
  time: number;
  eyeX: number; eyeY: number; eyeOpen: number;
  mouthCurve: number; mouthOpen: number;
  bounce: number; shake: number;
  browAngle: number; browY: number;
}

export interface AnimationPayload {
  frames: AnimationKeyframe[];
  loop: boolean;
  duration_ms: number;
}

export const playAnimation = (deviceId: string, animation: AnimationPayload) =>
  request<{ ok: boolean }>(`/devices/${deviceId}/animation`, {
    method: 'POST',
    body: JSON.stringify(animation),
  });

export const stopAnimation = (deviceId: string) =>
  request<{ ok: boolean }>(`/devices/${deviceId}/animation/stop`, {
    method: 'POST',
  });

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

  const res = await fetch(`${BASE}/firmware`, { method: 'POST', body: form, credentials: 'include' });
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

// Sensor Readings
export interface SensorReading {
  id: number;
  device_id: string;
  channel: number;
  value: number;
  recorded_at: string;
}

export const getSensorReadings = (deviceId: string, channel?: number, hours?: number) => {
  const params = new URLSearchParams();
  if (channel !== undefined) params.set('channel', String(channel));
  if (hours !== undefined) params.set('hours', String(hours));
  const qs = params.toString();
  return request<SensorReading[]>(`/devices/${deviceId}/sensors/readings${qs ? `?${qs}` : ''}`);
};

export const getLatestSensorReadings = (deviceId: string) =>
  request<SensorReading[]>(`/devices/${deviceId}/sensors/readings/latest`);

export const purgeSensorReadings = (deviceId: string) =>
  request<{ ok: boolean; deleted: number }>(`/devices/${deviceId}/sensors/readings`, { method: 'DELETE' });

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

// Pet / Token tracking
export interface PetStoreItem {
  id: number;
  name: string;
  unlocked: boolean;
  cost: number;
  progress: number;
}

export interface PetState {
  device_id: string;
  hunger: number;
  happiness: number;
  last_fed_at: string | null;
  last_pet_at: string | null;
  total_feeds: number;
  total_pets: number;
  mood: string;
  active_pet: string;
  active_pet_id: number;
  store: PetStoreItem[];
}

export interface TokenUsageEntry {
  id: number;
  device_id: string;
  input_tokens: number;
  output_tokens: number;
  model: string;
  recorded_at: string;
}

export interface DailyTokenUsage {
  date: string;
  input_tokens: number;
  output_tokens: number;
}

export interface TokenUsageSummary {
  total_input: number;
  total_output: number;
  total_tokens: number;
  today_input: number;
  today_output: number;
  today_total: number;
  entries_count: number;
  recent: TokenUsageEntry[];
  daily: DailyTokenUsage[];
}

export const getPetState = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<PetState>(`/pet${params}`);
};

export const feedPet = (foodType?: string, deviceId?: string) =>
  request<{ ok: boolean; hunger: number; happiness: number; mood: string; message: string }>('/pet/feed', {
    method: 'POST',
    body: JSON.stringify({ food_type: foodType, device_id: deviceId }),
  });

export const petPet = (deviceId?: string) =>
  request<{ ok: boolean; hunger: number; happiness: number; mood: string; message: string }>('/pet/pet', {
    method: 'POST',
    body: JSON.stringify({ device_id: deviceId }),
  });

export const selectPet = (petId: number, deviceId?: string) =>
  request<{ ok: boolean; active_pet: string; active_pet_id: number }>('/pet/select', {
    method: 'POST',
    body: JSON.stringify({ id: petId, device_id: deviceId }),
  });

// Pomodoro sync
export interface PomodoroState {
  session: 'focus' | 'shortBreak' | 'longBreak';
  status: 'idle' | 'running' | 'paused';
  time_left: number;
  total_duration: number;
  focus_count: number;
  today_sessions: number;
  today_minutes: number;
  config: { focus: number; short_break: number; long_break: number };
}

export const getPomodoroState = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<PomodoroState>(`/pomodoro${params}`);
};

export const sendPomodoroAction = (action: string, data?: Record<string, unknown>, deviceId?: string) =>
  request<{ ok: boolean; session: string; status: string; time_left: number }>('/pomodoro', {
    method: 'POST',
    body: JSON.stringify({ action, device_id: deviceId, ...data }),
  });

export const getTokenUsage = (deviceId?: string, days?: number) => {
  const params = new URLSearchParams();
  if (deviceId) params.set('device_id', deviceId);
  if (days) params.set('days', String(days));
  const qs = params.toString();
  return request<TokenUsageSummary>(`/pet/tokens${qs ? `?${qs}` : ''}`);
};

export const recordTokenUsage = (data: {
  device_id?: string;
  input_tokens: number;
  output_tokens: number;
  model?: string;
}) => request<{ ok: boolean }>('/pet/tokens', { method: 'POST', body: JSON.stringify(data) });

// Mood Journal
export interface MoodEntry {
  id: number;
  device_id: string;
  mood: string;
  note: string | null;
  energy: number;
  created_at: string;
}

export interface MoodStats {
  total_entries: number;
  this_week: number;
  avg_energy: number;
  most_common_mood: string | null;
  mood_distribution: { mood: string; count: number }[];
}

export const getMoodEntries = (deviceId?: string, days?: number) => {
  const params = new URLSearchParams();
  if (deviceId) params.set('device_id', deviceId);
  if (days) params.set('days', String(days));
  const qs = params.toString();
  return request<MoodEntry[]>(`/mood${qs ? `?${qs}` : ''}`);
};

export const createMoodEntry = (data: { device_id?: string; mood: string; note?: string; energy?: number }) =>
  request<MoodEntry>('/mood', { method: 'POST', body: JSON.stringify(data) });

export const getMoodStats = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<MoodStats>(`/mood/stats${params}`);
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
  verified: boolean;
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
  verified: boolean;
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

// Verified Publishers
export interface VerifiedPublisher {
  id: string;
  name: string;
  display_name: string;
  badge_type: string;
  verified_at: string;
  verified_by: string | null;
}

export const getVerifiedPublishers = () =>
  request<VerifiedPublisher[]>('/community/publishers');

export const addVerifiedPublisher = (data: {
  name: string;
  display_name: string;
  badge_type?: string;
}) => request<VerifiedPublisher>('/community/publishers', {
  method: 'POST',
  body: JSON.stringify(data),
});

export const removeVerifiedPublisher = (id: string) =>
  request<{ ok: boolean }>(`/community/publishers/${id}`, { method: 'DELETE' });

// Project Routes
export interface ProjectRoute {
  id: string;
  project_path: string;
  device_id: string;
  label: string | null;
  created_at: string;
  device_name: string | null;
}

export const getProjectRoutes = () => request<ProjectRoute[]>('/routes');

export const createProjectRoute = (data: {
  project_path: string;
  device_id: string;
  label?: string;
}) => request<ProjectRoute>('/routes', { method: 'POST', body: JSON.stringify(data) });

export const updateProjectRoute = (id: string, data: {
  project_path?: string;
  device_id?: string;
  label?: string;
}) => request<ProjectRoute>(`/routes/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteProjectRoute = (id: string) =>
  request<{ ok: boolean }>(`/routes/${id}`, { method: 'DELETE' });

// Device Groups
export interface DeviceGroup {
  id: string;
  name: string;
  color: string;
  created_at: string;
  device_ids: string[];
}

export const getGroups = () => request<DeviceGroup[]>('/groups');

export const createGroup = (data: { name: string; color?: string }) =>
  request<DeviceGroup>('/groups', { method: 'POST', body: JSON.stringify(data) });

export const updateGroup = (id: string, data: { name?: string; color?: string }) =>
  request<DeviceGroup>(`/groups/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteGroup = (id: string) =>
  request<{ ok: boolean }>(`/groups/${id}`, { method: 'DELETE' });

export const addGroupMember = (groupId: string, deviceId: string) =>
  request<DeviceGroup>(`/groups/${groupId}/members`, {
    method: 'POST',
    body: JSON.stringify({ device_id: deviceId }),
  });

export const removeGroupMember = (groupId: string, deviceId: string) =>
  request<{ ok: boolean }>(`/groups/${groupId}/members/${deviceId}`, { method: 'DELETE' });

export const sendGroupState = (groupId: string, state: string) =>
  request<{ ok: boolean; successes: number; failures: number }>(`/groups/${groupId}/state`, {
    method: 'POST',
    body: JSON.stringify({ state }),
  });

export const sendGroupCommand = (groupId: string, endpoint: string, body: Record<string, unknown>) =>
  request<{ ok: boolean; successes: number; failures: number }>(`/groups/${groupId}/command`, {
    method: 'POST',
    body: JSON.stringify({ endpoint, body }),
  });

// Plugin Sandboxing
export interface PluginSandbox {
  id: number;
  plugin_id: string;
  device_id: string;
  allowed_apis: string[];
  blocked_apis: string[];
  max_calls_per_minute: number;
  can_access_network: boolean;
  can_modify_state: boolean;
  can_send_notifications: boolean;
  can_access_sensors: boolean;
  enabled: boolean;
  created_at: string;
}

export const getPluginSandboxes = (opts?: { deviceId?: string; pluginId?: string }) => {
  const params = new URLSearchParams();
  if (opts?.deviceId) params.set('device_id', opts.deviceId);
  if (opts?.pluginId) params.set('plugin_id', opts.pluginId);
  const qs = params.toString();
  return request<PluginSandbox[]>(`/community/sandboxes${qs ? `?${qs}` : ''}`);
};

export const createPluginSandbox = (data: {
  plugin_id: string;
  device_id?: string;
  allowed_apis?: string[];
  blocked_apis?: string[];
  max_calls_per_minute?: number;
  can_access_network?: boolean;
  can_modify_state?: boolean;
  can_send_notifications?: boolean;
  can_access_sensors?: boolean;
}) => request<PluginSandbox>('/community/sandboxes', { method: 'POST', body: JSON.stringify(data) });

export const updatePluginSandbox = (id: number, data: Partial<Omit<PluginSandbox, 'id' | 'plugin_id' | 'device_id' | 'created_at'>>) =>
  request<{ ok: boolean }>(`/community/sandboxes/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deletePluginSandbox = (id: number) =>
  request<{ ok: boolean }>(`/community/sandboxes/${id}`, { method: 'DELETE' });

// Device-to-Device Links
export interface DeviceLink {
  id: string;
  source_device_id: string;
  target_device_id: string;
  trigger_type: string;
  trigger_config: Record<string, unknown>;
  action_type: string;
  action_config: Record<string, unknown>;
  enabled: boolean;
  cooldown_secs: number;
  last_triggered_at: string | null;
  created_at: string;
  source_device_name: string | null;
  target_device_name: string | null;
}

export const getDeviceLinks = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<DeviceLink[]>(`/device-links${params}`);
};

export const createDeviceLink = (data: {
  source_device_id: string;
  target_device_id: string;
  trigger_type: string;
  trigger_config?: Record<string, unknown>;
  action_type: string;
  action_config?: Record<string, unknown>;
  cooldown_secs?: number;
}) => request<DeviceLink>('/device-links', { method: 'POST', body: JSON.stringify(data) });

export const updateDeviceLink = (id: string, data: Partial<DeviceLink>) =>
  request<DeviceLink>(`/device-links/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteDeviceLink = (id: string) =>
  request<{ ok: boolean }>(`/device-links/${id}`, { method: 'DELETE' });

// User Management
export interface UserWithDevices {
  id: string;
  username: string;
  display_name: string;
  role: string;
  last_login_at: string | null;
  created_at: string;
  device_ids: string[];
}

export const getUsers = () => request<UserWithDevices[]>('/users');

export const createUser = (data: { username: string; display_name: string; password: string; role?: string }) =>
  request<UserWithDevices>('/users', { method: 'POST', body: JSON.stringify(data) });

export const getUser = (id: string) => request<UserWithDevices>(`/users/${id}`);

export const updateUser = (id: string, data: { display_name?: string; password?: string; role?: string }) =>
  request<UserWithDevices>(`/users/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteUser = (id: string) =>
  request<{ ok: boolean }>(`/users/${id}`, { method: 'DELETE' });

export const assignDeviceToUser = (userId: string, deviceId: string, permissions?: string) =>
  request<UserWithDevices>(`/users/${userId}/devices`, {
    method: 'POST',
    body: JSON.stringify({ device_id: deviceId, permissions }),
  });

export const unassignDeviceFromUser = (userId: string, deviceId: string) =>
  request<{ ok: boolean }>(`/users/${userId}/devices/${deviceId}`, { method: 'DELETE' });

// Tunnels / Remote Access
export interface TunnelConfig {
  id: string;
  name: string;
  tunnel_type: string;
  hostname: string | null;
  port: number;
  status: string;
  last_connected_at: string | null;
  config: Record<string, unknown>;
  created_at: string;
}

export const getTunnels = () => request<TunnelConfig[]>('/tunnels');

export const createTunnel = (data: {
  name: string;
  tunnel_type?: string;
  hostname?: string;
  port?: number;
  auth_token?: string;
  config?: Record<string, unknown>;
}) => request<TunnelConfig>('/tunnels', { method: 'POST', body: JSON.stringify(data) });

export const updateTunnel = (id: string, data: Partial<TunnelConfig>) =>
  request<TunnelConfig>(`/tunnels/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteTunnel = (id: string) =>
  request<{ ok: boolean }>(`/tunnels/${id}`, { method: 'DELETE' });

export const startTunnel = (id: string) =>
  request<{ ok: boolean; status: string; message: string; hostname: string | null }>(`/tunnels/${id}/start`, { method: 'POST' });

export const stopTunnel = (id: string) =>
  request<{ ok: boolean; status: string }>(`/tunnels/${id}/stop`, { method: 'POST' });

// Voice Control
export interface VoiceCommand {
  id: number;
  device_id: string;
  audio_size: number;
  duration_secs: number;
  transcript: string;
  response: string;
  status: string;
  created_at: string;
}

export interface VoiceConfig {
  device_id: string;
  wake_word_enabled: boolean;
  tts_enabled: boolean;
  tts_voice: string;
  volume: number;
  language: string;
}

export interface VoiceResponse {
  ok: boolean;
  transcript: string;
  response: string;
  state: string | null;
  tts_url: string | null;
}

export const getVoiceHistory = (deviceId?: string, limit?: number) => {
  const params = new URLSearchParams();
  if (deviceId) params.set('device_id', deviceId);
  if (limit) params.set('limit', String(limit));
  const qs = params.toString();
  return request<VoiceCommand[]>(`/voice/history${qs ? `?${qs}` : ''}`);
};

export const getVoiceConfig = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<VoiceConfig>(`/voice/config${params}`);
};

export const updateVoiceConfig = (data: {
  device_id?: string;
  wake_word_enabled?: boolean;
  tts_enabled?: boolean;
  tts_voice?: string;
  volume?: number;
  language?: string;
}) => request<{ ok: boolean }>('/voice/config', { method: 'PUT', body: JSON.stringify(data) });

export const sendVoiceCommand = (text: string, deviceId?: string) =>
  request<VoiceResponse>('/voice/command', {
    method: 'POST',
    body: JSON.stringify({ text, device_id: deviceId }),
  });

export const requestTts = (text: string, deviceId?: string, voice?: string) =>
  request<{ ok: boolean; text: string; audio_url: string | null; duration_secs: number | null; format: string }>('/voice/tts', {
    method: 'POST',
    body: JSON.stringify({ text, device_id: deviceId, voice }),
  });

// Mood Learning
export interface MoodPreference {
  id: number;
  device_id: string;
  state: string;
  animation_id: string | null;
  positive_responses: number;
  negative_responses: number;
  total_duration_secs: number;
  score: number;
  last_shown_at: string | null;
}

export interface MoodPattern {
  device_id: string;
  hour_of_day: number;
  day_of_week: number;
  preferred_state: string | null;
  preferred_animation: string | null;
  confidence: number;
  sample_count: number;
}

export interface MoodSuggestion {
  suggested_state: string | null;
  suggested_animation: string | null;
  confidence: number;
  reason: string;
}

export const recordMoodFeedback = (data: {
  device_id?: string;
  state: string;
  animation_id?: string;
  feedback: 'positive' | 'negative';
  duration_secs?: number;
}) => request<{ ok: boolean }>('/mood/feedback', { method: 'POST', body: JSON.stringify(data) });

export const getMoodPreferences = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<MoodPreference[]>(`/mood/preferences${params}`);
};

export const getMoodPatterns = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<MoodPattern[]>(`/mood/patterns${params}`);
};

export const getMoodSuggestion = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<MoodSuggestion>(`/mood/suggest${params}`);
};

// --- Phase 8: Desk Ecosystem & Smart Home ---

// Desk Lights
export interface DeskLightConfig {
  id: string;
  device_id: string;
  provider: string;
  name: string;
  bridge_ip: string | null;
  api_key: string | null;
  light_ids: string[];
  state_colors: Record<string, string>;
  enabled: boolean;
  created_at: string;
}

export const getDeskLights = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<DeskLightConfig[]>(`/desk-lights${params}`);
};

export const createDeskLight = (data: {
  device_id?: string;
  provider: string;
  name: string;
  bridge_ip?: string;
  api_key?: string;
  light_ids?: string[];
  state_colors?: Record<string, string>;
}) => request<DeskLightConfig>('/desk-lights', { method: 'POST', body: JSON.stringify(data) });

export const updateDeskLight = (id: string, data: {
  name?: string;
  bridge_ip?: string;
  api_key?: string;
  light_ids?: string[];
  state_colors?: Record<string, string>;
  enabled?: boolean;
}) => request<{ ok: boolean }>(`/desk-lights/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteDeskLight = (id: string) =>
  request<{ ok: boolean }>(`/desk-lights/${id}`, { method: 'DELETE' });

export const triggerDeskLightAction = (id: string, data: {
  color?: string;
  brightness?: number;
  effect?: string;
}) => request<{ ok: boolean }>(`/desk-lights/${id}/action`, { method: 'POST', body: JSON.stringify(data) });

// Music Integration
export interface MusicConfig {
  id: string;
  device_id: string;
  provider: string;
  access_token: string | null;
  refresh_token: string | null;
  auto_pause_meetings: boolean;
  focus_playlist_id: string | null;
  enabled: boolean;
  created_at: string;
}

export interface NowPlaying {
  is_playing: boolean;
  track_name: string | null;
  artist_name: string | null;
  album_name: string | null;
  album_art_url: string | null;
  progress_ms: number | null;
  duration_ms: number | null;
}

export const getMusicConfig = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<MusicConfig[]>(`/music/config${params}`);
};

export const createMusicConfig = (data: {
  device_id?: string;
  provider: string;
  access_token?: string;
  refresh_token?: string;
  focus_playlist_id?: string;
}) => request<MusicConfig>('/music/config', { method: 'POST', body: JSON.stringify(data) });

export const updateMusicConfig = (id: string, data: {
  access_token?: string;
  auto_pause_meetings?: boolean;
  focus_playlist_id?: string;
  enabled?: boolean;
}) => request<{ ok: boolean }>(`/music/config/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteMusicConfig = (id: string) =>
  request<{ ok: boolean }>(`/music/config/${id}`, { method: 'DELETE' });

export const getNowPlaying = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<NowPlaying>(`/music/now-playing${params}`);
};

export const musicAction = (action: string, deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<{ ok: boolean }>(`/music/action${params}`, { method: 'POST', body: JSON.stringify({ action }) });
};

// Standing Desk
export interface StandingDeskConfig {
  id: string;
  device_id: string;
  sit_remind_minutes: number;
  stand_remind_minutes: number;
  enabled: boolean;
  current_position: string;
  total_stand_minutes: number;
  total_sit_minutes: number;
  transitions_today: number;
  last_transition_at: string | null;
  created_at: string;
}

export interface DeskHealthReport {
  total_stand_minutes: number;
  total_sit_minutes: number;
  stand_ratio: number;
  transitions_today: number;
  daily_history: { date: string; stand_minutes: number; sit_minutes: number; transitions: number }[];
}

export const getStandingDesk = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<StandingDeskConfig>(`/standing-desk${params}`);
};

export const updateStandingDesk = (data: {
  sit_remind_minutes?: number;
  stand_remind_minutes?: number;
  enabled?: boolean;
}, deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<{ ok: boolean }>(`/standing-desk${params}`, { method: 'PUT', body: JSON.stringify(data) });
};

export const changePosition = (position: string, deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<{ ok: boolean; celebration: boolean; message: string }>(`/standing-desk/position${params}`, {
    method: 'POST', body: JSON.stringify({ position }),
  });
};

export const getDeskReport = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<DeskHealthReport>(`/standing-desk/report${params}`);
};

// Stream Deck
export interface StreamDeckButton {
  id: string;
  device_id: string;
  position: number;
  label: string;
  icon: string | null;
  action_type: string;
  action_config: Record<string, unknown>;
  enabled: boolean;
  created_at: string;
}

export const getStreamDeckButtons = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<StreamDeckButton[]>(`/streamdeck/buttons${params}`);
};

export const createStreamDeckButton = (data: {
  device_id?: string;
  position: number;
  label: string;
  icon?: string;
  action_type: string;
  action_config?: Record<string, unknown>;
}) => request<StreamDeckButton>('/streamdeck/buttons', { method: 'POST', body: JSON.stringify(data) });

export const updateStreamDeckButton = (id: string, data: {
  label?: string;
  icon?: string;
  action_type?: string;
  action_config?: Record<string, unknown>;
  enabled?: boolean;
}) => request<{ ok: boolean }>(`/streamdeck/buttons/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteStreamDeckButton = (id: string) =>
  request<{ ok: boolean }>(`/streamdeck/buttons/${id}`, { method: 'DELETE' });

export const triggerStreamDeckButton = (buttonId: string) =>
  request<{ ok: boolean }>('/streamdeck/trigger', { method: 'POST', body: JSON.stringify({ button_id: buttonId }) });

// Home Assistant
export interface HomeAssistantConfig {
  id: string;
  device_id: string;
  ha_url: string;
  access_token: string | null;
  entity_id: string | null;
  expose_states: boolean;
  expose_sensors: boolean;
  enabled: boolean;
  created_at: string;
}

export interface HomeAssistantEntity {
  entity_id: string;
  state: string;
  attributes: Record<string, unknown>;
}

export const getHomeAssistantConfig = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<HomeAssistantConfig | null>(`/homeassistant${params}`);
};

export const createHomeAssistantConfig = (data: {
  device_id?: string;
  ha_url: string;
  access_token?: string;
  entity_id?: string;
}) => request<HomeAssistantConfig>('/homeassistant', { method: 'POST', body: JSON.stringify(data) });

export const updateHomeAssistantConfig = (id: string, data: {
  ha_url?: string;
  access_token?: string;
  entity_id?: string;
  expose_states?: boolean;
  expose_sensors?: boolean;
  enabled?: boolean;
}) => request<{ ok: boolean }>(`/homeassistant/${id}`, { method: 'PUT', body: JSON.stringify(data) });

export const deleteHomeAssistantConfig = (id: string) =>
  request<{ ok: boolean }>(`/homeassistant/${id}`, { method: 'DELETE' });

export const getHomeAssistantEntity = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<HomeAssistantEntity>(`/homeassistant/entity${params}`);
};

export const syncHomeAssistant = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<{ ok: boolean }>(`/homeassistant/sync${params}`, { method: 'POST' });
};

// Desk Occupancy
export interface DeskOccupancyConfig {
  id: string;
  device_id: string;
  break_remind_minutes: number;
  enabled: boolean;
  created_at: string;
}

export interface OccupancyEvent {
  id: number;
  device_id: string;
  event_type: string;
  created_at: string;
}

export interface OccupancyReport {
  total_desk_hours: number;
  total_break_hours: number;
  avg_session_minutes: number;
  breaks_taken: number;
  optimal_break_suggestion: string;
  daily_stats: { date: string; desk_hours: number; break_count: number; longest_session_minutes: number }[];
}

export const getDeskOccupancyConfig = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<DeskOccupancyConfig>(`/desk-occupancy/config${params}`);
};

export const updateDeskOccupancyConfig = (data: {
  break_remind_minutes?: number;
  enabled?: boolean;
}, deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<{ ok: boolean }>(`/desk-occupancy/config${params}`, { method: 'PUT', body: JSON.stringify(data) });
};

export const recordOccupancyEvent = (eventType: string, deviceId?: string) =>
  request<OccupancyEvent>('/desk-occupancy/events', {
    method: 'POST', body: JSON.stringify({ device_id: deviceId, event_type: eventType }),
  });

export const getOccupancyEvents = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<OccupancyEvent[]>(`/desk-occupancy/events${params}`);
};

export const getOccupancyReport = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<OccupancyReport>(`/desk-occupancy/report${params}`);
};

// Multi-Monitor
export interface MonitorConfig {
  id: string;
  device_id: string;
  monitor_count: number;
  servo_pin: number | null;
  angle_map: Record<string, number>;
  detection_method: string;
  enabled: boolean;
  active_monitor: number;
  created_at: string;
}

export const getMonitorConfig = (deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<MonitorConfig>(`/monitors${params}`);
};

export const updateMonitorConfig = (data: {
  monitor_count?: number;
  servo_pin?: number;
  angle_map?: Record<string, number>;
  detection_method?: string;
  enabled?: boolean;
}, deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<{ ok: boolean }>(`/monitors${params}`, { method: 'PUT', body: JSON.stringify(data) });
};

export const setActiveMonitor = (monitor: number, deviceId?: string) => {
  const params = deviceId ? `?device_id=${deviceId}` : '';
  return request<{ ok: boolean; target_angle: number }>(`/monitors/active${params}`, {
    method: 'POST', body: JSON.stringify({ monitor }),
  });
};
