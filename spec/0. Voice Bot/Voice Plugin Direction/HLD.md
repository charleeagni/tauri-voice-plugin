# High Level Design: Voice Plugin Interface Boundary

## 1. Objective

Define the architectural boundary between `tauri-plugin-voice` and host applications so the plugin can be reused as a desktop voice interface layer. The design must specify the plugin state machine, the post-transcription handoff contract, the host's return-command shape, the schema for host-declared overlay states, and the re-engagement model.

This HLD covers contracts and boundaries only. It does not begin implementation. The TTS streaming MVP, rename, and recorder bridge internals are governed by their own design tracks and remain out of scope here.

## 2. Current State

The plugin today exposes STT and TTS as discrete file-and-stream contracts. Recording and overlay behavior is reached through the recorder bridge (`recorder-bridge` feature, default-on), which proxies sibling `tauri-plugin-recorder` commands. The `setup_record_transcribe_pipeline` command wires a hotkey to a record-then-transcribe flow and renders a short transcript overlay on completion.

There is no plugin-owned state machine for a voice interaction loop, no formal handoff event to a host application, and no schema for host-declared overlay states. Host applications today consume STT/TTS as primitives and must orchestrate the loop themselves.

## 3. Decision Summary

- The plugin owns the voice I/O loop: recording, transcription, overlay lifecycle, optional TTS.
- The host owns the brain: action routing, conversation context, response composition.
- Plugin states are externally **visible** (no opaque sub-states).
- Handoff is **event-out, command-in**: plugin emits a transcript event; host returns a tagged command.
- The return command is **schema-extensible**, not programmatically composable at runtime.
- Host-declared intermediate overlay states are declared at **config time**, not runtime.
- The plugin is **stateless across turns**; conversation context lives in the host backend.
- Overlay/recorder ownership **stays in the recorder bridge** for now.

## 4. State Machine

The plugin runs a single canonical state machine for one voice interaction:

```
Idle → Listening → Capturing → Transcribing → HandedOff → Idle
```

State definitions:

- **Idle** — no active interaction. Plugin is bootstrapped and ready.
- **Listening** — recording armed; user may begin speaking or cancel.
- **Capturing** — audio is actively being captured.
- **Transcribing** — capture complete, transcription in progress.
- **HandedOff** — transcript delivered to host; plugin is awaiting a return command.
- (Return to **Idle** after the host's return command resolves.)

Transitions are driven by the plugin. Each transition emits a state-change event so hosts and overlays can react. The host **cannot** force a transition between plugin-owned states; it can only initiate the loop and return a command after handoff.

Cancellation and error states extend this baseline:

- **Cancelled** — user or host aborted before transcription completed. Returns to Idle.
- **Failed** — recording, transcription, or playback errored. Returns to Idle with diagnostic detail.

These are terminal-to-Idle states, not branches in the happy path.

## 5. Handoff Event (Plugin → Host)

When the plugin reaches `Transcribing` and produces a transcript, it emits a handoff event and enters `HandedOff`.

Event payload fields (logical, not wire format):

- `interactionId` — opaque identifier correlating the handoff with the host's eventual return command.
- `transcript` — final transcript text.
- `transcribedAtMs` — emission timestamp.
- `engine` — STT engine identifier for traceability.
- `audioRef` — optional reference to the captured audio artifact for hosts that want it.

The plugin holds in `HandedOff` until the host returns a command for this `interactionId` or a configured handoff timeout elapses. On timeout, the plugin returns to `Idle` and emits a state-change event.

## 6. Return Command (Host → Plugin)

The host returns a single tagged command keyed on `interactionId`. The current contract carries two directives:

- `speak` — optional. If present, instructs the plugin to synthesize and play a host-supplied text response via TTS. Includes the text and optional voice/model overrides.
- `continue` — required. Instructs the plugin to either re-arm listening (`continue = true`) or return to Idle (`continue = false`) after the speak directive (if any) resolves.

The command shape is **schema-extensible** in future revisions. Forward-compatibility expectations:

- New optional directives (e.g. `ask_permission`, `transition_to_state`) may be added in later HLD revisions.
- The host does not construct directive lists at runtime. Every directive is a known field in the schema.
- Unknown directive fields are ignored by the plugin for forward compatibility; missing required fields fail validation.

Execution order when both directives are present: the plugin completes `speak` (including playback), then applies `continue`.

## 7. Intermediate State Declaration Schema

Hosts may declare additional overlay states at config time, executed during the host's window (between `HandedOff` and the plugin's loop resumption). These states are **not** plugin-owned states; they are overlay views the plugin renders on the host's behalf.

