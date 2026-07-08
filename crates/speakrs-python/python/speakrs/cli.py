from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Sequence

from . import ExecutionMode, Pipeline, SpeakrsError, prepare


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="speakrs")
    parser.add_argument("audio", type=Path)
    parser.add_argument("--model-dir", type=Path, default=None)
    parser.add_argument("--cache-dir", type=Path, default=None)
    parser.add_argument("--mode", choices=[mode.value for mode in ExecutionMode], default="CPU")
    parser.add_argument("--file-id", default="file1")
    args = parser.parse_args(argv)

    try:
        prepared = prepare(
            ExecutionMode(args.mode),
            cache_dir=args.cache_dir,
            model_dir=args.model_dir,
        )
        pipeline = Pipeline.from_prepared(prepared, ExecutionMode(args.mode))
        result = pipeline.diarize_file(args.audio, file_id=args.file_id)
        print(result.rttm, end="")
        return 0
    except SpeakrsError as exc:
        message = exc.args[-1] if exc.args else str(exc)
        print(f"error: {message}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
