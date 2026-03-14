# Low Level Design: Recorder Plugin API Contract and Event Schema (CODIN-242)

## 1. Purpose

Define concrete command/event/error schemas for the recorder plugin boundary used by consumers.

## 2. Locked Scope

In scope command contracts:
- `initialize_runtime`
- `start_recording`
- `stop_recording`
- `set_hotkey_bindings`
- `capture_hotkey`
- `get_runtime_state`

In scope event contracts:
- `state`
- `live`
- `error`
- `complete`

Out of scope:
- `interim` event.
- Recorder runtime internals and consumer migration implementation.

## 3. Command Contracts

### 4.1 initialize_runtime

Request fields:

Success fields:
- `status` (`ready` or `not_ready`)
- `readiness` (typed readiness summary)

Error fields:
- Standard error contract.

### 4.2 start_recording

Request fields:
- `outputDir` (optional absolute path)
- `fileNamePrefix` (optional)

Success fields:
- `sessionId`
- `status` (`recording`)
- `startedAt`

Error fields:
- Standard error contract.

### 4.3 stop_recording

Request fields:
- `sessionId`

Success fields:
- `status` (`stopped`)
- `path`
- `durationMs`

Error fields:
- Standard error contract.

### 4.4 set_hotkey_bindings

Request fields:
- `bindings` (hotkey mapping payload)
- `recorderConfig`
  - `outputDir` (optional absolute path)
  - `fileNamePrefix` (optional)

Success fields:
- `status` (`updated`)
- `effectiveBindings`

Error fields:
- Standard error contract.

Notes:
- `recorderConfig` defines defaults consumed by hotkey-triggered recording.
- Binding update is treated as atomic replace.

### 4.5 get_runtime_state

Request fields:

Success fields:
- `runtimeState`
  - `phase` (`idle`, `recording`, `transcribing`, `error`)
  - `sessionId` (optional)
  - `readiness`
  - `effectiveBindings` (optional)

Error fields:
- Standard error contract.

### 4.6 capture_hotkey

Request fields:
- `timeoutMs` (optional)

Success fields:
- `status` (`captured` or `cancelled` or `timeout`)
- `binding` (present when `captured`)

Error fields:
- Standard error contract.

## 4. Event Envelope and Payload Contracts

Common envelope:
- `eventType`
- `timestamp`
- `sessionId` (optional)
- `payload`

### 5.1 state event
- Emits recorder phase transition payload.
- Payload includes current phase and optional reason.

### 5.2 live event
- Emits live transcript/preview payload for active session.
- Payload includes text segment and sequence metadata.

### 5.3 error event
- Emits non-fatal or terminal error payload.
- Payload embeds standard error contract fields.

### 5.4 complete event
- Emits terminal completion payload for recording/transcription session.
- Payload includes final artifact references and completion metadata.

## 5. Error Contract

Standard error payload:
- `code`
- `message`
- `details` (optional)
- `retryable`

Rules:
- `code` is machine-parseable and stable.
- Existing codes are never repurposed.
- New codes are additive only.

## 6. Event/Query Consistency Rules

- Event stream is primary for live updates.
- `get_runtime_state` is the reconciliation snapshot for startup/reconnect.
- Consumers must tolerate receiving snapshot plus subsequent events.

## 7. Validation Plan

- Serialization tests for all command request/response structs.
- Serialization tests for all event envelopes and payload variants.
- Error code stability tests.
- Compatibility tests ensuring optional-field additions do not break `v1` decoding.

## 8. Exit Criteria

- All in-scope commands/events have concrete `v1` schema definitions.
- `interim` and `capture_hotkey` are excluded from this ticket scope.
- Contract tests exist for command/event/error serialization stability.
