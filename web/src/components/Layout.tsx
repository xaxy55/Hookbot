import { Link, Outlet, useLocation } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { getHealth } from '../api/client';
import { useTheme } from '../hooks/useTheme';

const NAV_SECTIONS = [
  {
    label: 'Control',
    items: [
      { path: '/', label: 'Overview', icon: BarChartIcon },
      { path: '/devices', label: 'Devices', icon: CpuIcon },
      { path: '/discovery', label: 'Discovery', icon: RadarIcon },
      { path: '/ble-setup', label: 'BLE WiFi Setup', icon: BluetoothIcon },
      { path: '/avatar', label: 'Avatar Editor', icon: FaceIcon },
      { path: '/animations', label: 'Animations', icon: PlayIcon },
    ],
  },
  {
    label: 'Firmware',
    items: [
      { path: '/ota', label: 'OTA Updates', icon: UploadIcon },
    ],
  },
  {
    label: 'Gamification',
    items: [
      { path: '/activity', label: 'Activity Feed', icon: ActivityIcon },
      { path: '/analytics', label: 'Analytics', icon: ChartLineIcon },
      { path: '/achievements', label: 'Achievements', icon: TrophyNavIcon },
      { path: '/store', label: 'Store', icon: StoreIcon },
    ],
  },
  {
    label: 'Connect',
    items: [
      { path: '/integrations', label: 'Integrations', icon: PlugIcon },
    ],
  },
  {
    label: 'Settings',
    items: [
      { path: '/settings', label: 'Config', icon: GearIcon },
      { path: '/logs', label: 'Logs', icon: ListIcon },
    ],
  },
];

