#!/usr/bin/env python3
import argparse
import sys

import mlx_whisper


def main() -> int:
    parser = argparse.ArgumentParser(description="Transcribe an audio file with mlx_whisper.")
    parser.add_argument("--audio", required=True, help="Absolute path to audio file")
    parser.add_argument("--model", required=True, help="Whisper model repo/id")
    args = parser.parse_args()

    try:
        result = mlx_whisper.transcribe(args.audio, path_or_hf_repo=args.model)
        if isinstance(result, dict):
            text = str(result.get("text", "")).strip()
        else:
            text = str(result).strip()
        print(text)
        return 0
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
