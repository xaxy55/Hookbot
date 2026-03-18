import { useState } from 'react';
import { runDiagnostics, type DiagResult, type DiagCheck } from '../api/client';

export default function DiagnosticsPage() {
  const [result, setResult] = useState<DiagResult | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState('');

  async function handleRun() {
    setRunning(true);
    setError('');
    try {
      const data = await runDiagnostics();
      setResult(data);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Diagnostics failed');
    }
    setRunning(false);
  }

  const overallColor: Record<string, string> = {
    pass: 'text-green-400 bg-green-500/10 border-green-500/20',
    warn: 'text-yellow-400 bg-yellow-500/10 border-yellow-500/20',
    fail: 'text-red-400 bg-red-500/10 border-red-500/20',
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-fg">Diagnostics</h1>
        <button
          onClick={handleRun}
          disabled={running}
          className="px-4 py-2 text-sm bg-red-600 hover:bg-red-700 text-fg rounded-lg disabled:opacity-50 transition-colors"
        >
          {running ? 'Running...' : 'Run Diagnostics'}
        </button>
      </div>

      <p className="text-sm text-subtle">
        Checks database, device connectivity (TCP port 80), HTTP endpoints, DNS resolution, and firmware storage.
      </p>

      {error && (
        <div className="rounded-lg border border-red-500/20 bg-red-500/10 p-4 text-sm text-red-400">
          {error}
        </div>
      )}

      {!result && !running && (
        <div className="text-center py-16 rounded-lg border border-dashed border-edge">
          <p className="text-dim">Click "Run Diagnostics" to check system health</p>
        </div>
      )}

      {running && !result && (
        <div className="text-center py-16 rounded-lg border border-edge bg-surface">
          <div className="inline-block w-6 h-6 border-2 border-gray-600 border-t-red-400 rounded-full animate-spin mb-3" />
          <p className="text-muted text-sm">Checking endpoints...</p>
        </div>
      )}

      {result && (
        <>
          {/* Overall status */}
          <div className={`rounded-lg border p-4 flex items-center gap-3 ${overallColor[result.overall] || overallColor.fail}`}>
            <StatusIcon status={result.overall} size={20} />
            <span className="font-semibold text-lg">
              {result.overall === 'pass' ? 'All checks passed' :
               result.overall === 'warn' ? 'Passed with warnings' :
               'Some checks failed'}
            </span>
            <span className="ml-auto text-xs opacity-60">
              {result.checks.length} check{result.checks.length !== 1 ? 's' : ''}
            </span>
          </div>

          {/* Check results */}
          <div className="rounded-lg border border-edge bg-surface overflow-hidden divide-y divide-edge/50">
            {result.checks.map((check, i) => (
              <CheckRow key={i} check={check} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

function CheckRow({ check }: { check: DiagCheck }) {
  const bg: Record<string, string> = {
    pass: '',
    warn: 'bg-yellow-500/[0.03]',
    fail: 'bg-red-500/[0.03]',
  };

  return (
    <div className={`flex items-center gap-3 px-4 py-3 ${bg[check.status] || ''}`}>
      <StatusIcon status={check.status} size={16} />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-fg">{check.name}</span>
          {check.latency_ms !== null && (
            <span className={`text-[10px] font-mono px-1.5 py-0.5 rounded ${
              check.latency_ms < 100 ? 'text-subtle bg-inset' :
              check.latency_ms < 500 ? 'text-yellow-500 bg-yellow-500/10' :
              'text-red-500 bg-red-500/10'
            }`}>
              {check.latency_ms}ms
            </span>
          )}
        </div>
        <p className="text-xs text-subtle mt-0.5 truncate">{check.message}</p>
      </div>
    </div>
  );
}

function StatusIcon({ status, size = 16 }: { status: string; size?: number }) {
  if (status === 'pass') {
    return (
      <svg width={size} height={size} viewBox="0 0 16 16" fill="none" className="text-green-400 flex-shrink-0">
        <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
        <path d="M5 8l2 2 4-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  }
  if (status === 'warn') {
    return (
      <svg width={size} height={size} viewBox="0 0 16 16" fill="none" className="text-yellow-400 flex-shrink-0">
        <path d="M8 1.5l6.5 12H1.5L8 1.5z" stroke="currentColor" strokeWidth="1.3" strokeLinejoin="round" />
        <path d="M8 6v3M8 11v.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      </svg>
    );
  }
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" className="text-red-400 flex-shrink-0">
      <circle cx="8" cy="8" r="7" stroke="currentColor" strokeWidth="1.5" />
      <path d="M5.5 5.5l5 5M10.5 5.5l-5 5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}
