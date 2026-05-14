# Voice Plugin Scope (Locked)

Companion to `PRD.md`. Resolves the open questions; supersedes them.

## Boundary

The plugin is a reusable **voice interface layer**: ears, mouth, overlay.
The host application is the **brain**: conversation logic, context, action routing.

## Plugin State Machine (visible)

```
Idle → Listening → Capturing → Transcribing → HandedOff → Idle
```

All states are externally visible. No opaque sub-states.

## Handoff Contract

- **Plugin → Host (handoff out):** event emitted on transcription complete, carrying the transcript.
- **Host → Plugin (handoff return):** tagged command with two directives today:
  - `speak`: speak a response (TTS on/off + text).
  - `continue`: keep the loop alive or end it.
- Shape is **extensible by schema**, not programmatically. New directives (e.g. `ask_permission`) can be added in future revisions of the contract, but the host does not compose directive lists at runtime.

## Overlay Ownership

- Plugin owns **lifecycle, rendering, transitions**.
- Host owns **content** for each state and may declare custom intermediate states **at config time** (not runtime). Each declared state ships with its own view.

## Conversation Context

The plugin is **stateless across turns**. The host backend owns conversation memory and re-engages the plugin via a plain `startListening` call. No initial overlay state hint is passed on re-entry.

## Out of Scope

- Overlay/recorder ownership reshuffle (stays bridged for now).
- Programmatic directive composition.
- Conversation context, multi-turn memory, agent/LLM orchestration.
- Initial overlay state hints on re-entry.

## Next Step

Draft the HLD covering: state machine, handoff event payload, return-command schema, intermediate state declaration schema. No implementation yet.
