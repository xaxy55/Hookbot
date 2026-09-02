// Registry of supported Hookbot boards.
//
// `device_type` is reported by the firmware on /info and /status, and is
// stamped onto firmware builds by the server (see server/src/routes/build.rs —
// the keys here must stay in sync with env_to_device_type there).

export interface BoardInfo {
  /** PlatformIO environment used to build for this board */
  env: string;
  /** Full name, used in build target pickers and firmware descriptions */
  label: string;
  /** Short badge text shown next to a device or firmware */
  badge: string;
  /** Badge classes for list/table rows */
  badgeClass: string;
  /** Badge classes for the device detail header */
  headerBadgeClass: string;
  /** Colour panel. False for the monochrome OLED, which has one ink colour. */
  color: boolean;
}

export const BOARDS: Record<string, BoardInfo> = {
  esp32_oled: {
    env: 'esp32',
    label: 'ESP32 OLED (128x64)',
    badge: 'OLED',
    badgeClass: 'border-amber-800 text-amber-400 bg-amber-900/20',
    headerBadgeClass: 'bg-amber-500/10 text-amber-400 border-amber-500/20',
    color: false,
  },
  esp32_4848s040c_lcd: {
    env: 'esp32-4848s040c',
    label: 'ESP32-S3 LCD (480x480 Touch)',
    badge: 'LCD Touch',
    badgeClass: 'border-cyan-800 text-cyan-400 bg-cyan-900/20',
    headerBadgeClass: 'bg-cyan-500/10 text-cyan-400 border-cyan-500/20',
    color: true,
  },
  xiao_c6_gc9a01_round: {
    env: 'xiao-c6-gc9a01',
    label: 'XIAO ESP32-C6 Round LCD (240x240)',
    badge: 'Round',
    badgeClass: 'border-violet-800 text-violet-400 bg-violet-900/20',
    headerBadgeClass: 'bg-violet-500/10 text-violet-400 border-violet-500/20',
    color: true,
  },
};

const UNKNOWN_BADGE_CLASS = 'border-edge text-subtle bg-inset';

/** Full board name, falling back to the raw device_type for unknown boards. */
export function boardLabel(deviceType?: string | null): string {
  if (!deviceType) return 'Unknown';
  return BOARDS[deviceType]?.label ?? deviceType;
}

/** Short badge text, falling back to the raw device_type for unknown boards. */
export function boardBadge(deviceType: string): string {
  return BOARDS[deviceType]?.badge ?? deviceType;
}

export function boardBadgeClass(deviceType: string): string {
  return BOARDS[deviceType]?.badgeClass ?? UNKNOWN_BADGE_CLASS;
}

export function boardHeaderBadgeClass(deviceType: string): string {
  return BOARDS[deviceType]?.headerBadgeClass ?? UNKNOWN_BADGE_CLASS;
}

/** Whether this board has a colour panel (so avatar colours are meaningful). */
export function boardHasColor(deviceType?: string | null): boolean {
  if (!deviceType) return false;
  return BOARDS[deviceType]?.color ?? false;
}
