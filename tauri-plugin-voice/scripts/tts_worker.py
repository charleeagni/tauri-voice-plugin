#!/usr/bin/env python3
"""
Long-lived TTS worker process.

Loads the Kokoro model once at startup, then processes synthesis requests from
stdin as newline-delimited JSON.

Request format:  {"id":"<req_id>","text":"...","output":"<wav_path>",
                  "model":"<model_repo>","voice":"af_heart",
                  "language":"a","speed":1.0}
Response format: {"id":"<req_id>","audio":"<wav_path>","model":"...",
                  "voice":"...","language":"a","duration_ms":1234,
                  "sample_rate_hz":24000,"format":"wav","error":null}
Error response:  {"id":"<req_id>","audio":null,"error":"<message>",
                  "error_code":"generation_failed"}
Ready signal:    {"status":"ready","model":"<model_repo>"}
Progress line:   {"status":"progress","phase":"download"|"preload",
                  "state":"start"|"complete"|"failed","model":"...",
                  "percent":0.0-1.0|null,"filename":null,
                  "error":"<msg>"|null}
"""
import argparse
import json
import os
import sys
import wave

# Redirect all normal print() / stdout to stderr to prevent third-party
# libraries (like spacy or huggingface_hub) from corrupting the JSON stream.
_stdout = sys.stdout
sys.stdout = sys.stderr

os.environ["HF_HUB_DISABLE_XET"] = "1"

import numpy as np
from mlx_audio.tts.utils import load_model


def _emit_json(payload):
    """Emit structured JSON safely to the original stdout."""
    _stdout.write(json.dumps(payload, separators=(",", ":")) + "\n")
    _stdout.flush()


def _emit_progress(phase, state, model, percent=None, filename=None, error=None):
    """Emit structured progress JSON to stdout."""
    _emit_json(
        {
            "status": "progress",
            "phase": phase,
            "state": state,
            "model": model,
            "percent": percent,
            "filename": filename,
            "error": error,
        }
    )


def _load_model(model_id):
    """Load the configured TTS model.

    :param model_id: HuggingFace model repo identifier.
    :return: Loaded mlx-audio model object.
    :raises Exception: When model download or preload fails.
    """
    _emit_progress("download", "start", model_id)
    _emit_progress("preload", "start", model_id)

    try:
        model = load_model(model_id)
    except Exception as exc:
        _emit_progress("download", "failed", model_id, error=str(exc))
        _emit_progress("preload", "failed", model_id, error=str(exc))
        raise

    _emit_progress("download", "complete", model_id, percent=1.0)
    _emit_progress("preload", "complete", model_id, percent=1.0)
    return model


def _as_numpy(audio):
    """Convert generated audio to a flat numpy array."""
    if hasattr(audio, "tolist"):
        return np.asarray(audio.tolist(), dtype=np.float32).reshape(-1)
    return np.asarray(audio, dtype=np.float32).reshape(-1)


def _write_wav(path, audio, sample_rate):
    """Write mono float audio as PCM16 WAV.

    :param path: Destination WAV path.
    :param audio: Generated float waveform.
    :param sample_rate: Output sample rate in hertz.
    :return: Duration in milliseconds.
    :raises Exception: When writing the WAV file fails.
    """
    samples = np.clip(_as_numpy(audio), -1.0, 1.0)
    pcm = (samples * 32767.0).astype(np.int16)

    with wave.open(path, "wb") as wav_file:
        wav_file.setnchannels(1)
        wav_file.setsampwidth(2)
        wav_file.setframerate(sample_rate)
        wav_file.writeframes(pcm.tobytes())

    if sample_rate <= 0:
        return None
    return int((len(pcm) / sample_rate) * 1000)


import base64
import struct
import threading


def _generate(model, req):
    """Generate a WAV file for one worker request."""
    text = req["text"]
    output = req["output"]
    model_id = req["model"]
    voice = req["voice"]
    language = req.get("language") or "a"
    speed = float(req.get("speed") or 1.0)

    results = list(
        model.generate(
            text=text,
            voice=voice,
            speed=speed,
            lang_code=language,
        )
    )
    if not results:
        raise RuntimeError("TTS model returned no audio")

    audios = [_as_numpy(result.audio) for result in results]
    audio = np.concatenate(audios) if len(audios) > 1 else audios[0]
    sample_rate = int(
        getattr(results[0], "sample_rate", None)
        or getattr(results[0], "sample_rate_hz", None)
        or 24000
    )
    duration_ms = _write_wav(output, audio, sample_rate)

    return {
        "id": req.get("id", ""),
        "audio": output,
        "model": model_id,
        "voice": voice,
        "language": language,
        "duration_ms": duration_ms,
        "sample_rate_hz": sample_rate,
        "format": "wav",
        "error": None,
    }


