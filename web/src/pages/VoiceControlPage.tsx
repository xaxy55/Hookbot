import { useState, useEffect, useRef } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getDevices,
  getVoiceConfig,
  updateVoiceConfig,
  getVoiceHistory,
  sendVoiceCommand,
  requestTts,
} from '../api/client';
import { useToast } from '../hooks/useToast';

export default function VoiceControlPage() {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const [selectedDevice, setSelectedDevice] = useState<string>('');
  const [commandText, setCommandText] = useState('');
  const [ttsText, setTtsText] = useState('');
  const [isListening, setIsListening] = useState(false);
  const recognitionRef = useRef<SpeechRecognitionInstance | null>(null);

  const { data: devices } = useQuery({
    queryKey: ['devices'],
    queryFn: getDevices,
  });

  const deviceId = selectedDevice || devices?.[0]?.id;

  const { data: config } = useQuery({
    queryKey: ['voiceConfig', deviceId],
    queryFn: () => getVoiceConfig(deviceId),
    enabled: !!deviceId,
  });

  const { data: history, refetch: refetchHistory } = useQuery({
    queryKey: ['voiceHistory', deviceId],
    queryFn: () => getVoiceHistory(deviceId, 20),
    enabled: !!deviceId,
    refetchInterval: 5000,
  });

  const updateConfigMutation = useMutation({
    mutationFn: (data: Parameters<typeof updateVoiceConfig>[0]) => updateVoiceConfig(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['voiceConfig'] });
      toast('Voice config updated', 'success');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const commandMutation = useMutation({
    mutationFn: (text: string) => sendVoiceCommand(text, deviceId),
    onSuccess: (data) => {
      refetchHistory();
      toast(data.response, 'success');
      if (data.state) {
        toast(`State changed to: ${data.state}`, 'info');
      }
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  const ttsMutation = useMutation({
    mutationFn: (text: string) => requestTts(text, deviceId, config?.tts_voice),
    onSuccess: (data) => {
      refetchHistory();
      toast(`Speaking: "${data.text}"`, 'success');
      setTtsText('');
    },
    onError: (e: Error) => toast(e.message, 'error'),
  });

  // Web Speech API for browser-based voice input
  const hasSpeechRecognition = typeof window !== 'undefined' &&
    ('SpeechRecognition' in window || 'webkitSpeechRecognition' in window);

  function startListening() {
    if (!hasSpeechRecognition) {
      toast('Speech recognition not supported in this browser', 'error');
      return;
    }

    const SpeechRecognitionCtor = window.SpeechRecognition || window.webkitSpeechRecognition;
    if (!SpeechRecognitionCtor) return;
    const recognition = new SpeechRecognitionCtor();
    recognition.continuous = false;
    recognition.interimResults = false;
    recognition.lang = config?.language || 'en-US';

    recognition.onresult = (event: SpeechRecognitionEvent) => {
      const transcript = event.results[0][0].transcript;
      setCommandText(transcript);
      commandMutation.mutate(transcript);
      setIsListening(false);
    };

    recognition.onerror = () => {
      setIsListening(false);
      toast('Speech recognition error', 'error');
    };

    recognition.onend = () => setIsListening(false);

    recognitionRef.current = recognition;
    recognition.start();
    setIsListening(true);
  }

  function stopListening() {
    recognitionRef.current?.stop();
    setIsListening(false);
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (commandText.trim()) {
      commandMutation.mutate(commandText.trim());
      setCommandText('');
    }
  }

  // Cleanup
  useEffect(() => {
    return () => { recognitionRef.current?.stop(); };
  }, []);

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold text-fg">Voice Control</h1>
        <p className="text-sm text-muted mt-1">
          Control your Hookbot with voice commands or text input
        </p>
      </div>

      {/* Device selector */}
      {devices && devices.length > 1 && (
        <div className="flex items-center gap-3">
          <label className="text-sm text-muted">Device:</label>
          <select
            value={selectedDevice}
            onChange={e => setSelectedDevice(e.target.value)}
            className="bg-inset border border-edge rounded-lg px-3 py-1.5 text-sm text-fg"
          >
            {devices.map(d => (
              <option key={d.id} value={d.id}>{d.name}</option>
            ))}
          </select>
        </div>
      )}

      {/* Voice input */}
      <div className="bg-surface border border-edge rounded-xl p-6">
        <h2 className="text-lg font-semibold text-fg mb-4">Voice Input</h2>

        {/* Microphone button */}
        <div className="flex flex-col items-center gap-4 mb-6">
          <button
            onClick={isListening ? stopListening : startListening}
            disabled={!hasSpeechRecognition}
            className={`w-24 h-24 rounded-full flex items-center justify-center transition-all ${
              isListening
                ? 'bg-red-500 animate-pulse shadow-lg shadow-red-500/30'
                : 'bg-brand hover:bg-brand/80 shadow-lg shadow-brand/20'
            } ${!hasSpeechRecognition ? 'opacity-50 cursor-not-allowed' : ''}`}
          >
            <MicIcon size={40} />
          </button>
          <span className="text-sm text-muted">
            {isListening ? 'Listening... tap to stop' :
             hasSpeechRecognition ? 'Tap to speak' : 'Speech not supported in this browser'}
          </span>
        </div>

        {/* Text command input */}
        <form onSubmit={handleSubmit} className="flex gap-3">
          <input
            type="text"
            value={commandText}
            onChange={e => setCommandText(e.target.value)}
            placeholder="Type a command... (e.g. 'set to thinking', 'what's the time?')"
            className="flex-1 bg-inset border border-edge rounded-lg px-4 py-2.5 text-sm text-fg placeholder:text-dim"
          />
          <button
            type="submit"
            disabled={!commandText.trim() || commandMutation.isPending}
            className="bg-brand text-white px-5 py-2.5 rounded-lg text-sm font-medium hover:bg-brand/80 disabled:opacity-50 transition-colors"
          >
            {commandMutation.isPending ? 'Sending...' : 'Send'}
          </button>
        </form>

        {/* Quick commands */}
        <div className="mt-4 flex flex-wrap gap-2">
          {['idle', 'thinking', 'waiting', 'success', 'taskcheck', 'error'].map(cmd => (
            <button
              key={cmd}
              onClick={() => commandMutation.mutate(`set state to ${cmd}`)}
              className="px-3 py-1.5 rounded-lg bg-inset border border-edge text-xs text-muted hover:text-fg hover:border-brand/40 transition-colors"
            >
              {cmd}
            </button>
          ))}
          <button
            onClick={() => commandMutation.mutate('status')}
            className="px-3 py-1.5 rounded-lg bg-inset border border-edge text-xs text-muted hover:text-fg hover:border-brand/40 transition-colors"
          >
            status
          </button>
          <button
            onClick={() => commandMutation.mutate('what time is it')}
            className="px-3 py-1.5 rounded-lg bg-inset border border-edge text-xs text-muted hover:text-fg hover:border-brand/40 transition-colors"
          >
            time
          </button>
        </div>
      </div>

      {/* Text-to-Speech */}
      <div className="bg-surface border border-edge rounded-xl p-6">
        <h2 className="text-lg font-semibold text-fg mb-4">Text-to-Speech</h2>
        <p className="text-xs text-muted mb-3">
          Type text below to have Hookbot speak it aloud through the I2S speaker (requires OPENAI_API_KEY on server).
        </p>
        <form onSubmit={e => { e.preventDefault(); if (ttsText.trim()) ttsMutation.mutate(ttsText.trim()); }} className="flex gap-3">
          <input
            type="text"
            value={ttsText}
            onChange={e => setTtsText(e.target.value)}
            placeholder="Type something for Hookbot to say..."
            className="flex-1 bg-inset border border-edge rounded-lg px-4 py-2.5 text-sm text-fg placeholder:text-dim"
          />
          <button
            type="submit"
            disabled={!ttsText.trim() || ttsMutation.isPending}
            className="bg-purple-600 text-white px-5 py-2.5 rounded-lg text-sm font-medium hover:bg-purple-500 disabled:opacity-50 transition-colors"
          >
            {ttsMutation.isPending ? 'Speaking...' : 'Speak'}
          </button>
        </form>
        <div className="mt-3 flex flex-wrap gap-2">
          {['Hello, I am your desk companion.', 'Build successful!', 'Error detected. Fix it now.', 'Time for a break.'].map(phrase => (
            <button
              key={phrase}
              onClick={() => ttsMutation.mutate(phrase)}
              className="px-3 py-1.5 rounded-lg bg-inset border border-edge text-xs text-muted hover:text-fg hover:border-purple-400/40 transition-colors"
            >
              {phrase}
            </button>
          ))}
        </div>
      </div>

      {/* Settings */}
      <div className="bg-surface border border-edge rounded-xl p-6">
        <h2 className="text-lg font-semibold text-fg mb-4">Voice Settings</h2>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-fg">Wake Word Detection</div>
              <div className="text-xs text-muted">Auto-record when speech is detected (on-device)</div>
            </div>
            <button
              onClick={() => updateConfigMutation.mutate({
                device_id: deviceId,
                wake_word_enabled: !config?.wake_word_enabled,
              })}
              className={`w-11 h-6 rounded-full transition-colors relative ${
                config?.wake_word_enabled ? 'bg-brand' : 'bg-inset border border-edge'
              }`}
            >
              <span className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform ${
                config?.wake_word_enabled ? 'translate-x-5' : 'translate-x-0.5'
              }`} />
            </button>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-fg">Text-to-Speech</div>
              <div className="text-xs text-muted">Hookbot speaks responses through I2S speaker</div>
            </div>
            <button
              onClick={() => updateConfigMutation.mutate({
                device_id: deviceId,
                tts_enabled: !config?.tts_enabled,
              })}
              className={`w-11 h-6 rounded-full transition-colors relative ${
                config?.tts_enabled ? 'bg-brand' : 'bg-inset border border-edge'
              }`}
            >
              <span className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform ${
                config?.tts_enabled ? 'translate-x-5' : 'translate-x-0.5'
              }`} />
            </button>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-fg">Speaker Volume</div>
              <div className="text-xs text-muted">{config?.volume ?? 80}%</div>
            </div>
            <input
              type="range"
              min={0}
              max={100}
              value={config?.volume ?? 80}
              onChange={e => updateConfigMutation.mutate({
                device_id: deviceId,
                volume: parseInt(e.target.value),
              })}
              className="w-32 accent-brand"
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-fg">TTS Voice</div>
              <div className="text-xs text-muted">OpenAI TTS voice character</div>
            </div>
            <select
              value={config?.tts_voice ?? 'onyx'}
              onChange={e => updateConfigMutation.mutate({
                device_id: deviceId,
                tts_voice: e.target.value,
              })}
              className="bg-inset border border-edge rounded-lg px-3 py-1.5 text-sm text-fg"
            >
              <option value="onyx">Onyx (deep)</option>
              <option value="alloy">Alloy (neutral)</option>
              <option value="echo">Echo (warm)</option>
              <option value="fable">Fable (expressive)</option>
              <option value="nova">Nova (friendly)</option>
              <option value="shimmer">Shimmer (clear)</option>
              <option value="ash">Ash (conversational)</option>
              <option value="coral">Coral (informative)</option>
              <option value="sage">Sage (calm)</option>
            </select>
          </div>

          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium text-fg">Language</div>
              <div className="text-xs text-muted">Speech recognition language</div>
            </div>
            <select
              value={config?.language ?? 'en'}
              onChange={e => updateConfigMutation.mutate({
                device_id: deviceId,
                language: e.target.value,
              })}
              className="bg-inset border border-edge rounded-lg px-3 py-1.5 text-sm text-fg"
            >
              <option value="en">English</option>
              <option value="es">Spanish</option>
              <option value="fr">French</option>
              <option value="de">German</option>
              <option value="ja">Japanese</option>
              <option value="no">Norwegian</option>
            </select>
          </div>
        </div>
      </div>

      {/* Hardware info */}
      <div className="bg-surface border border-edge rounded-xl p-6">
        <h2 className="text-lg font-semibold text-fg mb-3">Hardware Setup</h2>
        <div className="text-sm text-muted space-y-2">
          <p>Voice control requires I2S audio hardware connected to the ESP32:</p>
          <div className="grid grid-cols-2 gap-4 mt-3">
            <div className="bg-inset rounded-lg p-3">
              <div className="text-xs font-medium text-fg mb-1">INMP441 Microphone</div>
              <div className="text-xs text-dim font-mono space-y-0.5">
                <div>SCK (BCLK) → GPIO 26</div>
                <div>WS (LRCLK) → GPIO 32</div>
                <div>SD (DOUT) → GPIO 33</div>
                <div>VDD → 3.3V</div>
                <div>L/R → GND (left channel)</div>
              </div>
            </div>
            <div className="bg-inset rounded-lg p-3">
              <div className="text-xs font-medium text-fg mb-1">MAX98357A Speaker</div>
              <div className="text-xs text-dim font-mono space-y-0.5">
                <div>BCLK → GPIO 27</div>
                <div>LRC → GPIO 14</div>
                <div>DIN → GPIO 12</div>
                <div>VIN → 5V</div>
                <div>GAIN → unconnected (9dB)</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Command history */}
      <div className="bg-surface border border-edge rounded-xl p-6">
        <h2 className="text-lg font-semibold text-fg mb-4">Command History</h2>
        {history && history.length > 0 ? (
          <div className="space-y-2">
            {history.map(cmd => (
              <div key={cmd.id} className="flex items-start gap-3 bg-inset rounded-lg p-3">
                <div className={`w-2 h-2 rounded-full mt-1.5 flex-shrink-0 ${
                  cmd.status === 'processed' ? 'bg-green-500' :
                  cmd.status === 'transcribed' ? 'bg-blue-500' :
                  cmd.status === 'tts_requested' ? 'bg-purple-500' :
                  'bg-yellow-500'
                }`} />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-sm text-fg truncate">
                      {cmd.transcript || cmd.response || 'Audio received'}
                    </span>
                    <span className="text-[10px] text-dim flex-shrink-0">
                      {new Date(cmd.created_at).toLocaleTimeString()}
                    </span>
                  </div>
                  {cmd.response && cmd.transcript && (
                    <div className="text-xs text-muted mt-0.5">{cmd.response}</div>
                  )}
                  {cmd.duration_secs > 0 && (
                    <span className="text-[10px] text-dim">{cmd.duration_secs.toFixed(1)}s audio</span>
                  )}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-sm text-dim text-center py-8">
            No voice commands yet. Try speaking or typing a command above.
          </div>
        )}
      </div>
    </div>
  );
}

function MicIcon({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="9" y="2" width="6" height="12" rx="3" />
      <path d="M5 10a7 7 0 0014 0" />
      <line x1="12" y1="19" x2="12" y2="22" />
      <line x1="8" y1="22" x2="16" y2="22" />
    </svg>
  );
}

// Web Speech API types for browsers that support it
interface SpeechRecognitionEvent extends Event {
  results: SpeechRecognitionResultList;
}

interface SpeechRecognitionInstance extends EventTarget {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  start(): void;
  stop(): void;
  onresult: ((event: SpeechRecognitionEvent) => void) | null;
  onerror: ((event: Event) => void) | null;
  onend: (() => void) | null;
}

interface SpeechRecognitionConstructor {
  new (): SpeechRecognitionInstance;
}

declare global {
  interface Window {
    SpeechRecognition?: SpeechRecognitionConstructor;
    webkitSpeechRecognition?: SpeechRecognitionConstructor;
  }
}