export default function Layout() {
  const location = useLocation();
  const { theme, setTheme } = useTheme();

  const { data: health } = useQuery({
    queryKey: ['health'],
    queryFn: getHealth,
    refetchInterval: 10000,
  });

  const isOk = health?.status === 'ok';

  function isActive(path: string) {
    if (path === '/') return location.pathname === '/';
    return location.pathname.startsWith(path);
  }

  return (
    <div className="flex h-screen bg-canvas text-fg">
      {/* Sidebar */}
      <aside className="w-56 flex-shrink-0 border-r border-edge bg-surface light:shadow-[1px_0_8px_rgba(0,0,0,0.04)] flex flex-col">
        {/* Logo */}
        <div className="h-14 flex items-center gap-2 px-5 border-b border-edge">
          <span className="text-lg font-bold text-brand tracking-wider">DESKBOT</span>
          <span className="text-[10px] text-subtle font-medium tracking-widest">MGMT</span>
        </div>

        {/* Nav sections */}
        <nav className="flex-1 overflow-y-auto py-4 px-3 space-y-6">
          {NAV_SECTIONS.map((section) => (
            <div key={section.label}>
              <div className="flex items-center justify-between px-2 mb-1">
                <span className="text-[11px] font-medium text-subtle uppercase tracking-wider">
                  {section.label}
                </span>
                <span className="text-dim">-</span>
              </div>
              <div className="space-y-0.5">
                {section.items.map((item) => {
                  const active = isActive(item.path);
                  return (
                    <Link
                      key={item.path}
                      to={item.path}
                      className={`flex items-center gap-2.5 px-2.5 py-2 text-sm rounded-lg transition-colors ${
                        active
                          ? 'bg-brand/10 text-brand-fg'
                          : 'text-muted hover:text-fg hover:bg-inset/60'
                      }`}
                    >
                      <item.icon active={active} />
                      {item.label}
                    </Link>
                  );
                })}
              </div>
            </div>
          ))}
        </nav>
      </aside>

      {/* Main content */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Top bar */}
        <header className="h-14 flex items-center justify-end gap-3 px-6 border-b border-edge bg-canvas/80 backdrop-blur-sm">
          {/* Health badge */}
          <Link
            to="/diagnostics"
            className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium transition-colors cursor-pointer ${
              isOk
                ? 'bg-green-500/10 text-green-600 dark:text-green-400 hover:bg-green-500/20'
                : 'bg-red-500/10 text-red-600 dark:text-red-400 hover:bg-red-500/20 animate-pulse'
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${isOk ? 'bg-green-500' : 'bg-red-500'}`} />
            Health {isOk ? 'OK' : 'ERR'}
          </Link>

          {/* Theme toggle */}
          <div className="flex items-center bg-inset rounded-lg p-0.5">
            <button
              onClick={() => setTheme('system')}
              className={`p-1.5 rounded-md transition-colors ${theme === 'system' ? 'bg-raised text-fg' : 'text-subtle hover:text-muted'}`}
              title="System theme"
            >
              <MonitorIcon />
            </button>
            <button
              onClick={() => setTheme('light')}
              className={`p-1.5 rounded-md transition-colors ${theme === 'light' ? 'bg-raised text-fg' : 'text-subtle hover:text-muted'}`}
              title="Light theme"
            >
              <SunIcon />
            </button>
            <button
              onClick={() => setTheme('dark')}
              className={`p-1.5 rounded-md transition-colors ${theme === 'dark' ? 'bg-raised text-fg' : 'text-subtle hover:text-muted'}`}
              title="Dark theme"
            >
              <MoonIcon />
            </button>
          </div>
        </header>

        {/* Page content */}
        <main className="flex-1 overflow-y-auto p-6">
          <div className="mx-auto max-w-6xl">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}

// --- Icons (inline SVG, 16x16) ---

function BarChartIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <rect x="1" y="8" width="3" height="6" rx="0.5" fill="currentColor" opacity="0.6" />
      <rect x="6" y="4" width="3" height="10" rx="0.5" fill="currentColor" opacity="0.8" />
      <rect x="11" y="2" width="3" height="12" rx="0.5" fill="currentColor" />
    </svg>
  );
}

function CpuIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <rect x="4" y="4" width="8" height="8" rx="1" />
      <path d="M6 1v3M10 1v3M6 12v3M10 12v3M1 6h3M1 10h3M12 6h3M12 10h3" />
    </svg>
  );
}

function RadarIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <circle cx="8" cy="8" r="6" />
      <circle cx="8" cy="8" r="3" />
      <circle cx="8" cy="8" r="1" fill="currentColor" />
    </svg>
  );
}

function FaceIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <circle cx="8" cy="8" r="6.5" />
      <circle cx="5.5" cy="6.5" r="1" fill="currentColor" stroke="none" />
      <circle cx="10.5" cy="6.5" r="1" fill="currentColor" stroke="none" />
      <path d="M5.5 10.5c.8 1 1.5 1.2 2.5 1.2s1.7-.2 2.5-1.2" strokeLinecap="round" />
    </svg>
  );
}

function PlayIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <circle cx="8" cy="8" r="6.5" stroke="currentColor" strokeWidth="1.3" />
      <path d="M6.5 5l4.5 3-4.5 3V5z" fill="currentColor" />
    </svg>
  );
}

function UploadIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <path d="M8 10V3M8 3l-3 3M8 3l3 3M2 12v1a1 1 0 001 1h10a1 1 0 001-1v-1" />
    </svg>
  );
}

function GearIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <circle cx="8" cy="8" r="2.5" />
      <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.1 3.1l1.4 1.4M11.5 11.5l1.4 1.4M3.1 12.9l1.4-1.4M11.5 4.5l1.4-1.4" />
    </svg>
  );
}

function ListIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <path d="M5 4h8M5 8h8M5 12h6M2 4h.5M2 8h.5M2 12h.5" strokeLinecap="round" />
    </svg>
  );
}

function PlugIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <path d="M6 2v4M10 2v4M4 6h8v2a4 4 0 01-4 4 4 4 0 01-4-4V6zM8 12v2" />
    </svg>
  );
}

function MonitorIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
      <rect x="2" y="2" width="12" height="9" rx="1" />
      <path d="M5 14h6M8 11v3" />
    </svg>
  );
}

function SunIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
      <circle cx="8" cy="8" r="3" />
      <path d="M8 2v1.5M8 12.5V14M2 8h1.5M12.5 8H14M3.8 3.8l1 1M11.2 11.2l1 1M3.8 12.2l1-1M11.2 4.8l1-1" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
      <path d="M13 9.5A5.5 5.5 0 016.5 3 5.5 5.5 0 1013 9.5z" />
    </svg>
  );
}

function ActivityIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <path d="M1 8h3l2-5 2 10 2-5h5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function ChartLineIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <path d="M2 14V2M2 14h12M4 10l3-4 3 2 4-5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function TrophyNavIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <path d="M4 2h8v5a4 4 0 01-8 0V2zM4 4H2a1 1 0 00-1 1v1a2 2 0 002 2h1M12 4h2a1 1 0 011 1v1a2 2 0 01-2 2h-1M6 12h4M8 10v2M5 14h6" />
    </svg>
  );
}

function StoreIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <path d="M2 3h12l-1 8H3L2 3zM2 3L1.5 1" strokeLinecap="round" strokeLinejoin="round" />
      <circle cx="5" cy="13.5" r="1" fill="currentColor" stroke="none" />
      <circle cx="11" cy="13.5" r="1" fill="currentColor" stroke="none" />
    </svg>
  );
}

function BluetoothIcon({ active }: { active: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" className={active ? 'text-brand-fg' : 'text-subtle'}>
      <polyline points="5 4.5 11 10.5 8 14 8 2 11 5.5 5 11.5" />
    </svg>
  );
}
