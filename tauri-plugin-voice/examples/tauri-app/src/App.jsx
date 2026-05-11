import { useState, useEffect, useRef } from 'react';
import { flushSync } from 'react-dom';
import { listen } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import {
  voiceHealth,
  bootstrapVoice,
  downloadModel,
  listenModelProgress,
  getRuntimeState,
  captureHotkey,
  initializeRecorderRuntime,
  startRecording,
  stopRecording,
  transcribeFile,
  synthesizeSpeech,
  setHotkeyBindings,
  setOverlayMode,
  getOverlayMode,
  setupRecordTranscribePipeline,
  CHANNELS
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

  // TTS State
  const [ttsText, setTtsText] = useState('');
  const [ttsModelId, setTtsModelId] = useState('mlx-community/Kokoro-82M-bf16');
  const [ttsVoiceId, setTtsVoiceId] = useState('af_bella');
  const [ttsAudioUrl, setTtsAudioUrl] = useState(null);
  const [ttsAudioPath, setTtsAudioPath] = useState('');
  const audioRef = useRef(null);

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
          
          // Update local recording state if phase changes
          if (event.payload?.state?.phase === 'recording') {
            setIsRecording(true);
            setTranscription('');
          } else if (event.payload?.state?.phase === 'idle' || event.payload?.state?.phase === 'error') {
            setIsRecording(false);
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
    }
  };

  const clearLogs = () => setResponse('');

  return (
    <main className="container">
      <div className="header">
        <h1>Voice Plugin Debugger</h1>
        <div className={`lifecycle-badge ${lifecycleState}`}>
          {lifecycleState.toUpperCase()}
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
