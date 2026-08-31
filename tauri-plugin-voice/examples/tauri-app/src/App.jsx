import { useState, useEffect, useRef } from 'react';
import { flushSync } from 'react-dom';
import { listen, emit } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import {
  voiceHealth,
  bootstrapVoice,
  downloadModel,
  listenModelProgress,
  listenHandoff,
  listenHandoffTimeout,
  getRuntimeState,
  captureHotkey,
  initializeRecorderRuntime,
  startRecording,
  startListening,
  respond,
  stopRecording,
  transcribeFile,
  synthesizeSpeech,
  listenToTtsStream,
  playStreamedSpeech,
  setHotkeyBindings,
  setOverlayMode,
  getOverlayMode,
  setupRecordTranscribePipeline,
  CHANNELS,
  listDeclaredStates,
  registerState
} from 'tauri-plugin-voice-api';

function App() {
  const [response, setResponse] = useState('');
  const [hotkey, setHotkey] = useState('');
  const [isRecording, setIsRecording] = useState(false);
  const [sessionId, setSessionId] = useState(null);
  const [lastRecordingPath, setLastRecordingPath] = useState('');
  const [transcription, setTranscription] = useState('');
  const [isRuntimeInitialized, setIsRuntimeInitialized] = useState(false);
  const [overlayMode, setOverlayModeState] = useState({ type: 'consumer', consumerUrl: 'http://localhost:1420#overlay' });
  const [pipelineActive, setPipelineActive] = useState(false);
  
  const [downloadEngine, setDownloadEngine] = useState('stt');
  const [downloadModelId, setDownloadModelId] = useState('tiny.en');
  const [modelProgress, setModelProgress] = useState(null);
  const [lifecycleState, setLifecycleState] = useState('uninitialized');

  // Plugin state machine phase (Idle/Listening/Capturing/Transcribing/HandedOff).
  const [pluginPhase, setPluginPhase] = useState('idle');
  // Append-only log of every STATE event received.
  const [stateLog, setStateLog] = useState([]);
  // Current interactionId from the most recent handoff event; null when idle.
  const [currentInteractionId, setCurrentInteractionId] = useState(null);
  const [isDemoRunning, setIsDemoRunning] = useState(false);
  const demoCancelledRef = useRef(false);
  const demoStopRecordingRef = useRef(false);

  // TTS State
  const [ttsText, setTtsText] = useState('');
  const [ttsModelId, setTtsModelId] = useState('mlx-community/Kokoro-82M-bf16');
  const [ttsVoiceId, setTtsVoiceId] = useState('af_bella');
  const [ttsAudioUrl, setTtsAudioUrl] = useState(null);
  const [ttsAudioPath, setTtsAudioPath] = useState('');
  const audioRef = useRef(null);

  // Streaming TTS state
  const [streamPhase, setStreamPhase] = useState('idle');
  const [streamSynthesisId, setStreamSynthesisId] = useState(null);
  const [streamEvents, setStreamEvents] = useState([]);
  const [streamError, setStreamError] = useState(null);
  const streamHandleRef = useRef(null);
  const streamUnlistenRef = useRef(null);

  const updateResponse = (val) => {
    const timestamp = new Date().toLocaleTimeString();
    const content = typeof val === 'string' ? val : JSON.stringify(val, null, 2);
    setResponse((prev) => `[${timestamp}]\n${content}\n\n` + prev);
    return val;
  };

  useEffect(() => {
    let unlisten;
    const setupListener = async () => {
      try {
        console.log("Setting up listener for", CHANNELS.STATE);
        updateResponse("Listener active for channel: " + CHANNELS.STATE);
        
        unlisten = await listen(CHANNELS.STATE, (event) => {
          console.log("Received STATE event", event);
          updateResponse({
            channel: 'STATE',
            payload: event.payload,
            phase: event.payload?.state?.phase
          });

          const phase = event.payload?.state?.phase;

          // Track current canonical phase for the live badge.
          if (phase) setPluginPhase(phase);

          // Append timestamped entry to the state event log.
          setStateLog((prev) => [
            ...prev,
            { ts: new Date().toISOString(), phase, payload: event.payload },
          ]);

          // Update recording indicator using new canonical names.
          if (phase === 'listening' || phase === 'capturing') {
            setIsRecording(true);
            setTranscription('');
          } else if (phase === 'idle') {
            setIsRecording(false);
            setCurrentInteractionId(null);
          }
        });

        const unlistenLive = await listen(CHANNELS.LIVE, (event) => {
          console.log("Received LIVE event", event);
          updateResponse({
            channel: 'LIVE',
            payload: event.payload
          });
          if (event.payload?.text) {
            setTranscription(event.payload.text);
          }
        });

        const unlistenError = await listen(CHANNELS.ERROR, (event) => {
          console.log("Received ERROR event", event);
          updateResponse({
            channel: 'ERROR',
            payload: event.payload
          });
        });

        const unlistenProgress = await listenModelProgress((event) => {
          console.log("Received PROGRESS event", event);
          // flushSync forces React to render immediately, bypassing batch deferral.
          flushSync(() => setModelProgress(event));
          if (event.state === 'complete' && event.phase === 'preload') {
            setTimeout(() => setModelProgress(null), 2000);
          }
        });

        const unlistenHandoff = await listenHandoff((event) => {
          console.log("Received HANDOFF event", event);
          updateResponse({ channel: 'HANDOFF', payload: event });
          setCurrentInteractionId(event.interactionId);
        });

        const unlistenHandoffTimeout = await listenHandoffTimeout((event) => {
          console.log("Received HANDOFF_TIMEOUT event", event);
          updateResponse({ channel: 'HANDOFF_TIMEOUT', payload: event });
        });

        // Get initial overlay mode
        const modeRes = await getOverlayMode();
        setOverlayModeState(modeRes.overlayMode);

        // Initial health check
        voiceHealth().then(res => {
          console.log("[App] Initial health check result:", res);
          updateResponse({ action: "initial_health", result: res });
          if (res.lifecycleState) setLifecycleState(res.lifecycleState);
        }).catch(err => {
          console.error("Initial health check failed", err);
          updateResponse({ error: "Initial health check failed", detail: err });
        });

        return () => {
          if (unlisten) unlisten();
          if (unlistenLive) unlistenLive();
          if (unlistenError) unlistenError();
          if (unlistenProgress) unlistenProgress();
          if (unlistenHandoff) unlistenHandoff();
          if (unlistenHandoffTimeout) unlistenHandoffTimeout();
        };
      } catch (err) {
        updateResponse({ error: 'Failed to listen to channels', detail: err });
      }
    };
    setupListener();
  }, []);

  // Poll health while initializing or uninitialized to reflect async startup
  useEffect(() => {
    let interval = null;
    if (lifecycleState === 'initializing' || lifecycleState === 'uninitialized') {
      interval = setInterval(async () => {
        try {
          const res = await voiceHealth();
          if (res && res.lifecycleState && res.lifecycleState !== lifecycleState) {
            setLifecycleState(res.lifecycleState);
            // Optional: updateResponse({ action: "health_poll_update", result: res });
          }
        } catch (err) {
          console.error("Health poll failed", err);
        }
      }, 1000);
    }
    return () => {
      if (interval) clearInterval(interval);
    };
  }, [lifecycleState]);

  // Unlisten and cancel any active stream on unmount.
  useEffect(() => {
    return () => {
      if (streamUnlistenRef.current) streamUnlistenRef.current();
      if (streamHandleRef.current) streamHandleRef.current.cancel();
    };
  }, []);

  // Auto-respond with TTS after a real (non-demo) pipeline handoff.
  useEffect(() => {
    if (!currentInteractionId || isDemoRunning) return;

    let cancelled = false;
    let streamUnlisten = null;
    let handle = null;

    const run = async () => {
      const text = ttsText.trim() || 'Your request has been processed.';

      // Signal speaking phase so overlay shows "Agent Speaking".
      await emit(CHANNELS.STATE, {
        contractVersion: '0.1.0',
        eventId: `speaking-${Date.now()}`,
        emittedAtMs: Date.now(),
        state: { phase: 'speaking', micPermission: 'unknown' },
        readiness: { aggregateStatus: 'ready', recordingReady: true, shortcutReady: true, checks: [], issues: [] },
      });

      try {
        handle = await playStreamedSpeech({
          text,
          modelId: ttsModelId || undefined,
          voiceId: ttsVoiceId || undefined,
        });
        streamHandleRef.current = handle;
        await Promise.race([
          handle.playbackDone,
          new Promise((res) => {
            const chk = setInterval(() => { if (cancelled) { clearInterval(chk); res(); } }, 100);
          }),
        ]);
      } catch (err) {
        updateResponse({ warning: 'Pipeline TTS failed', detail: String(err) });
      }

      if (cancelled) return;
      try {
        await respond(currentInteractionId, { continue: true });
      } catch (err) {
        updateResponse({ warning: 'Pipeline respond failed', detail: String(err) });
      }
    };

    run();

    return () => {
      cancelled = true;
      if (handle) handle.cancel().catch(() => {});
    };
  }, [currentInteractionId, isDemoRunning]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleCaptureHotkey = async () => {
    try {
      updateResponse("Capturing hotkey (20s timeout)...");
      const result = await captureHotkey(20000);
      updateResponse(result);
      
      if (result && result.shortcut) {
        setHotkey(result.shortcut);
        updateResponse("Binding hotkey to toggle action...");
        const bindResult = await setHotkeyBindings({ toggle: result.shortcut }, {});
        updateResponse({ action: 'setHotkeyBindings', result: bindResult });
      }
    } catch (err) {
      updateResponse(err);
    }
  };

  const handleSetupPipeline = async () => {
    try {
      updateResponse('Setting up record→transcribe pipeline...');
      const result = await setupRecordTranscribePipeline({ toggleShortcut: hotkey });
      updateResponse(result);
      if (result && result.active) {
        setPipelineActive(true);
      }
    } catch (err) {
      updateResponse(err);
    }
  };

  const handleToggleRecording = async () => {
    try {
      if (isRecording) {
        const result = await stopRecording(sessionId);
        updateResponse(result);
        if (result && result.result && result.result.recordingWavPath) {
          setLastRecordingPath(result.result.recordingWavPath);
        }
        setIsRecording(false);
        setSessionId(null);
      } else {
        if (!isRuntimeInitialized) {
          const initRes = await initializeRecorderRuntime();
          updateResponse(initRes);
          setIsRuntimeInitialized(true);
        }
        setTranscription('');
        const startRes = await startRecording();
        updateResponse(startRes);
        if (startRes && startRes.sessionId) {
          setSessionId(startRes.sessionId);
          setIsRecording(true);
        }
      }
    } catch (err) {
      updateResponse(err);
    }
  };

  const handleDownloadModel = async () => {
    try {
      updateResponse(`Downloading ${downloadEngine.toUpperCase()} model: ${downloadModelId}...`);
      const result = await downloadModel({ engine: downloadEngine, modelId: downloadModelId });
      updateResponse(result);
    } catch (err) {
      updateResponse(err);
    }
  };

  const handleTranscribe = async () => {
    try {
      const result = await transcribeFile({ path: lastRecordingPath });
      updateResponse(result);
      if (result && result.text) {
        setTranscription(result.text);
      }
    } catch (err) {
      updateResponse(err);
    }
  };

  const handleSynthesizeSpeech = async () => {
    try {
      updateResponse(`Synthesizing speech...`);
      setTtsAudioUrl(null);
      setTtsAudioPath('');
      const result = await synthesizeSpeech({ 
        text: ttsText,
        modelId: ttsModelId || undefined,
        voiceId: ttsVoiceId || undefined,
      });
      updateResponse(result);
      
      if (result && result.audioPath) {
        const url = convertFileSrc(result.audioPath);
        setTtsAudioPath(result.audioPath);
        setTtsAudioUrl(url);
      }
    } catch (err) {
      updateResponse(err);
    }
  };

  const handleStreamSpeech = async () => {
    // Tear down any previous listener before starting fresh.
    if (streamUnlistenRef.current) {
      streamUnlistenRef.current();
      streamUnlistenRef.current = null;
    }
    setStreamEvents([]);
    setStreamError(null);
    setStreamPhase('streaming');
    setStreamSynthesisId(null);

    const unlisten = await listenToTtsStream((event) => {
      const timestamp = new Date().toLocaleTimeString();
      setStreamSynthesisId(event.synthesisId);
      setStreamEvents((prev) => [...prev, { timestamp, event }]);
      if (event.type === 'complete') {
        setStreamPhase('complete');
      } else if (event.type === 'error') {
        setStreamPhase('error');
        setStreamError(event.error || 'Unknown error');
      } else if (event.type === 'cancelled') {
        setStreamPhase('idle');
      }
    });
    streamUnlistenRef.current = unlisten;

    try {
      const handle = await playStreamedSpeech({
        text: ttsText,
        modelId: ttsModelId || undefined,
        voiceId: ttsVoiceId || undefined,
      });
      streamHandleRef.current = handle;
    } catch (err) {
      setStreamPhase('error');
      setStreamError(String(err));
      unlisten();
      streamUnlistenRef.current = null;
    }
  };

  const handleCancelStream = async () => {
    if (!streamHandleRef.current) return;
    setStreamPhase('cancelling');
    await streamHandleRef.current.cancel();
    streamHandleRef.current = null;
  };

  useEffect(() => {
    if (ttsAudioUrl && audioRef.current) {
      audioRef.current.load();
      audioRef.current.play().catch((err) => {
        updateResponse({
          error: 'TTS audio autoplay failed',
          detail: err?.message || err,
        });
      });
    }
  }, [ttsAudioUrl]);

  const handleOverlayModeChange = async (newModeStr) => {
    try {
      let newMode;
      if (newModeStr === 'default') {
        newMode = { type: 'default' };
      } else if (newModeStr === 'consumer') {
        newMode = { type: 'consumer', consumerUrl: 'http://localhost:1420#overlay' };
      } else {
        newMode = { type: 'disabled' };
      }
      
      const res = await setOverlayMode(newMode);
      setOverlayModeState(res.overlayMode);
      updateResponse({ action: 'setOverlayMode', mode: res.overlayMode });
    } catch (err) {
      updateResponse(err);
    }
  };

  const handleEngineChange = (e) => {
    const newEngine = e.target.value;
    setDownloadEngine(newEngine);
    if (newEngine === 'tts') {
      setDownloadModelId('mlx-community/Kokoro-82M-bf16');
    } else {
      setDownloadModelId('tiny.en');
      // Cancel active stream when switching away from TTS.
      if (streamHandleRef.current) {
        streamHandleRef.current.cancel();
        streamHandleRef.current = null;
      }
    }
  };

  const clearLogs = () => setResponse('');

  const DEMO_QUESTIONS = [
    "Hello! I'm your voice assistant. What would you like to talk about today?",
    "Interesting! Could you tell me more about that?",
    "Got it. What else is on your mind?",
  ];

  const handleSimulateDemo = async () => {
    demoCancelledRef.current = false;
    demoStopRecordingRef.current = false;
    setIsDemoRunning(true);

    const delay = (ms) => new Promise((resolve) => {
      const id = setTimeout(resolve, ms);
      const check = setInterval(() => {
        if (demoCancelledRef.current) { clearTimeout(id); clearInterval(check); resolve(); }
      }, 100);
      setTimeout(() => clearInterval(check), ms + 200);
    });

    const emitPhase = (phase) =>
      emit(CHANNELS.STATE, {
        contractVersion: '0.1.0',
        eventId: `demo-state-${Date.now()}`,
        emittedAtMs: Date.now(),
        state: { phase, micPermission: 'unknown' },
        readiness: { aggregateStatus: 'ready', recordingReady: true, shortcutReady: true, checks: [], issues: [] },
      });

    // Resolves when Stop Recording is clicked; also unblocks on cancel.
    const waitForStop = () => new Promise((resolve) => {
      const chk = setInterval(() => {
        if (demoStopRecordingRef.current || demoCancelledRef.current) {
          clearInterval(chk);
          demoStopRecordingRef.current = false;
          resolve();
        }
      }, 100);
    });

    // Emits speaking, plays TTS, waits for audio to actually finish.
    const speak = async (text) => {
      try {
        await emitPhase('speaking');
        const handle = await playStreamedSpeech({
          text,
          modelId: ttsModelId || undefined,
          voiceId: ttsVoiceId || undefined,
        });
        streamHandleRef.current = handle;
        await Promise.race([
          handle.playbackDone,
          new Promise((res) => {
            const chk = setInterval(() => {
              if (demoCancelledRef.current) { clearInterval(chk); res(); }
            }, 100);
          }),
        ]);
      } catch (err) {
        updateResponse({ warning: 'Demo TTS skipped', detail: String(err) });
        await delay(1500);
      }
    };

    try {
      let turnIndex = 0;

      // Agent opens with a question, then alternates: listen → user speaks → agent responds.
      while (!demoCancelledRef.current) {
        const agentText = ttsText.trim() || DEMO_QUESTIONS[turnIndex % DEMO_QUESTIONS.length];

        await speak(agentText);
        if (demoCancelledRef.current) break;

        // Wait for user to "speak" (click Stop Recording).
        await emitPhase('listening');
        await waitForStop();
        if (demoCancelledRef.current) break;

        await emitPhase('capturing');
        await delay(500);
        if (demoCancelledRef.current) break;

        await emitPhase('transcribing');
        await delay(1500);
        if (demoCancelledRef.current) break;

        await emitPhase('handed_off');
        await emit(CHANNELS.HANDOFF, {
          contractVersion: '0.1.0',
          eventId: `demo-handoff-${Date.now()}`,
          interactionId: `interaction-demo-${Date.now()}`,
          transcript: 'Simulated user transcript.',
          transcribedAtMs: Date.now(),
        });

        turnIndex++;
      }
    } finally {
      // Sole emitter of idle — ensures the auto-respond useEffect never races here.
      await emitPhase('idle');
      setIsDemoRunning(false);
      if (streamHandleRef.current) {
        streamHandleRef.current.cancel().catch(() => {});
        streamHandleRef.current = null;
      }
    }
  };

  const handleStopDemoRecording = () => {
    // Advances the demo loop from listening → capturing.
    demoStopRecordingRef.current = true;
  };

  const handleCancelDemo = () => {
    // Set cancel flag and cancel active TTS; finally block emits idle and clears isDemoRunning.
    demoCancelledRef.current = true;
    if (streamHandleRef.current) {
      streamHandleRef.current.cancel().catch(() => {});
      streamHandleRef.current = null;
    }
  };

  return (
    <main className="container">
      <div className="header">
        <h1>Voice Plugin Debugger</h1>
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <div className={`plugin-phase-badge ${pluginPhase}`}>
            {pluginPhase.toUpperCase()}
          </div>
          {currentInteractionId && (
            <code style={{ fontSize: '0.7rem', color: '#9b59b6', background: '#f3e9f9', padding: '2px 8px', borderRadius: '8px' }}>
              {currentInteractionId.slice(-8)}
            </code>
          )}
          <div className={`lifecycle-badge ${lifecycleState}`}>
            {lifecycleState.toUpperCase()}
          </div>
        </div>
      </div>

      <div className="section">
        <h2>Section 1 — Hotkey Registration</h2>
        <button onClick={handleCaptureHotkey}>Capture Hotkey</button>
        {hotkey && <p>Registered Hotkey: <code>{hotkey}</code></p>}
        {hotkey && (
          <button onClick={handleSetupPipeline} disabled={pipelineActive}>
            {pipelineActive ? 'Pipeline Active' : 'Setup Record→Transcribe Pipeline'}
          </button>
        )}
        {pipelineActive && (
          <p style={{ fontSize: '0.8rem', color: '#4a9' }}>
            Pipeline is active. Use <code>{hotkey}</code> to toggle record and auto-transcribe.
            After handoff, TTS will auto-play using the text configured in the TTS section below.
          </p>
        )}
        <p style={{ fontSize: '0.8rem', color: '#666' }}>
          Once captured, the hotkey is bound to the toggle recording action.
        </p>
      </div>

      <div className="section">
        <h2>Section 2 — Recorder Button</h2>
        <button onClick={handleToggleRecording}>
          {isRecording ? 'Stop Recording' : 'Start Recording'}
        </button>
        {lastRecordingPath && (
          <p className="path-display">
            Last Recording: <a href="#" onClick={(e) => { e.preventDefault(); updateResponse(lastRecordingPath); }}>{lastRecordingPath}</a>
          </p>
        )}
      </div>

      <div className="section">
        <h2>Section 3 — Transcription (STT)</h2>
        <button onClick={handleTranscribe} disabled={!lastRecordingPath}>
          Transcribe Last Recording
        </button>
        <div style={{ marginTop: '10px' }}>
          <textarea
            readOnly
            rows={5}
            className="transcription-area"
            value={transcription}
            placeholder="Transcription result will appear here..."
          />
        </div>
      </div>

      <div className="section">
        <h2>Section 4 — Overlay Control</h2>
        <div className="overlay-modes">
          <label>
            <input 
              type="radio" 
              name="overlayMode" 
              value="default" 
              checked={overlayMode.type === 'default'} 
              onChange={() => handleOverlayModeChange('default')} 
            /> Default
          </label>
          <label>
            <input 
              type="radio" 
              name="overlayMode" 
              value="consumer" 
              checked={overlayMode.type === 'consumer'} 
              onChange={() => handleOverlayModeChange('consumer')} 
            /> Consumer
          </label>
          <label>
            <input 
              type="radio" 
              name="overlayMode" 
              value="disabled" 
              checked={overlayMode.type === 'disabled'} 
              onChange={() => handleOverlayModeChange('disabled')} 
            /> Disabled
          </label>
        </div>
        <p style={{ fontSize: '0.8rem', color: '#666', marginTop: '10px' }}>
          <b>Consumer:</b> Current app handles the UI. <b>Default:</b> Plugin handles the UI.
        </p>
      </div>

      <div className="section">
        <h2>Section 5 — Model Management</h2>
        <p style={{ fontSize: '0.8rem', color: '#666' }}>
          Bootstrap loads default models on demand. Use Download to switch models.
        </p>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap' }}>
          <select value={downloadEngine} onChange={handleEngineChange}>
            <option value="stt">STT</option>
            <option value="tts">TTS</option>
          </select>
          {downloadEngine === 'stt' ? (
            <select value={downloadModelId} onChange={(e) => setDownloadModelId(e.target.value)}>
              {['tiny', 'tiny.en', 'base', 'base.en', 'small', 'small.en', 'medium', 'medium.en', 'large', 'large-v3', 'large-v3-turbo', 'turbo'].map((m) => (
                <option key={m} value={m}>{m}</option>
              ))}
            </select>
          ) : (
            <input 
              type="text" 
              value={downloadModelId} 
              onChange={(e) => setDownloadModelId(e.target.value)} 
              placeholder="e.g. mlx-community/Kokoro-82M-bf16"
            />
          )}
          <button onClick={handleDownloadModel}>Download Model</button>
        </div>
        {modelProgress && (
          <div className="model-progress">
            {modelProgress.engine && (
              <span className="progress-engine" style={{marginRight: '8px', fontWeight: 'bold'}}>{modelProgress.engine.toUpperCase()}</span>
            )}
            <span className="progress-phase">{modelProgress.phase}</span>
            <span className="progress-state">{modelProgress.state}</span>
            {modelProgress.filename && (
              <span className="progress-filename">{modelProgress.filename}</span>
            )}
            {modelProgress.percent != null && (
              <div className="progress-bar-track">
                <div
                  className="progress-bar-fill"
                  style={{ width: `${Math.round(modelProgress.percent * 100)}%` }}
                />
              </div>
            )}
            {modelProgress.error && (
              <span className="progress-error">{modelProgress.error}</span>
            )}
          </div>
        )}
      </div>

      <div className="section">
        <h2>Section 6 — Text-to-Speech (TTS)</h2>
        <textarea
          rows={3}
          className="transcription-area"
          value={ttsText}
          onChange={(e) => setTtsText(e.target.value)}
          placeholder="Enter text to synthesize..."
        />
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap', marginTop: '10px' }}>
          <input
            type="text"
            placeholder="Model ID (e.g. mlx-community/Kokoro-82M-bf16)"
            value={ttsModelId}
            onChange={(e) => setTtsModelId(e.target.value)}
            style={{ flex: 1, minWidth: '200px' }}
          />
          <input
            type="text"
            placeholder="Voice ID (e.g. af_bella)"
            value={ttsVoiceId}
            onChange={(e) => setTtsVoiceId(e.target.value)}
            style={{ width: '150px' }}
          />
        </div>
        <button onClick={handleSynthesizeSpeech} disabled={!ttsText} style={{ marginTop: '10px' }}>
          Synthesize Speech
        </button>
        {ttsAudioUrl && (
          <div style={{ marginTop: '10px' }}>
            <p className="path-display" style={{ marginBottom: '8px' }}>Generated WAV: {ttsAudioPath}</p>
            <audio
              ref={audioRef}
              controls
              preload="metadata"
              style={{ width: '100%', height: '40px' }}
              onError={() => {
                const error = audioRef.current?.error;
                updateResponse({
                  error: 'TTS audio playback failed',
                  code: error?.code,
                  source: ttsAudioUrl,
                });
              }}
            >
              <source src={ttsAudioUrl} type="audio/wav" />
            </audio>
          </div>
        )}
      </div>

      <div className="section">
        <h2>Section 7 — Streaming TTS (Web Audio)</h2>
        <p style={{ fontSize: '0.8rem', color: '#666' }}>Reuses Model ID / Voice ID / Text from Section 6.</p>

        <div style={{ display: 'flex', gap: '12px', alignItems: 'center', marginBottom: '10px', fontSize: '0.85rem' }}>
          <span>Phase: <code>{streamPhase}</code></span>
          {streamSynthesisId && <span>ID: <code>{streamSynthesisId.slice(0, 8)}…</code></span>}
        </div>

        {streamError && (
          <div style={{ background: '#4a1a1a', color: '#ff8888', padding: '8px', borderRadius: '4px', marginBottom: '10px', fontSize: '0.85rem' }}>
            Error: {streamError}
          </div>
        )}

        <div style={{ display: 'flex', gap: '8px', marginBottom: '12px' }}>
          <button
            onClick={handleStreamSpeech}
            disabled={!ttsText || streamPhase === 'streaming' || streamPhase === 'cancelling'}
          >
            Stream Speech
          </button>
          <button onClick={handleCancelStream} disabled={streamPhase !== 'streaming'}>
            Cancel Stream
          </button>
        </div>

        <div
          style={{
            background: '#111',
            border: '1px solid #333',
            borderRadius: '4px',
            padding: '8px',
            height: '200px',
            overflowY: 'auto',
            fontSize: '0.75rem',
            fontFamily: 'monospace',
          }}
        >
          {streamEvents.length === 0 && <span style={{ color: '#555' }}>No events yet…</span>}
          {streamEvents.map((entry, idx) => {
            const { event, timestamp } = entry;
            const typeColors = { start: '#4af', chunk: '#8d8', complete: '#4f4', error: '#f44', cancelled: '#fa4' };
            const color = typeColors[event.type] || '#ccc';
            return (
              <div key={idx} style={{ borderBottom: '1px solid #1e1e1e', paddingBottom: '3px', marginBottom: '3px' }}>
                <span style={{ color: '#555' }}>[{timestamp}]</span>{' '}
                <span style={{ color, fontWeight: 'bold' }}>{event.type}</span>{' '}
                id={event.synthesisId?.slice(0, 8)} seq={event.sequence}
                {event.sampleRateHz != null && <> rate={event.sampleRateHz}</>}
                {event.channels != null && <> ch={event.channels}</>}
                {event.format && <> fmt={event.format}</>}
                {event.final && <span style={{ color: '#fa4' }}> final</span>}
                {event.error && <span style={{ color: '#f44' }}> err=&quot;{event.error}&quot;</span>}
              </div>
            );
          })}
        </div>
      </div>

      <div className="section">
        <h2>Section 8 — Phase 3 Return Command</h2>
        <div style={{ display: 'flex', gap: '8px', marginBottom: '12px' }}>
          <button onClick={() => respond(currentInteractionId, { continue: false }).then(updateResponse).catch(updateResponse)} disabled={pluginPhase !== 'handed_off'}>
            End Interaction (continue: false)
          </button>
          <button onClick={() => respond(currentInteractionId, { continue: true }).then(updateResponse).catch(updateResponse)} disabled={pluginPhase !== 'handed_off'}>
            Continue (continue: true)
          </button>
        </div>
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center', marginBottom: '12px' }}>
          <button onClick={() => respond(currentInteractionId, { continue: false, speak: { text: ttsText, model: ttsModelId || undefined, voice: ttsVoiceId || undefined } }).then(updateResponse).catch(updateResponse)} disabled={pluginPhase !== 'handed_off' || !ttsText}>
            Speak then End
          </button>
          <button onClick={() => respond(currentInteractionId, { continue: true, speak: { text: ttsText, model: ttsModelId || undefined, voice: ttsVoiceId || undefined } }).then(updateResponse).catch(updateResponse)} disabled={pluginPhase !== 'handed_off' || !ttsText}>
            Speak then Continue
          </button>
          <p style={{ fontSize: '0.8rem', color: '#666' }}>Uses TTS Text from Section 6</p>
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button onClick={() => respond("stale-interaction-id", { continue: false }).then(updateResponse).catch(updateResponse)} disabled={pluginPhase !== 'handed_off'}>
            Test Stale Interaction ID
          </button>
          <button onClick={() => startListening(5000).then(updateResponse).catch(updateResponse)}>
            Start Listening with 5s Timeout
          </button>
        </div>
      </div>

      <div className="section">
        <h2>Section 9 — Phase 4 Overlay Injection Demo</h2>
        <div style={{ display: 'flex', gap: '8px', marginBottom: '12px' }}>
          <button onClick={() => listDeclaredStates().then(updateResponse).catch(updateResponse)}>
            List Declared States
          </button>
          <button onClick={() => registerState({ name: "RuntimeState", timeoutMs: 1000 }).then(updateResponse).catch(updateResponse)}>
            Try Runtime Register
          </button>
        </div>
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <button onClick={() => respond(currentInteractionId, { continue: false, transitionToState: "Thinking" }).then(updateResponse).catch(updateResponse)} disabled={pluginPhase !== 'handed_off'}>
            Show Thinking
          </button>
        </div>
      </div>

      <div className="section">
        <h2>Demo — Real End-to-End Flow</h2>
        <p style={{ fontSize: '0.8rem', color: '#666', marginBottom: '12px' }}>
          Executes a real interaction loop. Use Section 8 above to respond once transcription finishes.
        </p>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button onClick={handleSimulateDemo} disabled={pluginPhase === 'listening' || pluginPhase === 'capturing'}>
            Start Demo
          </button>
          <button onClick={handleStopDemoRecording} disabled={pluginPhase !== 'listening' && pluginPhase !== 'capturing'}>
            Stop Recording
          </button>
          <button onClick={handleCancelDemo} style={{ background: '#c0392b', color: 'white' }}>
            Cancel TTS
          </button>
        </div>
      </div>

      <div className="section">
        <h2>State Event Log</h2>
        <div style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: '6px' }}>
          <button
            style={{ fontSize: '0.75rem', padding: '2px 10px' }}
            onClick={() => setStateLog([])}
          >
            Clear
          </button>
        </div>
        <div className="state-event-log">
          {stateLog.length === 0 && (
            <span style={{ color: '#555', fontSize: '0.8rem' }}>No state events yet…</span>
          )}
          {stateLog.map((entry, idx) => (
            <div key={idx} className={`state-log-entry ${entry.phase || ''}`}>
              <span className="state-log-ts">{entry.ts}</span>
              <span className={`state-log-phase ${entry.phase || ''}`}>
                {entry.phase ?? '—'}
              </span>
              {entry.payload?.diagnostic && (
                <span className="state-log-diagnostic" style={{ marginLeft: '10px', color: '#e74c3c' }}>
                  {entry.payload.diagnostic}
                </span>
              )}
            </div>
          ))}
        </div>
      </div>

      <div className="actions" style={{ marginTop: '20px' }}>
        <button onClick={() => voiceHealth().then(res => {
          if (res.lifecycleState) setLifecycleState(res.lifecycleState);
          return updateResponse(res);
        })}>Check Health</button>
        <button onClick={() => {
          updateResponse('Starting Voice Bootstrap...');
          bootstrapVoice({}).then(updateResponse).catch(updateResponse);
        }}>Bootstrap Voice</button>
        <button onClick={() => getRuntimeState().then(updateResponse)}>Get Runtime State</button>
        <button className="clear" onClick={clearLogs}>Clear Logs</button>
      </div>

      <pre className="debug-console"><code>{response}</code></pre>
    </main>
  );
}

export default App;
