import { useState, useEffect, useMemo, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { login, getAuthStatus } from '../api/client';

const SERVER_BASE = import.meta.env.VITE_API_BASE_URL || '';

/* ── Floating particle background ──────────────────── */
function Particles({ count = 40 }: { count?: number }) {
  const dots = useMemo(
    () =>
      Array.from({ length: count }, (_, i) => {
        const size = Math.random() * 3 + 1;
        const x = Math.random() * 100;
        const delay = Math.random() * 20;
        const duration = Math.random() * 15 + 15;
        const drift = (Math.random() - 0.5) * 30;
        const opacity = Math.random() * 0.4 + 0.1;
        return (
          <span
            key={i}
            className="absolute rounded-full pointer-events-none"
            style={{
              width: size,
              height: size,
              left: `${x}%`,
              bottom: '-5%',
              opacity: 0,
              background:
                i % 3 === 0
                  ? '#ef4444'
                  : i % 3 === 1
                    ? '#6366f1'
                    : '#8b5cf6',
              animation: `particle-rise ${duration}s ${delay}s infinite`,
              '--drift': `${drift}px`,
              '--particle-opacity': opacity,
            } as React.CSSProperties}
          />
        );
      }),
    [count],
  );
  return <div className="fixed inset-0 overflow-hidden">{dots}</div>;
}

/* ── Hookbot logo icon ─────────────────────────────── */
function HookIcon() {
  return (
    <svg
      width="48"
      height="48"
      viewBox="0 0 48 48"
      fill="none"
      className="animate-[icon-pop_0.6s_0.3s_both] drop-shadow-[0_0_24px_rgba(239,68,68,0.4)]"
    >
      <circle cx="24" cy="24" r="22" stroke="url(#logo-grad)" strokeWidth="2.5" fill="none" />
      <path
        d="M18 16c0-3.3 2.7-6 6-6s6 2.7 6 6v12c0 3.3-2.7 6-6 6-1.7 0-3.2-.7-4.2-1.8"
        stroke="url(#logo-grad)"
        strokeWidth="2.5"
        strokeLinecap="round"
        fill="none"
      />
      <circle cx="24" cy="24" r="3" fill="url(#logo-grad)" />
      <defs>
        <linearGradient id="logo-grad" x1="8" y1="8" x2="40" y2="40">
          <stop stopColor="#ef4444" />
          <stop offset="1" stopColor="#8b5cf6" />
        </linearGradient>
      </defs>
    </svg>
  );
}

export default function LoginPage() {
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [workosEnabled, setWorkosEnabled] = useState<boolean | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    getAuthStatus()
      .then((status) => {
        if (status.authenticated) {
          navigate('/', { replace: true });
          return;
        }
        setWorkosEnabled(status.workos_enabled ?? false);
      })
      .catch(() => setWorkosEnabled(false));
  }, [navigate]);

  const handlePasswordSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      await login(password);
      navigate('/', { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setLoading(false);
    }
  };

  const handleWorkosLogin = () => {
    window.location.href = `${SERVER_BASE}/auth/login`;
  };

  /* ── Loading state ────────────────────────────────── */
  if (workosEnabled === null) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-canvas">
        <div className="h-8 w-8 rounded-full border-2 border-brand/30 border-t-brand animate-spin" />
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-canvas relative overflow-hidden font-sans">
      {/* ambient glow blobs */}
      <div className="absolute top-1/3 -left-32 w-96 h-96 rounded-full bg-red-500/[.06] blur-[120px] animate-[drift_12s_ease-in-out_infinite]" />
      <div className="absolute bottom-1/4 -right-32 w-96 h-96 rounded-full bg-indigo-500/[.06] blur-[120px] animate-[drift_14s_2s_ease-in-out_infinite_reverse]" />

      <Particles />

      {/* card */}
      <div
        className="
          relative z-10 w-full max-w-[380px] mx-4
          bg-surface/80 backdrop-blur-xl
          border border-edge/60
          rounded-2xl p-8
          shadow-[0_0_80px_-20px_rgba(239,68,68,0.12),0_0_40px_-10px_rgba(99,102,241,0.10)]
          animate-[card-in_0.7s_ease_both]
        "
      >
        {/* logo + title */}
        <div className="flex flex-col items-center gap-3 mb-6 animate-[fade-up_0.5s_0.15s_both]">
          <HookIcon />
          <h1 className="text-2xl font-extrabold tracking-tight text-fg">
            Hookbot
          </h1>
          <p className="text-sm text-muted">
            {workosEnabled ? 'Sign in to continue' : 'Enter password to continue'}
          </p>
        </div>

        {/* error */}
        {error && (
          <div className="mb-4 px-3 py-2.5 rounded-lg bg-red-500/10 border border-red-500/25 text-red-400 text-sm animate-[shake_0.4s_ease]">
            {error}
          </div>
        )}

        {workosEnabled ? (
          <div className="animate-[fade-up_0.5s_0.35s_both]">
            <button
              onClick={handleWorkosLogin}
              className="
                group relative w-full py-3 rounded-xl
                font-semibold text-white overflow-hidden
                bg-gradient-to-r from-indigo-500 to-purple-500
                shadow-[0_4px_24px_-4px_rgba(99,102,241,0.45)]
                hover:shadow-[0_4px_32px_-2px_rgba(99,102,241,0.6)]
                hover:scale-[1.02] active:scale-[0.98]
                transition-all duration-200 cursor-pointer
              "
            >
              {/* shimmer sweep */}
              <span className="absolute inset-0 bg-gradient-to-r from-transparent via-white/20 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700" />
              <span className="relative">Sign in with WorkOS</span>
            </button>
            <p className="mt-4 text-xs text-center text-subtle">
              Sign up or log in with email, Google, GitHub, and more
            </p>
          </div>
        ) : (
          <form onSubmit={handlePasswordSubmit} className="animate-[fade-up_0.5s_0.35s_both]">
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Password"
              autoFocus
              className="
                w-full px-4 py-3 rounded-xl
                bg-inset border border-edge
                text-fg placeholder:text-subtle
                outline-none
                focus:border-brand/50 focus:ring-2 focus:ring-brand/20
                transition-all duration-200
                mb-4
              "
            />
            <button
              type="submit"
              disabled={loading || !password}
              className="
                group relative w-full py-3 rounded-xl
                font-semibold text-white overflow-hidden
                bg-gradient-to-r from-red-500 to-rose-500
                shadow-[0_4px_24px_-4px_rgba(239,68,68,0.45)]
                hover:shadow-[0_4px_32px_-2px_rgba(239,68,68,0.6)]
                hover:scale-[1.02] active:scale-[0.98]
                transition-all duration-200
                disabled:opacity-40 disabled:pointer-events-none
                disabled:shadow-none
              "
            >
              <span className="absolute inset-0 bg-gradient-to-r from-transparent via-white/20 to-transparent -translate-x-full group-hover:translate-x-full transition-transform duration-700" />
              <span className="relative">{loading ? 'Signing in...' : 'Sign in'}</span>
            </button>
          </form>
        )}
      </div>
    </div>
  );
}