def _float_to_pcm16(samples):
    """Convert float32 numpy array to PCM16 little-endian bytes.

    :param samples: Float numpy array in [-1.0, 1.0].
    :return: Raw PCM16 bytes.
    """
    clipped = np.clip(samples, -1.0, 1.0)
    pcm = (clipped * 32767.0).astype(np.int16)
    return pcm.tobytes()


def _stream_generate(model, req, cancel_event):
    """Generate audio in streaming mode and emit chunk events via stdout.

    :param model: Loaded TTS model.
    :param req: Parsed request dict with stream fields.
    :param cancel_event: threading.Event; set when cancellation is requested.
    :raises RuntimeError: When the model returns no results.
    """
    req_id = req.get("id", "")
    model_id = req.get("model", "")
    voice = req.get("voice", "af_heart")
    language = req.get("language") or "a"
    speed = float(req.get("speed") or 1.0)
    chunk_duration_ms = int(req.get("chunk_duration_ms") or 200)
    text = req["text"]

    results = list(
        model.generate(
            text=text,
            voice=voice,
            speed=speed,
            lang_code=language,
        )
    )
    if not results:
        raise RuntimeError("TTS model returned no audio")

    # Assemble full audio before chunking (mlx-audio generates complete results).
    audios = [_as_numpy(result.audio) for result in results]
    audio = np.concatenate(audios) if len(audios) > 1 else audios[0]
    sample_rate = int(
        getattr(results[0], "sample_rate", None)
        or getattr(results[0], "sample_rate_hz", None)
        or 24000
    )

    _emit_json({
        "status": "stream_start",
        "id": req_id,
        "sequence": 0,
        "model": model_id,
        "voice": voice,
        "language": language,
        "sample_rate_hz": sample_rate,
        "channels": 1,
        "format": "pcm_s16le",
    })

    # Chunk by frame count derived from chunk_duration_ms.
    frames_per_chunk = max(1, int(sample_rate * chunk_duration_ms / 1000))
    total_frames = len(audio)
    sequence = 1

    for offset in range(0, total_frames, frames_per_chunk):
        if cancel_event.is_set():
            _emit_json({"status": "stream_cancelled", "id": req_id, "sequence": sequence})
            return

        chunk = audio[offset: offset + frames_per_chunk]
        pcm_bytes = _float_to_pcm16(chunk)
        duration_ms = int(len(chunk) / sample_rate * 1000)
        is_final = (offset + frames_per_chunk) >= total_frames

        _emit_json({
            "status": "stream_chunk",
            "id": req_id,
            "sequence": sequence,
            "sample_rate_hz": sample_rate,
            "channels": 1,
            "format": "pcm_s16le",
            "audio_base64": base64.b64encode(pcm_bytes).decode("ascii"),
            "duration_ms": duration_ms,
            "final": is_final,
        })
        sequence += 1

    _emit_json({"status": "stream_complete", "id": req_id, "sequence": sequence})


def main() -> int:
    parser = argparse.ArgumentParser(description="Long-lived TTS worker.")
    parser.add_argument("--model", required=True, help="TTS model repo/id")
    args = parser.parse_args()

    model = _load_model(args.model)
    _emit_json({"status": "ready", "model": args.model})

    # cancel_event signals an in-progress stream to stop.
    cancel_event = threading.Event()
    active_stream_id = None

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        req_id = ""
        try:
            req = json.loads(line)
            req_id = req.get("id", "")

            # Handle explicit cancel command.
            if req.get("command") == "cancel":
                if req_id == active_stream_id:
                    cancel_event.set()
                continue

            mode = req.get("mode", "file")

            if mode == "stream":
                cancel_event.clear()
                active_stream_id = req_id

                try:
                    _stream_generate(model, req, cancel_event)
                except Exception as exc:
                    _emit_json({
                        "status": "stream_error",
                        "id": req_id,
                        "sequence": 0,
                        "error": str(exc),
                    })
                finally:
                    active_stream_id = None
                    cancel_event.clear()
            else:
                response = _generate(model, req)
                _emit_json(response)

        except Exception as exc:
            _emit_json(
                {
                    "id": req_id,
                    "audio": None,
                    "error": str(exc),
                    "error_code": "generation_failed",
                }
            )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
