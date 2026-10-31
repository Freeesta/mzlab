"""The app (`flow/run` of the desktop engine, the files handed over as `b64:` text like the Flow page does) gives the bytes of a native `flow run` for a plan with `imported_lists` on
`screen.suspects` and `annot.regulatory`; the browser leg of the same three-way parity is `tests/test_normative_screen_wasm.py` (same native run, same inputs). The native run and its
inputs come from the Rust test `both_paths_run_in_one_plan_for_the_wasm_parity`. Skipped without cargo."""
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
pytestmark = pytest.mark.skipif(not shutil.which("cargo"), reason="cargo not available")


def run(*a, env):
    return subprocess.run(a, cwd=ROOT, check=True, capture_output=True, env=env)


def test_the_app_has_the_bytes_of_flow_run_with_imported_lists(tmp_path):
    env = {**os.environ, "MZLAB_PARITA_NORMATIVE": str(tmp_path / "run")}
    run("cargo", "test", "--locked", "-q", "-p", "mzlab-cli", "--lib", "both_paths_run_in_one_plan_for_the_wasm_parity", env=env)
    assert (tmp_path / "run" / "s.json").stat().st_size > 0
    run("cargo", "test", "--locked", "-q", "-p", "mzlab-desktop", "--test", "flow_imported_parity", env=env)
