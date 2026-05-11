import { useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { getRuntimeState, CHANNELS } from 'tauri-plugin-voice-api';
import './overlay.css';

const DONE_DISPLAY_MS = 500;

function OverlayApp() {
  const [overlayPhase, setOverlayPhase] = useState('hidden');
  const [interimPreview, setInterimPreview] = useState('');

  // Track previous and active phase for done-state detection.
  const previousPhaseRef = useRef('idle');
  const activePhaseRef = useRef('idle');
  const doneTimerRef = useRef(null);

  useEffect(() => {
    let disposed = false;
    let unlistenState;
    let unlistenLive;
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

      if (phase === 'recording') {
        setOverlayPhase('recording');
        void showOverlay();
        return;
      }

      if (phase === 'transcribing') {
        setOverlayPhase('transcribing');
        void showOverlay();
        return;
      }

      // Transcribing → idle means transcription completed; show "Done" briefly.
      if (phase === 'idle' && previousPhase === 'transcribing') {
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
          if (activePhaseRef.current !== 'recording' && activePhaseRef.current !== 'transcribing') {
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

        if (disposed) {
          stateUn();
          liveUn();
          return;
        }
        unlistenState = stateUn;
        unlistenLive = liveUn;
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
      document.documentElement.classList.remove('overlay-body');
      document.body.classList.remove('overlay-body');
    };
  }, []);

  if (overlayPhase === 'hidden') {
    return <main className="overlay-root" />;
  }

  const overlayLabel =
    overlayPhase === 'recording'
      ? 'Recording'
      : overlayPhase === 'transcribing'
        ? 'Transcribing'
        : 'Done';

  const overlayStateClass =
    overlayPhase === 'recording'
      ? 'is-recording'
      : overlayPhase === 'transcribing'
        ? 'is-transcribing'
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
