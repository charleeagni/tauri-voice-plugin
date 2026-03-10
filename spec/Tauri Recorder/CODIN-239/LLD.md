# CODIN-239 LLD

## Files To Update
- tauri-plugin-stt/Cargo.toml
- tauri-plugin-stt/README.md
- tauri-plugin-stt/tests/smoke_contract.rs

## Detailed Changes
1. Add test-only dependency
- Add `serde_json` under dev dependencies in Cargo.toml.
- Keep runtime dependencies unchanged.

2. Add smoke contract tests
- Create `tests/smoke_contract.rs` with focused tests for:
  - Bootstrap response tagged enum serialization.
  - Health response diagnostics payload shape.
  - Typed error code serialization for representative errors.
  - Transcribe request serialization shape.
- Keep tests deterministic and isolated from OS/runtime side effects.

3. Expand README
- Add objective and platform constraints.
- Add clean-machine bootstrap flow.
- Add command contract with request and response field definitions.
- Add typed error codes list.
- Add smoke test execution section.
- Add troubleshooting section for common failures.

## Verification Steps
- Run `cargo test` in `tauri-plugin-stt`.
- Confirm no production code behavior changes.
- Confirm README instructions reference current paths and commands.
