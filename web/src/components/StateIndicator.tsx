import type { AvatarState } from '../types';

const STATE_COLORS: Record<AvatarState, string> = {
  idle: 'bg-blue-500',
  thinking: 'bg-purple-500',
  waiting: 'bg-yellow-500',
  success: 'bg-green-500',
  taskcheck: 'bg-teal-500',
  error: 'bg-red-500',
};

const STATE_LABELS: Record<AvatarState, string> = {
  idle: 'Idle',
  thinking: 'Thinking',
  waiting: 'Waiting',
  success: 'Success',
  taskcheck: 'Task Check',
  error: 'Error',
};

export default function StateIndicator({ state, size = 'sm' }: { state: string; size?: 'sm' | 'lg' }) {
  const s = (state || 'idle') as AvatarState;
  const dotSize = size === 'lg' ? 'w-3 h-3' : 'w-2 h-2';
  const textSize = size === 'lg' ? 'text-sm' : 'text-xs';

  return (
    <div className="flex items-center gap-1.5">
      <span className={`${dotSize} rounded-full ${STATE_COLORS[s] || 'bg-gray-500'} ${s !== 'idle' ? 'animate-pulse' : ''}`} />
      <span className={`${textSize} text-gray-400`}>{STATE_LABELS[s] || state}</span>
    </div>
  );
}
