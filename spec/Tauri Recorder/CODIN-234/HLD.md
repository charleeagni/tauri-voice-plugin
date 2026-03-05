# High Level Design: Plugin Scaffold and uv Sidecar Packaging Skeleton (CODIN-234)

## 1. Objective

Create a standalone Tauri plugin repository scaffold in the current project root that establishes the structural foundation for a reusable STT plugin using a bundled `uv` sidecar.

This story delivers repository and packaging skeleton only.

## 2. Parent Context

Parent: `CODIN-233`.

Locked constraints inherited from parent:
- STT-only scope.
- Local-only runtime.
- macOS Apple Silicon target (`aarch64-apple-darwin`).
- Separate plugin repository architecture.
- Sidecar-only process execution model.
- Request/response API style.
- Bootstrap initiated by user command.

Runtime lock for this stream:
- Python version is fixed to `3.14`.

## 3. Problem Context

The plugin cannot progress to bootstrap and transcription implementation without a consistent repository layout, sidecar declaration points, and packaging placeholders.

If these foundations are delayed, downstream tasks (`CODIN-235` to `CODIN-239`) risk drift in path conventions, build expectations, and command wiring.

## 4. Scope

In scope:
- Scaffold plugin-focused repository structure in current root.
- Add initial Rust plugin crate layout and command surface placeholders.
- Add Tauri sidecar declaration placeholders for `uv` integration.
- Define app-data path conventions used by later tasks.
- Add placeholder dependency lock artifacts and packaging stubs.
- Add minimal documentation for scaffold purpose and boundaries.

Out of scope:
- Implementing bootstrap execution flow.
- Installing Python dependencies.
- Running MLX Whisper transcription.
- Final typed error implementation.
- Health/readiness logic beyond placeholder interfaces.
- Consumer integration into Steno.

## 5. Proposed Solution

Establish a minimal plugin architecture with clear boundaries:

- Plugin crate boundary:
  - Rust crate exposes command signatures only.
  - Command handlers return placeholder "not implemented" responses.
  - No runtime process execution in this story.

- Sidecar packaging boundary:
  - Add sidecar registration placeholders for `uv` binary packaging.
  - Reserve target-specific sidecar naming for macOS Apple Silicon.
  - Keep security posture explicit: only sidecar invocation channel will be allowed later.

- Runtime path contract boundary:
  - Reserve canonical app-data paths for future tasks:
    - `APP_DATA_DIR/python/.venv`
    - `APP_DATA_DIR/python/.venv/bin/python`
    - `APP_DATA_DIR/models`
    - `APP_DATA_DIR/logs`

- Dependency reproducibility boundary:
  - Include lock-file placeholder for STT dependency pinning.
  - Defer actual package resolution to bootstrap implementation story.

## 6. High-Level Component View

- Repository Layer
  - Top-level plugin project metadata.
  - Documentation and constraints.

- Plugin Runtime Layer (Rust)
  - Command registration shell:
    - `bootstrap_stt`
    - `transcribe_file`
    - `stt_health`
  - Placeholder response schema surface.

- Packaging Layer
  - Tauri sidecar config placeholders for `uv`.
  - Platform target constraints recorded.

- Future Runtime Contract Layer
  - Path constants and policy notes used by downstream implementation tasks.

## 7. Deliverables for CODIN-234

- Plugin repo scaffold in current directory.
- Initial Rust plugin command skeleton with no functional execution.
- Sidecar packaging placeholders for `uv`.
- Placeholder dependency lock artifact location.
- README section documenting scaffold purpose and next-story boundaries.

## 8. Non-Functional Expectations

- Readability-first project structure.
- Deterministic naming and path conventions.
- No hidden runtime side effects.
- Security posture documented at scaffold level.

## 9. Risks and Mitigations

- Risk: Scaffold includes accidental runtime behavior.
  - Mitigation: Keep command handlers placeholder-only.

- Risk: Path conventions diverge across future tasks.
  - Mitigation: Record canonical `APP_DATA_DIR` paths in scaffold docs.

- Risk: Sidecar naming mismatch during packaging.
  - Mitigation: Define target-specific placeholder naming early and reuse unchanged.

## 10. Success Criteria

- Repository has clear plugin scaffold structure and build placeholders.
- Command surface exists for `bootstrap_stt`, `transcribe_file`, and `stt_health`.
- Sidecar packaging placeholders for `uv` are present.
- Python runtime target is documented as `3.14`.
- Scope remains limited to scaffold only, with no functional bootstrap/transcription behavior.

## 11. Implementation Notes for Next Story

`CODIN-235` will implement idempotent bootstrap logic against this scaffold and should not alter established path contracts unless explicitly approved.
