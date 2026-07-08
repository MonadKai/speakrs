from __future__ import annotations

import asyncio
from pathlib import Path

import pytest

import speakrs


REPO_ROOT = Path(__file__).resolve().parents[3]


def test_import_surface_and_defaults():
    assert speakrs.__version__ == "0.5.0"
    assert speakrs.ExecutionMode.CPU.value == "CPU"
    assert "segmentation-3.0.onnx" in speakrs.required_model_files()


def test_prepare_fixture_models_and_diarize_file():
    events = []
    prepared = speakrs.prepare(model_dir=REPO_ROOT / "fixtures" / "models")
    pipeline = speakrs.Pipeline.from_prepared(prepared)

    result = pipeline.diarize_file(
        REPO_ROOT / "fixtures" / "test_short.wav",
        progress=events.append,
    )

    assert result.mode == "CPU"
    assert result.model_revision == speakrs.DEFAULT_MODEL_REVISION
    assert result.duration > 0.0
    assert result.timing.total_ms >= result.timing.pipeline_ms
    assert events == [
        speakrs.ProgressEvent.DecodingAudio,
        speakrs.ProgressEvent.ResamplingAudio,
        speakrs.ProgressEvent.Completed,
    ]


def test_async_file_api_and_cancel_token():
    prepared = speakrs.prepare(model_dir=REPO_ROOT / "fixtures" / "models")
    pipeline = speakrs.Pipeline.from_prepared(prepared)

    result = asyncio.run(
        pipeline.diarize_file_async(REPO_ROOT / "fixtures" / "test_short.wav")
    )
    assert result.mode == "CPU"

    token = speakrs.CancelToken()
    token.cancel()
    with pytest.raises(speakrs.SpeakrsError) as exc:
        pipeline.diarize_samples([0.0] * 16_000, cancel_token=token)
    assert exc.value.args[0] == "cancelled"


def test_queue_accepts_sample_and_path_jobs():
    prepared = speakrs.prepare(model_dir=REPO_ROOT / "fixtures" / "models")
    pipeline = speakrs.Pipeline.from_prepared(prepared)
    queue = pipeline.into_queue()

    sample_job_id = queue.push_samples("sample", [0.0] * 16_000)
    path_job_id = queue.push_file("path", REPO_ROOT / "fixtures" / "test_short.wav")
    sample_result = queue.recv()
    path_result = queue.recv()

    assert {sample_result.job_id, path_result.job_id} == {sample_job_id, path_job_id}
    assert {sample_result.file_id, path_result.file_id} == {"sample", "path"}
    assert sample_result.ok
    assert path_result.ok
    sample = sample_result.unwrap()
    path = path_result.unwrap()
    assert sample.mode == "CPU"
    assert path.mode == "CPU"
    assert sample.duration > 0.0
    assert path.duration > 0.0


def test_cli_diarizes_fixture(capsys):
    from speakrs.cli import main

    exit_code = main(
        [
            str(REPO_ROOT / "fixtures" / "test_short.wav"),
            "--model-dir",
            str(REPO_ROOT / "fixtures" / "models"),
        ]
    )
    captured = capsys.readouterr()

    assert exit_code == 0
    assert captured.err == ""


def test_cli_prints_clean_speakrs_errors(capsys, tmp_path):
    from speakrs.cli import main

    exit_code = main(
        [
            str(REPO_ROOT / "fixtures" / "test_short.wav"),
            "--model-dir",
            str(tmp_path / "missing-models"),
        ]
    )
    captured = capsys.readouterr()

    assert exit_code == 1
    assert captured.out == ""
    assert captured.err.startswith("error: ")
    assert "model manifest is missing" in captured.err
    assert "Traceback" not in captured.err
