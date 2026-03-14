# CODIN-242 HLD

## Objective
Define the recorder plugin API surface and event contract as a stable boundary for consumers.

## Scope
- Define command surface and payload families for:
  - `initialize_runtime`
  - `start_recording`
  - `stop_recording`
  - `set_hotkey_bindings`
  - `capture_hotkey`
  - `get_runtime_state`
- Define event surface and payload families for:
  - `state`
  - `live`
  - `error`
  - `complete`
- Define stable machine-parseable error contract.

## Out of Scope
- Runtime implementation details for audio/hotkeys/overlay/orchestration.
- Consumer migration implementation.
- UI/overlay rendering implementation.

## Design
### 1. Contract-first boundary
- Consumers integrate through typed commands/events only.
- Plugin internals remain private and replaceable.

### 2. Command model
- Commands return typed success or typed error payloads.
- Hotkey binding contract may include recorder execution context fields.

### 3. Event model
- Events are emitted as typed envelopes.
- Event ordering/terminal semantics are defined at contract level.

### 4. Error contract
- Errors expose stable machine-parseable codes and user-safe messages.
- Error code set is append-only.

## Validation
- Contract matrix is documented for commands/events and error mappings.
- Contract-level serialization/smoke checks are defined.

## Deliverables
- Concise HLD for `CODIN-242` scope and boundary.
- Follow-on LLD for concrete schemas, field-level constraints, and behavior details.
