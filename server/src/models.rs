use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub workos_id: String,
    pub email: String,
    pub name: String,
    pub api_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub ip_address: String,
    pub purpose: Option<String>,
    pub personality: Option<String>,
    pub device_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub device_id: String,
    pub led_brightness: i32,
    pub led_colors: Option<serde_json::Value>,
    pub sound_enabled: bool,
    pub sound_volume: i32,
    pub avatar_preset: Option<serde_json::Value>,
    pub custom_data: Option<serde_json::Value>,
    pub sound_pack: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Firmware {
    pub id: String,
    pub version: String,
    pub filename: String,
    pub size_bytes: i64,
    pub checksum: String,
    pub uploaded_at: String,
    pub notes: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtaJob {
    pub id: String,
    pub firmware_id: String,
    pub device_id: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub error_msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct StatusLog {
    pub device_id: String,
    pub state: String,
    pub uptime_ms: i64,
    pub free_heap: i64,
    pub recorded_at: String,
}

// --- API request/response types ---

#[derive(Debug, Deserialize)]
pub struct CreateDevice {
    pub name: String,
    pub hostname: String,
    pub ip_address: String,
    pub purpose: Option<String>,
    pub personality: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDevice {
    pub name: Option<String>,
    pub hostname: Option<String>,
    pub ip_address: Option<String>,
    pub purpose: Option<String>,
    pub personality: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfig {
    pub led_brightness: Option<i32>,
    pub led_colors: Option<serde_json::Value>,
    pub sound_enabled: Option<bool>,
    pub sound_volume: Option<i32>,
    pub avatar_preset: Option<serde_json::Value>,
    pub custom_data: Option<serde_json::Value>,
    pub sound_pack: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StateChange {
    pub state: String,
    pub tool: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OtaDeploy {
    pub firmware_id: String,
    pub device_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub label: String,
    pub status: u8, // 0=pending, 1=active, 2=done, 3=failed
}

#[derive(Debug, Deserialize)]
pub struct HookEvent {
    pub event: String,
    pub tool_name: Option<String>,
    pub tool_output: Option<String>,
    #[allow(dead_code)]
    pub project: Option<String>,
    pub device_id: Option<String>,
    pub tasks: Option<Vec<TaskItem>>,
    pub active_task: Option<u8>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubHookQuery {
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceWithStatus {
    #[serde(flatten)]
    pub device: Device,
    pub latest_status: Option<StatusSnapshot>,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSnapshot {
    pub state: String,
    pub uptime_ms: i64,
    pub free_heap: i64,
    pub recorded_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub hostname: String,
    pub ip_address: String,
    pub port: u16,
    pub already_registered: bool,
}

// --- Gamification types ---

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    pub id: i64,
    pub device_id: Option<String>,
    pub tool_name: String,
    pub event: String,
    pub project: Option<String>,
    pub duration_ms: Option<i64>,
    pub xp_earned: i64,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub device_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub tool_count: i64,
    pub xp_earned: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XpEntry {
    pub id: i64,
    pub device_id: Option<String>,
    pub amount: i64,
    pub reason: String,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: i64,
    pub device_id: Option<String>,
    pub badge_id: String,
    pub earned_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Streak {
    pub device_id: String,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub last_active_date: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct GamificationStats {
    pub total_xp: i64,
    pub level: i64,
    pub xp_for_current_level: i64,
    pub xp_for_next_level: i64,
    pub total_tool_uses: i64,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub achievements_earned: i64,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct ActivityEntry {
    pub id: i64,
    pub tool_name: String,
    pub event: String,
    pub xp_earned: i64,
    pub created_at: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsData {
    pub tools_per_day: Vec<DayCount>,
    pub hourly_activity: Vec<HourCount>,
    pub tool_distribution: Vec<ToolCount>,
    pub state_distribution: Vec<StateCount>,
    pub session_lengths: Vec<SessionLength>,
    pub xp_over_time: Vec<DayXp>,
}

#[derive(Debug, Serialize)]
pub struct DayCount {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct HourCount {
    pub hour: i64,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ToolCount {
    pub tool_name: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct StateCount {
    pub state: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionLength {
    pub date: String,
    pub duration_minutes: f64,
}

#[derive(Debug, Serialize)]
pub struct DayXp {
    pub date: String,
    pub xp: i64,
}

#[derive(Debug, Serialize)]
pub struct BadgeDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub earned: bool,
    pub earned_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LeaderboardEntry {
    pub device_id: String,
    pub device_name: String,
    pub total_xp: i64,
    pub level: i64,
    pub current_streak: i64,
    pub achievements: i64,
}

// --- Notification types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: i64,
    pub device_id: String,
    pub source: String,
    pub unread: i64,
    pub message: Option<String>,
    pub delivered: bool,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotification {
    pub source: String,
    pub unread: Option<i64>,
    pub active: Option<bool>,
    pub message: Option<String>,
}

// --- Sensor types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    pub id: i64,
    pub device_id: String,
    pub channel: i32,
    pub pin: i32,
    pub sensor_type: String,
    pub label: Option<String>,
    pub poll_interval_ms: i32,
    pub threshold: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSensorConfig {
    pub channels: Vec<SensorChannelUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SensorChannelUpdate {
    pub channel: i32,
    pub pin: i32,
    pub sensor_type: String,
    pub label: Option<String>,
    pub poll_interval_ms: Option<i32>,
    pub threshold: Option<i32>,
}

// --- Sensor reading types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub id: i64,
    pub device_id: String,
    pub channel: i32,
    pub value: f64,
    pub recorded_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SensorReadingsQuery {
    pub channel: Option<i32>,
    pub hours: Option<u64>,
}

// --- Automation types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: String,
    pub device_id: String,
    pub name: String,
    pub enabled: bool,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub action_type: String,
    pub action_config: serde_json::Value,
    pub cooldown_secs: i64,
    pub last_triggered_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRule {
    pub name: String,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub action_type: String,
    pub action_config: serde_json::Value,
    pub cooldown_secs: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRule {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<serde_json::Value>,
    pub action_type: Option<String>,
    pub action_config: Option<serde_json::Value>,
    pub cooldown_secs: Option<i64>,
}

// --- Project routing types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRoute {
    pub id: String,
    pub project_path: String,
    pub device_id: String,
    pub label: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRouteWithDevice {
    pub id: String,
    pub project_path: String,
    pub device_id: String,
    pub label: Option<String>,
    pub created_at: String,
    pub device_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRoute {
    pub project_path: String,
    pub device_id: String,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRoute {
    pub project_path: Option<String>,
    pub device_id: Option<String>,
    pub label: Option<String>,
}

// --- Device Group types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
    pub device_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroup {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroup {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddGroupMember {
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupStateRequest {
    pub state: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupCommandRequest {
    pub endpoint: String,
    pub body: serde_json::Value,
}

// --- Verified publisher types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedPublisher {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub badge_type: String,
    pub verified_at: String,
    pub verified_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateVerifiedPublisher {
    pub name: String,
    pub display_name: String,
    pub badge_type: Option<String>,
}

// --- Plugin Sandbox types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSandbox {
    pub id: i64,
    pub plugin_id: String,
    pub device_id: String,
    pub allowed_apis: Vec<String>,
    pub blocked_apis: Vec<String>,
    pub max_calls_per_minute: i64,
    pub can_access_network: bool,
    pub can_modify_state: bool,
    pub can_send_notifications: bool,
    pub can_access_sensors: bool,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePluginSandbox {
    pub plugin_id: String,
    pub device_id: Option<String>,
    pub allowed_apis: Option<Vec<String>>,
    pub blocked_apis: Option<Vec<String>>,
    pub max_calls_per_minute: Option<i64>,
    pub can_access_network: Option<bool>,
    pub can_modify_state: Option<bool>,
    pub can_send_notifications: Option<bool>,
    pub can_access_sensors: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePluginSandbox {
    pub allowed_apis: Option<Vec<String>>,
    pub blocked_apis: Option<Vec<String>>,
    pub max_calls_per_minute: Option<i64>,
    pub can_access_network: Option<bool>,
    pub can_modify_state: Option<bool>,
    pub can_send_notifications: Option<bool>,
    pub can_access_sensors: Option<bool>,
    pub enabled: Option<bool>,
}

// --- Device Link types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLink {
    pub id: String,
    pub source_device_id: String,
    pub target_device_id: String,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub action_type: String,
    pub action_config: serde_json::Value,
    pub enabled: bool,
    pub cooldown_secs: i64,
    pub last_triggered_at: Option<String>,
    pub created_at: String,
    pub source_device_name: Option<String>,
    pub target_device_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceLink {
    pub source_device_id: String,
    pub target_device_id: String,
    pub trigger_type: String,
    pub trigger_config: Option<serde_json::Value>,
    pub action_type: String,
    pub action_config: Option<serde_json::Value>,
    pub cooldown_secs: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceLink {
    pub trigger_type: Option<String>,
    pub trigger_config: Option<serde_json::Value>,
    pub action_type: Option<String>,
    pub action_config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub cooldown_secs: Option<i64>,
}

// --- User types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub last_login_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub display_name: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserWithDevices {
    #[serde(flatten)]
    pub user: LocalUser,
    pub device_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignDevice {
    pub device_id: String,
    pub permissions: Option<String>,
}

// --- Tunnel types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub id: String,
    pub name: String,
    pub tunnel_type: String,
    pub hostname: Option<String>,
    pub port: i64,
    pub status: String,
    pub last_connected_at: Option<String>,
    pub config: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTunnel {
    pub name: String,
    pub tunnel_type: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<i64>,
    pub auth_token: Option<String>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTunnel {
    pub name: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<i64>,
    pub auth_token: Option<String>,
    pub config: Option<serde_json::Value>,
}

// --- Mood Learning types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodPreference {
    pub id: i64,
    pub device_id: String,
    pub state: String,
    pub animation_id: Option<String>,
    pub positive_responses: i64,
    pub negative_responses: i64,
    pub total_duration_secs: i64,
    pub score: f64,
    pub last_shown_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodPattern {
    pub device_id: String,
    pub hour_of_day: i64,
    pub day_of_week: i64,
    pub preferred_state: Option<String>,
    pub preferred_animation: Option<String>,
    pub confidence: f64,
    pub sample_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct RecordMoodFeedback {
    pub device_id: Option<String>,
    pub state: String,
    pub animation_id: Option<String>,
    pub feedback: String, // "positive" or "negative"
    pub duration_secs: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MoodSuggestion {
    pub suggested_state: Option<String>,
    pub suggested_animation: Option<String>,
    pub confidence: f64,
    pub reason: String,
}

// --- Voice Control types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    pub id: i64,
    pub device_id: String,
    pub audio_size: i64,
    pub duration_secs: f64,
    pub transcript: String,
    pub response: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct VoiceResponse {
    pub ok: bool,
    pub transcript: String,
    pub response: String,
    pub state: Option<String>,
    pub tts_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TtsRequest {
    pub device_id: Option<String>,
    pub text: String,
    pub voice: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TtsResponse {
    pub ok: bool,
    pub text: String,
    pub audio_url: Option<String>,
    pub duration_secs: Option<f64>,
    pub format: String,
}

#[derive(Debug, Deserialize)]
pub struct VoiceCommandRequest {
    pub device_id: Option<String>,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct VoiceHistoryQuery {
    pub device_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub device_id: String,
    pub wake_word_enabled: bool,
    pub tts_enabled: bool,
    pub tts_voice: String,
    pub volume: i32,
    pub language: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVoiceConfig {
    pub device_id: Option<String>,
    pub wake_word_enabled: Option<bool>,
    pub tts_enabled: Option<bool>,
    pub tts_voice: Option<String>,
    pub volume: Option<i32>,
    pub language: Option<String>,
}

// --- Context types ---

#[derive(Debug, Serialize)]
pub struct DeviceContext {
    pub context: String,
    pub confidence: f64,
    pub recent_tools: Vec<String>,
}
