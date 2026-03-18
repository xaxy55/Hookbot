export interface Device {
  id: string;
  name: string;
  hostname: string;
  ip_address: string;
  purpose?: string;
  personality?: string;
  device_type?: string;
  created_at: string;
  updated_at: string;
}

export interface DeviceWithStatus extends Device {
  latest_status?: StatusSnapshot;
  online: boolean;
}

export interface StatusSnapshot {
  state: string;
  uptime_ms: number;
  free_heap: number;
  recorded_at: string;
}

export interface DeviceConfig {
  device_id: string;
  led_brightness: number;
  led_colors?: Record<string, string>;
  sound_enabled: boolean;
  sound_volume: number;
  avatar_preset?: Record<string, unknown>;
  custom_data?: Record<string, unknown>;
}

export interface Firmware {
  id: string;
  version: string;
  filename: string;
  size_bytes: number;
  checksum: string;
  uploaded_at: string;
  notes?: string;
  device_type?: string;
}

export interface OtaJob {
  id: string;
  firmware_id: string;
  device_id: string;
  status: 'pending' | 'in_progress' | 'success' | 'failed';
  created_at: string;
  updated_at: string;
  error_msg?: string;
}

export interface DiscoveredDevice {
  hostname: string;
  ip_address: string;
  port: number;
  already_registered: boolean;
}

export type AvatarState = 'idle' | 'thinking' | 'waiting' | 'success' | 'taskcheck' | 'error';

// --- Gamification types ---

export interface GamificationStats {
  total_xp: number;
  level: number;
  xp_for_current_level: number;
  xp_for_next_level: number;
  total_tool_uses: number;
  current_streak: number;
  longest_streak: number;
  achievements_earned: number;
  title: string;
}

export interface ActivityEntry {
  id: number;
  tool_name: string;
  event: string;
  xp_earned: number;
  created_at: string;
  device_id?: string;
}

export interface AnalyticsData {
  tools_per_day: { date: string; count: number }[];
  hourly_activity: { hour: number; count: number }[];
  tool_distribution: { tool_name: string; count: number }[];
  state_distribution: { state: string; count: number }[];
  session_lengths: { date: string; duration_minutes: number }[];
  xp_over_time: { date: string; xp: number }[];
}

export interface BadgeDefinition {
  id: string;
  name: string;
  description: string;
  icon: string;
  earned: boolean;
  earned_at?: string;
}

export interface LeaderboardEntry {
  device_id: string;
  device_name: string;
  total_xp: number;
  level: number;
  current_streak: number;
  achievements: number;
}
