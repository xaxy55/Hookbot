import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getDevices, sendState, getGamificationStats, sendPomodoroAction } from '../api/client';

type SessionType = 'focus' | 'shortBreak' | 'longBreak';
type TimerStatus = 'idle' | 'running' | 'paused';

interface SessionConfig {
  focus: number;
  shortBreak: number;
  longBreak: number;
}

const DEFAULT_CONFIG: SessionConfig = {
  focus: 25,
  shortBreak: 5,
  longBreak: 15,
};

const SESSION_LABELS: Record<SessionType, string> = {
  focus: 'Focus',
  shortBreak: 'Short Break',
  longBreak: 'Long Break',
};

const SESSION_COLORS: Record<SessionType, { ring: string; glow: string; bg: string; text: string }> = {
  focus: {
    ring: '#6366f1',
    glow: 'drop-shadow(0 0 20px rgba(99,102,241,0.5))',
    bg: 'bg-indigo-500/10',
    text: 'text-indigo-400',
  },
  shortBreak: {
    ring: '#22c55e',
    glow: 'drop-shadow(0 0 20px rgba(34,197,94,0.4))',
    bg: 'bg-green-500/10',
    text: 'text-green-400',
  },
  longBreak: {
    ring: '#06b6d4',
    glow: 'drop-shadow(0 0 20px rgba(6,182,212,0.4))',
    bg: 'bg-cyan-500/10',
    text: 'text-cyan-400',
  },
};

function playCompletionSound() {
  try {
    const ctx = new AudioContext();
    const notes = [523.25, 659.25, 783.99]; // C5, E5, G5
    notes.forEach((freq, i) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.value = freq;
      gain.gain.setValueAtTime(0, ctx.currentTime + i * 0.15);
      gain.gain.linearRampToValueAtTime(0.15, ctx.currentTime + i * 0.15 + 0.05);
      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + i * 0.15 + 0.4);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(ctx.currentTime + i * 0.15);
      osc.stop(ctx.currentTime + i * 0.15 + 0.5);
    });
  } catch {
    // Audio not available
  }
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

