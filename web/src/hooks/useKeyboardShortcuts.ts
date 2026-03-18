import { useEffect, useCallback } from 'react';
import { sendState } from '../api/client';
import { useToast } from './useToast';

type AvatarState = 'idle' | 'thinking' | 'waiting' | 'success' | 'taskcheck' | 'error';

const KEY_STATE_MAP: Record<string, AvatarState> = {
  '1': 'idle',
  '2': 'thinking',
  '3': 'waiting',
  '4': 'success',
  '5': 'taskcheck',
  '6': 'error',
};

export function useKeyboardShortcuts(deviceId: string | undefined) {
  const { toast } = useToast();

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      // Ignore when typing in form elements
      const tag = (e.target as HTMLElement).tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      if ((e.target as HTMLElement).isContentEditable) return;

      // Ignore with modifier keys
      if (e.ctrlKey || e.metaKey || e.altKey) return;

      const state = KEY_STATE_MAP[e.key];
      if (!state || !deviceId) return;

      e.preventDefault();
      sendState(deviceId, state)
        .then(() => toast(`State: ${state}`, 'info', 1500))
        .catch(() => toast(`Failed to send state: ${state}`, 'error'));
    },
    [deviceId, toast],
  );

  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}

export const SHORTCUT_LIST = Object.entries(KEY_STATE_MAP).map(([key, state]) => ({
  key,
  state,
}));
