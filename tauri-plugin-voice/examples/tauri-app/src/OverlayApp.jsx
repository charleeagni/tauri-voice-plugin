import { useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { getRuntimeState, CHANNELS } from 'tauri-plugin-voice-api';
import './overlay.css';

const DONE_DISPLAY_MS = 500;

function Thinking() {
  return (
    <section className="overlay-indicator is-thinking" aria-live="polite">
      <span className="overlay-dot is-thinking" style={{ backgroundColor: 'purple', width: '12px', height: '12px', borderRadius: '50%', display: 'inline-block' }} />
      <span className="overlay-text-stack">
        <span className="overlay-text">Thinking...</span>
      </span>
    </section>
  );
}

const overlayRegistry = {
  Thinking: Thinking,
};

function OverlayApp() {
  const [overlayPhase, setOverlayPhase] = useState('hidden');
  const [interimPreview, setInterimPreview] = useState('');
  const [declaredStateName, setDeclaredStateName] = useState(null);

  // Track previous and active phase for done-state detection.
  const previousPhaseRef = useRef('idle');
  const activePhaseRef = useRef('idle');
  const doneTimerRef = useRef(null);

  useEffect(() => {
    let disposed = false;
    let unlistenState;
    let unlistenLive;
    let unlistenRender;
    const currentWindow = getCurrentWindow();

    const clearDoneTimer = () => {
      if (doneTimerRef.current) {
        clearTimeout(doneTimerRef.current);
        doneTimerRef.current = null;
      }
    };

    const hideOverlay = async () => {
      try {
        await currentWindow.hide();
      } catch {
        // Ignore overlay hide failures.
      }
    };

    const showOverlay = async () => {
      try {
        await currentWindow.show();
      } catch {
        // Ignore overlay show failures.
      }
    };

    // Central phase transition handler.
    const applyPhase = (phase) => {
      const previousPhase = previousPhaseRef.current;
      previousPhaseRef.current = phase;
      activePhaseRef.current = phase;

      clearDoneTimer();

      if (phase === 'listening' || phase === 'recording' || phase === 'capturing') {
        setOverlayPhase('recording');
        void showOverlay();
        return;
      }

      if (phase === 'transcribing') {
        setOverlayPhase('transcribing');
        void showOverlay();
        return;
      }

      if (phase === 'handed_off') {
        setOverlayPhase('handed_off');
        void showOverlay();
        return;
      }

      if (phase === 'speaking') {
        setOverlayPhase('speaking');
        void showOverlay();
        return;
      }

      // Check if it's a declared state in our registry
      if (overlayRegistry[phase]) {
        setOverlayPhase('declared_state');
        setDeclaredStateName(phase);
        void showOverlay();
        return;
      }

      // Transcribing or Speaking → idle: show "Done" briefly.
      if (phase === 'idle' && (previousPhase === 'transcribing' || previousPhase === 'speaking')) {
        setOverlayPhase('done');
        void showOverlay();
        doneTimerRef.current = setTimeout(() => {
          if (disposed) return;
          setInterimPreview('');
          setOverlayPhase('hidden');
          void hideOverlay();
        }, DONE_DISPLAY_MS);
        return;
      }

      // Any other idle or error transition hides immediately.
      if (phase === 'idle' || phase === 'error') {
        setInterimPreview('');
      }
      setOverlayPhase('hidden');
      void hideOverlay();
    };

    // Mark document for transparent overlay styling.
    document.documentElement.classList.add('overlay-body');
    document.body.classList.add('overlay-body');

    const init = async () => {
      // Seed phase from current runtime state.
      try {
        const snapshot = await getRuntimeState();
        if (!disposed) {
          applyPhase(snapshot.state.phase);
        }
      } catch {
        // Ignore init state fetch failures.
      }

      try {
        // Listen for phase changes.
        const stateUn = await listen(CHANNELS.STATE, (event) => {
          applyPhase(event.payload?.state?.phase);
        });

        // Listen for live transcript frames.
        const liveUn = await listen(CHANNELS.LIVE, (event) => {
          if (activePhaseRef.current !== 'recording' && activePhaseRef.current !== 'capturing' && activePhaseRef.current !== 'transcribing') {
            // Show final transcript text in done state.
            if (event.payload?.text) {
              setInterimPreview(event.payload.text);
            }
            return;
          }
          if (event.payload?.text) {
            setInterimPreview(event.payload.text);
          }
        });

        const renderUn = await listen(CHANNELS.RENDER_DECLARED_STATE, (event) => {
          if (disposed) return;
          console.log("Render declared state:", event.payload);
          const { stateName } = event.payload;
          if (overlayRegistry[stateName]) {
             setOverlayPhase('declared_state');
             setDeclaredStateName(stateName);
             void showOverlay();
          }
        });

        if (disposed) {
          stateUn();
          liveUn();
          renderUn();
          return;
        }
        unlistenState = stateUn;
        unlistenLive = liveUn;
        unlistenRender = renderUn;
      } catch {
        // Ignore listener setup failures.
      }
    };

    void init();

    return () => {
      disposed = true;
      clearDoneTimer();
      unlistenState?.();
      unlistenLive?.();
      unlistenRender?.();
      document.documentElement.classList.remove('overlay-body');
      document.body.classList.remove('overlay-body');
    };
  }, []);

  if (overlayPhase === 'hidden') {
    return <main className="overlay-root" />;
  }

  if (overlayPhase === 'declared_state' && declaredStateName && overlayRegistry[declaredStateName]) {
    const Component = overlayRegistry[declaredStateName];
    return (
      <main className="overlay-root">
        <Component />
      </main>
    );
  }

  const overlayLabel =
    overlayPhase === 'recording'
      ? 'Recording'
      : overlayPhase === 'transcribing'
        ? 'Transcribing'
        : overlayPhase === 'handed_off'
          ? 'Awaiting host…'
          : overlayPhase === 'speaking'
            ? 'Agent Speaking'
            : overlayPhase === 'cancelled'
              ? 'Cancelled'
              : overlayPhase === 'failed'
                ? 'Failed'
                : 'Done';

  const overlayStateClass =
    overlayPhase === 'recording'
      ? 'is-recording'
      : overlayPhase === 'transcribing'
        ? 'is-transcribing'
        : overlayPhase === 'handed_off'
          ? 'is-handed-off'
          : overlayPhase === 'speaking'
            ? 'is-speaking'
            : overlayPhase === 'cancelled'
              ? 'is-cancelled'
              : overlayPhase === 'failed'
                ? 'is-failed'
                : 'is-done';

  const showPreviewLine =
    (overlayPhase === 'recording' || overlayPhase === 'transcribing' || overlayPhase === 'done') &&
    interimPreview.trim().length > 0;

  return (
    <main className="overlay-root">
      <section className={`overlay-indicator ${overlayStateClass}`} aria-live="polite">
        <span className={`overlay-dot ${overlayStateClass}`} />
        <span className="overlay-text-stack">
          <span className="overlay-text">{overlayLabel}</span>
          {showPreviewLine && <span className="overlay-preview">{interimPreview}</span>}
        </span>
      </section>
    </main>
  );
}

export default OverlayApp;
