import { createContext, useContext, useState, useEffect, useCallback } from 'react';
import { useHotkey, useHeldKeys } from '@tanstack/react-hotkeys';
import { useNavigate } from 'react-router-dom';

interface HotkeyContextValue {
  modHeld: boolean;
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
}

const STORAGE_KEY = 'hookbot_sidebar_collapsed';

export const HotkeyContext = createContext<HotkeyContextValue>({
  modHeld: false,
  sidebarCollapsed: false,
  toggleSidebar: () => {},
});

export function useHotkeyContext() {
  return useContext(HotkeyContext);
}

/** Section index → first nav path */
export const SECTION_SHORTCUTS: Record<number, string> = {
  1: '/',           // Control → Overview
  2: '/ota',        // Firmware → OTA
  3: '/activity',   // Gamification → Activity
  4: '/community',  // Community → Plugin Store
  5: '/device-links', // Multi-Device → Device Links
  6: '/social',     // Social → Social Hub
  7: '/voice',      // AI & Voice → Voice Control
  8: '/desk-lights', // Desk Ecosystem → Desk Lighting
  9: '/account',    // Settings → Account
};

export function useHotkeySetup() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    try {
      return localStorage.getItem(STORAGE_KEY) === 'true';
    } catch {
      return false;
    }
  });

  const toggleSidebar = useCallback(() => {
    setSidebarCollapsed(prev => {
      const next = !prev;
      try { localStorage.setItem(STORAGE_KEY, String(next)); } catch {}
      return next;
    });
  }, []);

  // Track held modifier keys
  const heldKeys = useHeldKeys();
  const modHeld = heldKeys.includes('Meta') || heldKeys.includes('Control');

  // Cmd+B → toggle sidebar
  useHotkey('Mod+B', (e) => {
    e.preventDefault();
    toggleSidebar();
  });

  return { modHeld, sidebarCollapsed, toggleSidebar };
}

/** Hook to register Cmd+1..9 navigation shortcuts */
export function useNavigationHotkeys() {
  const navigate = useNavigate();

  for (const [num, path] of Object.entries(SECTION_SHORTCUTS)) {
    // eslint-disable-next-line react-hooks/rules-of-hooks
    useHotkey(`Mod+${num}`, (e) => {
      e.preventDefault();
      navigate(path);
    });
  }
}