export default function PomodoroPage() {
  const { data: devices } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [selectedDevice, setSelectedDevice] = useState('');
  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: gamStats } = useQuery({
    queryKey: ['gamification-stats', deviceId],
    queryFn: () => getGamificationStats(deviceId),
    enabled: !!deviceId,
    refetchInterval: 30000,
  });

  const [config, setConfig] = useState<SessionConfig>(DEFAULT_CONFIG);
  const [sessionType, setSessionType] = useState<SessionType>('focus');
  const [status, setStatus] = useState<TimerStatus>('idle');
  const [timeLeft, setTimeLeft] = useState(DEFAULT_CONFIG.focus * 60);
  const [focusCount, setFocusCount] = useState(0);
  const [todaySessions, setTodaySessions] = useState(0);
  const [todayMinutes, setTodayMinutes] = useState(0);
  const [showSettings, setShowSettings] = useState(false);

  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const statusRef = useRef<TimerStatus>('idle');
  const sessionTypeRef = useRef<SessionType>('focus');
  const focusCountRef = useRef(0);

  // Keep refs in sync
  statusRef.current = status;
  sessionTypeRef.current = sessionType;
  focusCountRef.current = focusCount;

  const totalDuration = config[sessionType] * 60;
  const progress = totalDuration > 0 ? (totalDuration - timeLeft) / totalDuration : 0;

  const sendDeviceState = useCallback(
    async (state: string) => {
      if (deviceId) {
        try {
          await sendState(deviceId, state);
        } catch {
          // Device might be offline
        }
      }
    },
    [deviceId]
  );

  const syncPomodoro = useCallback(
    async (action: string, extra?: Record<string, unknown>) => {
      if (deviceId) {
        try {
          await sendPomodoroAction(action, extra, deviceId);
        } catch {
          // Device might be offline
        }
      }
    },
    [deviceId]
  );

  const advanceSession = useCallback(() => {
    const currentType = sessionTypeRef.current;
    const currentFocusCount = focusCountRef.current;

    if (currentType === 'focus') {
      // Focus just ended
      const newCount = currentFocusCount + 1;
      setFocusCount(newCount);
      focusCountRef.current = newCount;
      setTodaySessions((s) => s + 1);
      setTodayMinutes((m) => m + config.focus);

      playCompletionSound();
      sendDeviceState('success');

      // After 4 focus sessions, long break
      if (newCount % 4 === 0) {
        setSessionType('longBreak');
        sessionTypeRef.current = 'longBreak';
        setTimeLeft(config.longBreak * 60);
      } else {
        setSessionType('shortBreak');
        sessionTypeRef.current = 'shortBreak';
        setTimeLeft(config.shortBreak * 60);
      }
    } else {
      // Break just ended
      playCompletionSound();
      setSessionType('focus');
      sessionTypeRef.current = 'focus';
      setTimeLeft(config.focus * 60);
    }

    setStatus('idle');
    statusRef.current = 'idle';
  }, [config, sendDeviceState]);

  const startTimer = useCallback(() => {
    if (intervalRef.current) clearInterval(intervalRef.current);

    setStatus('running');
    statusRef.current = 'running';

    if (sessionTypeRef.current === 'focus') {
      sendDeviceState('thinking');
    } else {
      sendDeviceState('idle');
    }
    syncPomodoro('start', {
      session: sessionTypeRef.current,
      time_left: timeLeft,
      total_duration: config[sessionTypeRef.current] * 60,
    });

    intervalRef.current = setInterval(() => {
      setTimeLeft((prev) => {
        if (prev <= 1) {
          if (intervalRef.current) clearInterval(intervalRef.current);
          intervalRef.current = null;
          // Use setTimeout to avoid state update during render
          setTimeout(() => advanceSession(), 0);
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
  }, [sendDeviceState, advanceSession]);

  const pauseTimer = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
    setStatus('paused');
    sendDeviceState('idle');
    syncPomodoro('pause');
  }, [sendDeviceState, syncPomodoro]);

  const resetTimer = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
    setStatus('idle');
    setTimeLeft(config[sessionType] * 60);
    sendDeviceState('idle');
    syncPomodoro('reset');
  }, [config, sessionType, sendDeviceState, syncPomodoro]);

  const toggleTimer = useCallback(() => {
    if (statusRef.current === 'running') {
      pauseTimer();
    } else {
      startTimer();
    }
  }, [startTimer, pauseTimer]);

  const switchSession = useCallback(
    (type: SessionType) => {
      // Don't interrupt a running timer
      if (statusRef.current === 'running') return;

      setSessionType(type);
      sessionTypeRef.current = type;
      setTimeLeft(config[type] * 60);
      setStatus('idle');
      syncPomodoro('pause', {
        session: type,
        time_left: config[type] * 60,
        total_duration: config[type] * 60,
      });
    },
    [config, syncPomodoro]
  );

  // Keyboard shortcut: Space to toggle
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.code === 'Space' && e.target === document.body) {
        e.preventDefault();
        toggleTimer();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [toggleTimer]);

  // Cleanup interval on unmount
  useEffect(() => {
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, []);

  // Update timeLeft when config changes (only if idle)
  useEffect(() => {
    if (status === 'idle') {
      setTimeLeft(config[sessionType] * 60);
    }
  }, [config, sessionType, status]);

  const colors = SESSION_COLORS[sessionType];
  const ringRadius = 120;
  const ringCircumference = 2 * Math.PI * ringRadius;
  const strokeDashoffset = ringCircumference * (1 - progress);

  // Pulse animation class for breaks
  const isPulsing = status === 'running' && sessionType !== 'focus';

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-fg">Pomodoro Focus Timer</h1>
        <button
          onClick={() => setShowSettings(!showSettings)}
          className="text-xs px-3 py-1.5 rounded-md border border-edge bg-surface text-subtle hover:text-fg hover:bg-raised transition-colors"
        >
          {showSettings ? 'Close Settings' : 'Settings'}
        </button>
      </div>

      {/* Settings Panel */}
      {showSettings && (
        <div className="rounded-lg border border-edge bg-surface p-5 space-y-4">
          <h2 className="text-sm font-semibold text-fg">Timer Settings</h2>
          <div className="grid grid-cols-3 gap-4">
            {(['focus', 'shortBreak', 'longBreak'] as const).map((type) => (
              <div key={type}>
                <label className="block text-xs text-subtle mb-1">{SESSION_LABELS[type]} (min)</label>
                <input
                  type="number"
                  min={1}
                  max={120}
                  value={config[type]}
                  onChange={(e) => {
                    const val = parseInt(e.target.value) || 1;
                    setConfig((c) => ({ ...c, [type]: Math.max(1, Math.min(120, val)) }));
                  }}
                  className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg font-mono focus:outline-none focus:ring-1 focus:ring-indigo-500"
                />
              </div>
            ))}
          </div>

          {/* Device Selector */}
          <div>
            <label className="block text-xs text-subtle mb-1">Target Device</label>
            <select
              value={selectedDevice}
              onChange={(e) => setSelectedDevice(e.target.value)}
              className="w-full px-3 py-2 text-sm bg-inset border border-edge rounded-md text-fg focus:outline-none focus:ring-1 focus:ring-indigo-500"
            >
              <option value="">Auto (first device)</option>
              {devices?.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name} {d.online ? '' : '(offline)'}
                </option>
              ))}
            </select>
          </div>
        </div>
      )}

      {/* Session Type Tabs */}
      <div className="flex justify-center gap-2">
        {(['focus', 'shortBreak', 'longBreak'] as const).map((type) => (
          <button
            key={type}
            onClick={() => switchSession(type)}
            disabled={status === 'running'}
            className={`px-4 py-2 text-sm rounded-lg border transition-all ${
              sessionType === type
                ? `${SESSION_COLORS[type].bg} ${SESSION_COLORS[type].text} border-current`
                : 'border-edge bg-surface text-subtle hover:text-fg hover:bg-raised disabled:opacity-50'
            }`}
          >
            {SESSION_LABELS[type]}
          </button>
        ))}
      </div>

      {/* Timer Ring */}
      <div className="flex justify-center">
        <div className="relative">
          <svg
            width="280"
            height="280"
            viewBox="0 0 280 280"
            className="transform -rotate-90 transition-all duration-300"
            style={{
              filter: status === 'running' ? colors.glow : 'none',
            }}
          >
            {/* Background ring */}
            <circle
              cx="140"
              cy="140"
              r={ringRadius}
              fill="none"
              stroke="currentColor"
              strokeWidth="8"
              className="text-edge"
            />
            {/* Progress ring */}
            <circle
              cx="140"
              cy="140"
              r={ringRadius}
              fill="none"
              stroke={colors.ring}
              strokeWidth="8"
              strokeLinecap="round"
              strokeDasharray={ringCircumference}
              strokeDashoffset={strokeDashoffset}
              className="transition-all duration-1000 ease-linear"
              style={{
                opacity: isPulsing ? undefined : 1,
                animation: isPulsing ? 'pomoPulse 2s ease-in-out infinite' : undefined,
              }}
            />
            {/* Glow ring (focus only) */}
            {status === 'running' && sessionType === 'focus' && (
              <circle
                cx="140"
                cy="140"
                r={ringRadius}
                fill="none"
                stroke={colors.ring}
                strokeWidth="16"
                strokeLinecap="round"
                strokeDasharray={ringCircumference}
                strokeDashoffset={strokeDashoffset}
                className="transition-all duration-1000 ease-linear"
                style={{ opacity: 0.15 }}
              />
            )}
          </svg>

          {/* Center content */}
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <span className="text-5xl font-mono font-bold text-fg tracking-wider tabular-nums">
              {formatTime(timeLeft)}
            </span>
            <span className={`text-sm mt-2 font-medium ${colors.text}`}>
              {SESSION_LABELS[sessionType]}
            </span>
            {status === 'paused' && (
              <span className="text-xs text-muted mt-1 animate-pulse">Paused</span>
            )}
          </div>
        </div>
      </div>

      {/* Controls */}
      <div className="flex justify-center gap-3">
        <button
          onClick={toggleTimer}
          className={`px-8 py-3 text-sm font-semibold rounded-lg transition-all ${
            status === 'running'
              ? 'bg-amber-600 hover:bg-amber-700 text-white'
              : 'bg-indigo-600 hover:bg-indigo-700 text-white'
          }`}
        >
          {status === 'running' ? 'Pause' : status === 'paused' ? 'Resume' : 'Start'}
        </button>
        {status !== 'idle' && (
          <button
            onClick={resetTimer}
            className="px-6 py-3 text-sm font-medium rounded-lg border border-edge bg-surface text-subtle hover:text-fg hover:bg-raised transition-all"
          >
            Reset
          </button>
        )}
      </div>

      <p className="text-center text-xs text-dim">
        Press <kbd className="px-1.5 py-0.5 rounded bg-inset border border-edge text-subtle font-mono text-[10px]">Space</kbd> to start/pause
      </p>

      {/* Stats Bar */}
      <div className="grid grid-cols-3 gap-3">
        <div className="rounded-lg border border-edge bg-surface p-4 text-center">
          <div className="text-2xl font-bold text-fg font-mono">{todaySessions}</div>
          <div className="text-xs text-subtle mt-1">Sessions Today</div>
        </div>
        <div className="rounded-lg border border-edge bg-surface p-4 text-center">
          <div className="text-2xl font-bold text-fg font-mono">{todayMinutes}</div>
          <div className="text-xs text-subtle mt-1">Focus Minutes</div>
        </div>
        <div className="rounded-lg border border-edge bg-surface p-4 text-center">
          <div className="text-2xl font-bold text-fg font-mono">{focusCount}</div>
          <div className="text-xs text-subtle mt-1">Total Pomodoros</div>
        </div>
      </div>

      {/* Cycle Progress */}
      <div className="rounded-lg border border-edge bg-surface p-4">
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-semibold text-fg">Cycle Progress</h2>
          <span className="text-xs text-subtle">
            {focusCount % 4} / 4 until long break
          </span>
        </div>
        <div className="flex gap-2">
          {[0, 1, 2, 3].map((i) => {
            const completed = (focusCount % 4) > i;
            const isCurrent = (focusCount % 4) === i && sessionType === 'focus';
            return (
              <div key={i} className="flex-1 flex flex-col items-center gap-1.5">
                <div
                  className={`w-full h-2 rounded-full transition-all duration-500 ${
                    completed
                      ? 'bg-indigo-500'
                      : isCurrent && status === 'running'
                      ? 'bg-indigo-500/50'
                      : 'bg-inset border border-edge'
                  }`}
                  style={
                    isCurrent && status === 'running'
                      ? {
                          background: `linear-gradient(90deg, #6366f1 ${progress * 100}%, transparent ${progress * 100}%)`,
                        }
                      : undefined
                  }
                />
                <span className="text-[10px] text-dim">
                  {completed ? 'Done' : isCurrent ? 'Now' : `#${i + 1}`}
                </span>
              </div>
            );
          })}
        </div>
      </div>

      {/* XP / Gamification */}
      {gamStats && (
        <div className="rounded-lg border border-edge bg-surface p-4">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-sm font-semibold text-fg">Level {gamStats.level}</h2>
              <p className="text-xs text-subtle mt-0.5">{gamStats.title}</p>
            </div>
            <div className="text-right">
              <div className="text-lg font-bold text-indigo-400 font-mono">
                {gamStats.total_xp.toLocaleString()} XP
              </div>
              <div className="text-[10px] text-dim">
                {gamStats.xp_for_current_level} / {gamStats.xp_for_next_level} to next level
              </div>
            </div>
          </div>
          <div className="mt-3 w-full h-1.5 bg-inset rounded-full overflow-hidden">
            <div
              className="h-full bg-indigo-500 rounded-full transition-all duration-700"
              style={{
                width: `${
                  gamStats.xp_for_next_level > 0
                    ? (gamStats.xp_for_current_level / gamStats.xp_for_next_level) * 100
                    : 0
                }%`,
              }}
            />
          </div>
          <div className="flex items-center gap-4 mt-3 text-xs text-subtle">
            <span>Streak: <span className="text-fg font-mono">{gamStats.current_streak}d</span></span>
            <span>Achievements: <span className="text-fg font-mono">{gamStats.achievements_earned}</span></span>
          </div>
        </div>
      )}

      {/* Inline keyframe style for pulse animation */}
      <style>{`
        @keyframes pomoPulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.5; }
        }
      `}</style>
    </div>
  );
}
