#!/usr/bin/env python3
"""
Long-lived STT worker process.

Loads the Whisper model once at startup (triggering download if not cached),
then processes transcription requests from stdin as newline-delimited JSON.

Request format:  {"id": "<req_id>", "audio": "<absolute_path>"}
Response format: {"id": "<req_id>", "text": "<transcript>", "error": null}
Error response:  {"id": "<req_id>", "text": null, "error": "<message>"}
Ready signal:    {"status": "ready", "model": "<model_repo>"}
Progress line:   {"status": "progress", "phase": "download"|"preload",
                  "state": "start"|"in_progress"|"complete"|"failed",
                  "model": "<model_repo>", "percent": 0.0-1.0|null,
                  "filename": "<file>"|null, "error": "<msg>"|null}
"""
import argparse
import json
import sys

import tqdm
import mlx_whisper
from mlx_whisper import load_models
from huggingface_hub import snapshot_download


def _emit_progress(phase, state, model, percent=None, filename=None, error=None):
    """Emit a structured progress JSON line to stdout (compact, no spaces)."""
    print(
        json.dumps(
            {
                "status": "progress",
                "phase": phase,
                "state": state,
                "model": model,
                "percent": percent,
                "filename": filename,
                "error": error,
            },
            separators=(",", ":"),
        ),
        flush=True,
    )


def _make_progress_tqdm(model_repo):
    """Return a tqdm subclass that emits JSON progress lines for the download phase."""

    class _ProgressTqdm(tqdm.tqdm):
        """Wraps tqdm; serialises HuggingFace download progress to stdout."""

        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)

            # Emit start once tqdm initialises with a known total.
            _emit_progress(
                "download",
                "start",
                model_repo,
                percent=None,
                filename=getattr(self, "desc", None),
            )

        def update(self, n=1):
            super().update(n)

            # Compute fraction; None when total is unknown.
            total = self.total or 0
            percent = round(self.n / total, 4) if total > 0 else None

            _emit_progress(
                "download",
                "in_progress",
                model_repo,
                percent=percent,
                filename=getattr(self, "desc", None),
            )

        def close(self):
            if not getattr(self, "_closed_progress", False):
                self._closed_progress = True
                _emit_progress("download", "complete", model_repo, percent=1.0)
            super().close()

    return _ProgressTqdm


def _load_model_with_progress(model_repo):
    """Download and load model; emit download and preload progress events.

    Downloads via snapshot_download with token=False (anonymous, works for all
    public mlx-community repos regardless of any stored HF credentials).
    Passes the cached local path to load_model for the preload phase.

    :param model_repo: HuggingFace repo ID, e.g. mlx-community/whisper-tiny.en
    :return: Local cache path string for use in transcription calls.
    """
    # --- Download phase ---
    _emit_progress("download", "start", model_repo)

    # huggingface_hub 1.x uses hf_tqdm internally; module-attribute patching
    # does not reach the actual download progress bars. Use the official
    # tqdm_class parameter instead (supported since huggingface_hub 0.21).
    _ProgressTqdm = _make_progress_tqdm(model_repo)

    try:
        # token=False forces anonymous access; avoids stale keychain credentials.
        local_path = snapshot_download(repo_id=model_repo, token=False, tqdm_class=_ProgressTqdm)
    except Exception as exc:
        _emit_progress("download", "failed", model_repo, error=str(exc))
        raise

    _emit_progress("download", "complete", model_repo, percent=1.0)

    # --- Preload phase: load weights from local cache into memory ---
    _emit_progress("preload", "start", model_repo)

    try:
        load_models.load_model(local_path)
    except Exception as exc:
        _emit_progress("preload", "failed", model_repo, error=str(exc))
        raise

    _emit_progress("preload", "complete", model_repo, percent=1.0)

    return local_path


def main() -> int:
    parser = argparse.ArgumentParser(description="Long-lived STT worker.")

    # Model repo identifier, e.g. mlx-community/whisper-tiny
    parser.add_argument("--model", required=True, help="Whisper model repo/id")
    args = parser.parse_args()

    # Neutralise any stale keychain credentials for this process only.
    # huggingface_hub reads keyring at request time; models are public repos.
    try:
        import keyring
        import keyring.backend

        class _NullKeyring(keyring.backend.KeyringBackend):
            priority = 10
            def get_password(self, service, username): return None
            def set_password(self, service, username, password): pass
            def delete_password(self, service, username): pass

        keyring.set_keyring(_NullKeyring())
    except Exception:
        pass

    # Load model into memory; returns local cache path for transcription.
    local_model_path = _load_model_with_progress(args.model)

    # Signal readiness to the parent Rust process.
    print(json.dumps({"status": "ready", "model": args.model}), flush=True)

    # Process transcription requests until stdin is closed.
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        req_id = ""
        try:
            req = json.loads(line)
            req_id = req.get("id", "")
            audio_path = req["audio"]

            # Use local path; avoids HF auth on every transcription call.
            result = mlx_whisper.transcribe(audio_path, path_or_hf_repo=local_model_path)

            if isinstance(result, dict):
                text = str(result.get("text", "")).strip()
            else:
                text = str(result).strip()

            print(json.dumps({"id": req_id, "text": text, "error": None}), flush=True)

        except Exception as exc:
            print(json.dumps({"id": req_id, "text": None, "error": str(exc)}), flush=True)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
