import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  sttHealth,
  bootstrapStt,
  getRuntimeState,
  captureHotkey,
  initializeRecorderRuntime,
  startRecording,
  stopRecording,
  transcribeFile,
  setHotkeyBindings,
  setOverlayMode,
  getOverlayMode,
  subscribeOverlayState,
  setupRecordTranscribePipeline,
  CHANNELS
} from 'tauri-plugin-tauri-plugin-stt-api';

// Overlay Component
function VoiceOverlay({ mode, transcription }) {
  const [state, setState] = useState(null);
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    if (mode === 'disabled') {
      setIsVisible(false);
      return;
    }

    let unsubscribe;
    const setup = async () => {
      unsubscribe = await subscribeOverlayState((newState) => {
        setState(newState);
        // Show overlay when recording or transcribing
        setIsVisible(newState.phase === 'recording' || newState.phase === 'transcribing');
      }, { overlayMode: mode });
    };

    setup();
    return () => {
      if (unsubscribe) unsubscribe();
    };
  }, [mode]);

  if (!isVisible || !state) return null;

  return (
    <div className={`voice-overlay-ui ${state.phase}`}>
      <div className="overlay-content">
        <div className="status-indicator">
          <span className="dot"></span>
          <span className="phase-text">{state.phase.toUpperCase()}</span>
        </div>
        {state.phase === 'recording' && <div className="pulse-ring"></div>}
        {state.phase === 'transcribing' && transcription && (
          <div className="overlay-transcription">
            {transcription}
          </div>
        )}
      </div>
    </div>
  );
}

function App() {
  const [response, setResponse] = useState('');
  const [hotkey, setHotkey] = useState('');
  const [isRecording, setIsRecording] = useState(false);
  const [sessionId, setSessionId] = useState(null);
  const [lastRecordingPath, setLastRecordingPath] = useState('');
  const [transcription, setTranscription] = useState('');
  const [isRuntimeInitialized, setIsRuntimeInitialized] = useState(false);
  const [overlayMode, setOverlayModeState] = useState('consumer');
  const [pipelineActive, setPipelineActive] = useState(false);

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

        // Get initial overlay mode
        const modeRes = await getOverlayMode();
        setOverlayModeState(modeRes.overlayMode);

        return () => {
          if (unlisten) unlisten();
          if (unlistenLive) unlistenLive();
          if (unlistenError) unlistenError();
        };
      } catch (err) {
        updateResponse({ error: 'Failed to listen to channels', detail: err });
      }
    };
    setupListener();
  }, []);

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

  const handleOverlayModeChange = async (newMode) => {
    try {
      const res = await setOverlayMode(newMode);
      setOverlayModeState(res.overlayMode);
      updateResponse({ action: 'setOverlayMode', mode: res.overlayMode });
    } catch (err) {
      updateResponse(err);
    }
  };

  const clearLogs = () => setResponse('');

  return (
    <main className="container">
      <VoiceOverlay mode={overlayMode} transcription={transcription} />
      
      <h1>STT Plugin Debugger</h1>

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
        <h2>Section 3 — Transcription</h2>
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
              checked={overlayMode === 'default'} 
              onChange={() => handleOverlayModeChange('default')} 
            /> Default
          </label>
          <label>
            <input 
              type="radio" 
              name="overlayMode" 
              value="consumer" 
              checked={overlayMode === 'consumer'} 
              onChange={() => handleOverlayModeChange('consumer')} 
            /> Consumer
          </label>
          <label>
            <input 
              type="radio" 
              name="overlayMode" 
              value="disabled" 
              checked={overlayMode === 'disabled'} 
              onChange={() => handleOverlayModeChange('disabled')} 
            /> Disabled
          </label>
        </div>
        <p style={{ fontSize: '0.8rem', color: '#666', marginTop: '10px' }}>
          <b>Consumer:</b> Current app handles the UI. <b>Default:</b> Plugin handles the UI.
        </p>
      </div>

      <div className="actions" style={{ marginTop: '20px' }}>
        <button onClick={() => sttHealth().then(updateResponse)}>Check Health</button>
        <button onClick={() => {
          updateResponse("Starting STT Bootstrap...");
          bootstrapStt().then(updateResponse).catch(updateResponse);
        }}>Bootstrap STT</button>
        <button onClick={() => getRuntimeState().then(updateResponse)}>Get Runtime State</button>
        <button className="clear" onClick={clearLogs}>Clear Logs</button>
      </div>

      <pre className="debug-console"><code>{response}</code></pre>
    </main>
  );
}

export default App;
