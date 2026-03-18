import { useState, useCallback, useEffect, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { ToastContext, type Toast, type ToastType } from '../hooks/useToast';

// ── Provider ───────────────────────────────────────

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const dismiss = useCallback((id: string) => {
    const t = timers.current.get(id);
    if (t) clearTimeout(t);
    timers.current.delete(id);
    setToasts(prev => prev.filter(t => t.id !== id));
  }, []);

  const toast = useCallback((message: string, type: ToastType = 'info', duration = 4000) => {
    const id = crypto.randomUUID();
    setToasts(prev => [...prev.slice(-4), { id, message, type, duration }]);
    const timer = setTimeout(() => dismiss(id), duration);
    timers.current.set(id, timer);
  }, [dismiss]);

  useEffect(() => {
    return () => timers.current.forEach(t => clearTimeout(t));
  }, []);

  return (
    <ToastContext.Provider value={{ toasts, toast, dismiss }}>
      {children}
      {createPortal(<ToastContainer toasts={toasts} dismiss={dismiss} />, document.body)}
    </ToastContext.Provider>
  );
}

// ── Container ──────────────────────────────────────

function ToastContainer({ toasts, dismiss }: { toasts: Toast[]; dismiss: (id: string) => void }) {
  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-[9999] flex flex-col gap-2 max-w-sm">
      {toasts.map(t => (
        <ToastItem key={t.id} toast={t} onDismiss={() => dismiss(t.id)} />
      ))}
    </div>
  );
}

// ── Item ───────────────────────────────────────────

const STYLES: Record<ToastType, { border: string; icon: string; iconColor: string }> = {
  success: { border: 'border-green-500/40', icon: '✓', iconColor: 'text-green-400' },
  error:   { border: 'border-red-500/40',   icon: '✕', iconColor: 'text-red-400' },
  info:    { border: 'border-blue-500/40',   icon: 'ℹ', iconColor: 'text-blue-400' },
};

function ToastItem({ toast, onDismiss }: { toast: Toast; onDismiss: () => void }) {
  const [visible, setVisible] = useState(false);
  const s = STYLES[toast.type];

  useEffect(() => {
    requestAnimationFrame(() => setVisible(true));
  }, []);

  return (
    <div
      className={`
        flex items-start gap-3 px-4 py-3 rounded-lg border
        bg-surface ${s.border} shadow-lg shadow-black/20
        transition-all duration-300 ease-out cursor-pointer
        ${visible ? 'opacity-100 translate-x-0' : 'opacity-0 translate-x-8'}
      `}
      onClick={onDismiss}
    >
      <span className={`${s.iconColor} text-sm font-bold mt-0.5 shrink-0`}>{s.icon}</span>
      <span className="text-sm text-fg-2 leading-snug">{toast.message}</span>
    </div>
  );
}
