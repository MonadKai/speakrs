from __future__ import annotations

from pathlib import Path

import speakrs


repo_root = Path(__file__).resolve().parents[3]
prepared = speakrs.prepare(model_dir=repo_root / "fixtures" / "models")
pipeline = speakrs.Pipeline.from_prepared(prepared)
result = pipeline.diarize_file(repo_root / "fixtures" / "test.wav")
print(result.rttm, end="")
