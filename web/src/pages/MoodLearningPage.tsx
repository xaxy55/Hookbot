import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getMoodPreferences, getMoodPatterns, getMoodSuggestion, recordMoodFeedback, getDevices } from '../api/client';
import { useToast } from '../hooks/useToast';
import { useState } from 'react';

const STATES = ['idle', 'thinking', 'waiting', 'success', 'taskcheck', 'error'];
const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

export default function MoodLearningPage() {
  const queryClient = useQueryClient();
  const { toast } = useToast();
  const { data: devices = [] } = useQuery({ queryKey: ['devices'], queryFn: getDevices });
  const [deviceId, setDeviceId] = useState<string>('');

  const effectiveDeviceId = deviceId || devices[0]?.id;

  const { data: preferences = [] } = useQuery({
    queryKey: ['mood-preferences', effectiveDeviceId],
    queryFn: () => getMoodPreferences(effectiveDeviceId),
    enabled: !!effectiveDeviceId,
  });

  const { data: patterns = [] } = useQuery({
    queryKey: ['mood-patterns', effectiveDeviceId],
    queryFn: () => getMoodPatterns(effectiveDeviceId),
    enabled: !!effectiveDeviceId,
  });

  const { data: suggestion } = useQuery({
    queryKey: ['mood-suggestion', effectiveDeviceId],
    queryFn: () => getMoodSuggestion(effectiveDeviceId),
    enabled: !!effectiveDeviceId,
    refetchInterval: 30000,
  });

  const feedbackMut = useMutation({
    mutationFn: (data: { state: string; feedback: 'positive' | 'negative' }) =>
      recordMoodFeedback({ device_id: effectiveDeviceId, state: data.state, feedback: data.feedback }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mood-preferences'] });
      queryClient.invalidateQueries({ queryKey: ['mood-patterns'] });
      queryClient.invalidateQueries({ queryKey: ['mood-suggestion'] });
      toast('Feedback recorded', 'success');
    },
  });

  const scoreColor = (score: number) => {
    if (score >= 0.7) return 'text-green-400';
    if (score >= 0.4) return 'text-yellow-400';
    return 'text-red-400';
  };

  // Build heatmap data: 7 days x 24 hours
  const heatmap: Record<string, { state: string | null; confidence: number }> = {};
  for (const p of patterns) {
    heatmap[`${p.day_of_week}-${p.hour_of_day}`] = {
      state: p.preferred_state,
      confidence: p.confidence,
    };
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-fg">Mood Learning</h1>
          <p className="text-sm text-muted mt-1">Track preferences and adapt personality over time</p>
        </div>
        {devices.length > 1 && (
          <select value={deviceId} onChange={e => setDeviceId(e.target.value)}
            className="rounded-lg bg-inset border border-edge px-3 py-2 text-sm text-fg">
            <option value="">All devices</option>
            {devices.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
          </select>
        )}
      </div>

      {/* Current Suggestion */}
      {suggestion && (
        <div className="bg-surface border border-edge rounded-xl p-5 mb-6">
          <h2 className="text-sm font-medium text-subtle uppercase mb-3">Current Suggestion</h2>
          {suggestion.suggested_state ? (
            <div className="flex items-center gap-4">
              <div className="text-3xl font-bold text-brand-fg">{suggestion.suggested_state}</div>
              <div>
                <div className="text-sm text-fg">Confidence: <span className={scoreColor(suggestion.confidence)}>
                  {(suggestion.confidence * 100).toFixed(0)}%
                </span></div>
                <div className="text-xs text-muted mt-0.5">{suggestion.reason}</div>
              </div>
            </div>
          ) : (
            <p className="text-muted text-sm">{suggestion.reason}</p>
          )}
        </div>
      )}

      {/* Quick Feedback */}
      <div className="bg-surface border border-edge rounded-xl p-5 mb-6">
        <h2 className="text-sm font-medium text-subtle uppercase mb-3">Rate States</h2>
        <p className="text-xs text-muted mb-3">Help the hookbot learn which states you prefer right now</p>
        <div className="grid grid-cols-3 sm:grid-cols-6 gap-2">
          {STATES.map(state => (
            <div key={state} className="bg-inset rounded-lg p-3 text-center">
              <div className="text-sm font-medium text-fg mb-2 capitalize">{state}</div>
              <div className="flex justify-center gap-1">
                <button onClick={() => feedbackMut.mutate({ state, feedback: 'positive' })}
                  className="w-8 h-8 rounded-lg bg-green-500/10 text-green-400 hover:bg-green-500/20 text-lg flex items-center justify-center">
                  +
                </button>
                <button onClick={() => feedbackMut.mutate({ state, feedback: 'negative' })}
                  className="w-8 h-8 rounded-lg bg-red-500/10 text-red-400 hover:bg-red-500/20 text-lg flex items-center justify-center">
                  -
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Preferences Table */}
      {preferences.length > 0 && (
        <div className="bg-surface border border-edge rounded-xl p-5 mb-6">
          <h2 className="text-sm font-medium text-subtle uppercase mb-3">Learned Preferences</h2>
          <div className="space-y-2">
            {preferences.map(p => {
              const total = p.positive_responses + p.negative_responses;
              return (
                <div key={p.id} className="flex items-center justify-between py-2 border-b border-edge last:border-0">
                  <div className="flex items-center gap-3">
                    <span className="text-sm font-medium text-fg capitalize w-24">{p.state}</span>
                    {p.animation_id && <span className="text-xs text-dim">anim: {p.animation_id}</span>}
                  </div>
                  <div className="flex items-center gap-4">
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-green-400">+{p.positive_responses}</span>
                      <span className="text-red-400">-{p.negative_responses}</span>
                    </div>
                    <div className="w-24 bg-inset rounded-full h-2 overflow-hidden">
                      <div className={`h-full rounded-full ${p.score >= 0.5 ? 'bg-green-500' : 'bg-red-500'}`}
                        style={{ width: `${p.score * 100}%` }} />
                    </div>
                    <span className={`text-xs font-medium w-10 text-right ${scoreColor(p.score)}`}>
                      {(p.score * 100).toFixed(0)}%
                    </span>
                    <span className="text-xs text-dim w-16 text-right">{total} votes</span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Time-based Pattern Heatmap */}
      {patterns.length > 0 && (
        <div className="bg-surface border border-edge rounded-xl p-5">
          <h2 className="text-sm font-medium text-subtle uppercase mb-3">Time Patterns</h2>
          <p className="text-xs text-muted mb-3">Preferred states by day and hour</p>
          <div className="overflow-x-auto">
            <div className="inline-grid gap-0.5" style={{ gridTemplateColumns: `auto repeat(24, 1fr)` }}>
              {/* Hour headers */}
              <div />
              {Array.from({ length: 24 }, (_, h) => (
                <div key={h} className="text-[10px] text-dim text-center w-5">{h}</div>
              ))}
              {/* Day rows */}
              {DAYS.map((day, di) => (
                <>
                  <div key={`label-${di}`} className="text-[10px] text-muted pr-1 flex items-center">{day}</div>
                  {Array.from({ length: 24 }, (_, h) => {
                    const cell = heatmap[`${di}-${h}`];
                    const opacity = cell ? Math.max(0.15, cell.confidence) : 0.05;
                    return (
                      <div key={`${di}-${h}`}
                        className="w-5 h-5 rounded-sm"
                        style={{ backgroundColor: cell?.state ? `rgba(99, 102, 241, ${opacity})` : `rgba(128, 128, 128, ${opacity})` }}
                        title={cell ? `${cell.state} (${(cell.confidence * 100).toFixed(0)}%)` : 'No data'}
                      />
                    );
                  })}
                </>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
