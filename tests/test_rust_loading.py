"""Loading helpers of the Rust engine (rust-centroidi.js, rust-xicbin.js, rust-recycle.js) against fake bridge handles (tests/rust_loading_node.mjs)."""
import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
NODE = shutil.which("node")


@pytest.fixture(scope="module")
def out():
    if not NODE:
        pytest.skip("node not available")
    return json.loads(subprocess.run([NODE, "tests/rust_loading_node.mjs"], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout)


def test_cancel_during_quota_check_does_not_start_the_conversion(out):
    assert out["abortInWindow"]["cancelled"] is True and out["abortInWindow"]["rpc"] == 0


def test_cancel_stops_the_current_worker(out):
    assert out["abortStaleWorker"]["aborted"] == ["w2"]


def test_quota_errors_keep_their_shape_and_other_errors_pass_through(out):
    assert out["quotaJson"]["key"] == "error.quota"
    assert out["plainError"]["name"] == "TypeError"


def test_xicbin_rejects_a_non_numeric_tolerance(out):
    assert out["tolNaN"] == {"res": None, "rpc": 0}


def test_xicbin_returns_every_buffer_even_when_an_answer_fails(out):
    assert out["failureRecycles"]["res"] is None and out["failureRecycles"]["recycled"] == 2


def test_xicbin_sends_together_and_keeps_the_order(out):
    s = out["success"]
    assert s["order"] == [2, 0, 1] and s["n"] == [0, 2, 1] and s["recycled"] == 3 and s["rust"] == 1 and s["parallel"] == 3


def test_recycler_now_clears_the_timer_and_honours_disabled(out):
    assert out["recycler"] == {"armed": True, "cleared": True, "recycled": 1}