Each declared intermediate state specifies:

- `name` — unique identifier within the host configuration.
- `viewRef` — reference to the host-supplied view used to render this state.
- `entryCondition` — declarative trigger describing when the plugin should display this state (e.g. "after handoff, before return command resolves").
- `exitCondition` — declarative trigger describing when the plugin should advance past this state (e.g. "on return command received", "on timeout").
- `timeoutMs` — optional max duration before automatic advance.

The schema is **closed**: hosts cannot inject states at runtime, only declare them in plugin configuration during initialization. The plugin validates the declared set at bootstrap and rejects malformed declarations.

The set of valid `entryCondition` and `exitCondition` values is enumerated by the plugin and grows only through HLD revisions, mirroring the return-command extensibility model.

## 8. Re-Engagement and Multi-Turn

The plugin is stateless across turns. There is no plugin-side conversation memory, no turn counter, no carried context.

Multi-turn behavior is implemented entirely by the host:

- Host stores conversation context in its own backend.
- Host re-engages the plugin by calling `startListening` (or equivalent loop-initiation command) again.
- No initial overlay state hint is passed on re-entry; the plugin always begins a new interaction in `Listening`.
- Each interaction has a fresh `interactionId`.

`continue = true` in the return command is a convenience that lets the plugin re-arm listening without a round-trip back to the host's loop logic, but it is functionally equivalent to the host calling `startListening` again. Either path is supported.

## 9. Overlay Ownership and Recorder Bridge Boundary

Overlay rendering remains in the recorder bridge for the current cycle. The voice plugin does **not** take ownership of overlay primitives in this HLD.

Implications:

- The plugin coordinates overlay state transitions through the existing recorder bridge channel.
- Host-declared intermediate states are rendered via the recorder bridge's overlay surface.
- A future HLD may move overlay ownership into the voice plugin directly; this is explicitly deferred.

The interface defined here is **overlay-implementation-agnostic**: the state machine, handoff event, return command, and intermediate state schema do not depend on where the overlay code physically lives.

## 10. Out of Scope

- Implementation work of any kind (state machine code, event plumbing, schema validation).
- Conversation context, agent orchestration, LLM routing, or any "brain" logic.
- Programmatic runtime composition of directive lists.
- Initial overlay state hints on re-entry.
- Moving overlay ownership out of the recorder bridge.
- TTS runtime/model changes beyond what the existing TTS streaming MVP provides.
- Wire-format choices for events and commands (deferred to LLD).
- Persistence of interaction history or audio artifacts.

## 11. Risks and Mitigations

- **Risk:** Handoff timeout strands the user mid-interaction.
  - Mitigation: Timeout transitions to Idle with a state-change event; hosts can detect and recover.
- **Risk:** Host returns a command for a stale `interactionId`.
  - Mitigation: Plugin rejects commands whose `interactionId` does not match the current `HandedOff` interaction.
- **Risk:** Closed schema for intermediate states blocks legitimate host needs.
  - Mitigation: Schema extends through HLD revisions; the cost of one design pass is preferable to runtime composition.
- **Risk:** `continue = true` and host-initiated re-engagement diverge in behavior.
  - Mitigation: Both paths funnel through the same internal `startListening` entry point.
- **Risk:** Overlay-in-recorder-bridge constrains the visible state contract.
  - Mitigation: The state machine and event surface defined here are overlay-implementation-agnostic and survive a future overlay ownership move.
- **Risk:** Hosts treat the plugin as stateful and store context inside it.
  - Mitigation: No plugin-side context fields; explicit documentation that re-engagement is fresh.

## 12. Validation Expectations

- Plugin exposes a visible state machine with the canonical states from §4.
- Transcription completion emits a handoff event with the fields from §5.
- The plugin honours a single tagged return command per `interactionId` with the directives from §6.
- Intermediate overlay states can only be added via config-time declaration; runtime injection is rejected.
- Re-engagement requires a fresh `startListening` call (directly or via `continue = true`) and does not carry context.
- Cancellation and failure transitions return to Idle with state-change events.
- The contract is consumable by a host that uses STT only, TTS only, or both, without changes to the boundary.

## 13. Exit Criteria

- This HLD is approved as the architecture boundary for the voice interface.
- The next task writes an LLD translating these decisions into command names, event channel names, payload shapes, configuration schema, and file-level edit plans.
- No implementation begins until the LLD is approved.
