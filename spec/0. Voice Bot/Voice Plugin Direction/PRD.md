# Voice Plugin Direction PRD

> **Status:** Open questions resolved. See `SCOPE.md` for locked decisions and the canonical contract. This document remains as the product rationale.

## Purpose

This project is zeroing in on `tauri-plugin-voice` as a desktop voice interface layer, not just an STT or TTS utility.

The plugin should eventually own the user-facing voice loop: listening, transcription, response display, and optional spoken output. The host application should own the actual conversation or action logic that happens between user speech and plugin response.

## Product Sentiment

Voice should be treated as the interface.

The voice plugin owns recording, transcription, overlay lifecycle, and rendering. The host application owns what gets displayed in the overlay and what happens after transcription.

## Overlay Ownership Model

The plugin owns overlay **lifecycle and rendering** — when it appears, transitions, and disappears. The host owns overlay **content and customization** — what text is shown, what choices are offered, what intermediate states exist, and how each state looks.

### What the host can control

- **Enable or disable the overlay entirely** — the host may not need the feature at all.
- **Customize the view for any given state** — the host can style or template each state the overlay can be in.
- **Inject intermediate states at configuration time** — after transcription, the host can declare additional states the overlay should pass through before returning to idle. These are declared upfront via a constrained schema, not injected arbitrarily at runtime. The plugin needs a well-defined schema for what a valid intermediate state looks like so it can own transitions reliably.
- **Customize the view for each intermediate state** — each declared intermediate state gets its own host-supplied view.

### What the host cannot control

- When the overlay appears or disappears at runtime.
- Transitions between states — the plugin drives these.
- Anything before transcription is complete.

## Interaction Loop

The clean handoff model:

1. User triggers recording through the voice plugin.
2. Plugin owns everything — recording, transcription, overlay states during this window.
3. Transcription completes. **Plugin hands off control to the host.**
4. Host controls what is displayed in the overlay during its window (intermediate states, responses, prompts).
5. Host hands control back to the plugin with two explicit decisions:
   - Speak the response or not (TTS on/off).
   - Continue the loop (start listening again) or end the interaction.

## Resolved Direction

All open questions previously listed here are resolved in `SCOPE.md`. Headlines:

- Plugin owns the voice I/O loop with a visible state machine; host owns conversation logic and context.
- Handoff is event-out (transcript) → command-in (`speak` + `continue`, extensible by schema, not runtime composition).
- Intermediate overlay states are host-declared at config time.
- Plugin is stateless across turns; multi-turn re-engagement is just another `startListening` call.
- Overlay/recorder ownership stays bridged for now.

Next step is an HLD covering the state machine, handoff payloads, and intermediate-state schema. No implementation.
