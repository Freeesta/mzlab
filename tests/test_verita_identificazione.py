"""`tools/bench/verita_identificazione.py` on an invented pair of libraries (no public data needed): the truth is the InChIKey block, the answer is
computed by the Rust example and must be the same on every run; without files the tool skips."""
import importlib.util
import random
import shutil
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("verita_id", ROOT / "tools/bench/verita_identificazione.py")
V = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V)
pytestmark = pytest.mark.skipif(shutil.which("cargo") is None, reason="cargo is not installed")


def msp(n, noise, seed):
    """n invented compounds; `noise` jitters the intensities, never the m/z, so the truth is the same compound in both files."""
    out = []
    for i in range(n):
        r = random.Random(1000 + i)  # same compound in both files
        prec = 200.0 + 7.3 * i
        peaks = [(round(60 + r.random() * (prec - 80), 4), 100 + r.random() * 900) for _ in range(8)]
        j = random.Random(seed + i)
        out.append(f"Name: C{i}\nPrecursorMZ: {prec}\nPrecursor_type: [M+H]+\nIon_mode: Positive\nInChIKey: {i:014d}-ABCDEFGHIJ-N\nNum Peaks: {len(peaks)}\n"
                   + "".join(f"{m}\t{v * (1 + noise * (j.random() - 0.5)):.2f}\n" for m, v in sorted(peaks)))
    return "\n".join(out)


def test_rank1_and_determinism(tmp_path):
    lib, qs = tmp_path / "lib.msp", tmp_path / "q.msp"
    lib.write_text(msp(40, 0.0, 1))
    qs.write_text(msp(40, 0.3, 2))
    rec = V.record(lib, qs)
    s = rec["generated"]["scores"]
    assert rec["generated"]["with_truth"] == 40
    assert all(s[k]["rank1"] == 40 for k in s)
    assert rec["outcomes"] == {"entropy_rank1_not_below_cosine": True, "deterministic": True}
    assert rec["used"]["library"]["sha256"] == V.sha256(lib)


def test_a_query_without_a_compound_in_the_library_has_no_truth(tmp_path):
    lib, qs = tmp_path / "lib.msp", tmp_path / "q.msp"
    lib.write_text(msp(20, 0.0, 1))
    qs.write_text(msp(30, 0.0, 2))
    g = V.measure(lib, qs)
    assert (g["queries"], g["with_truth"]) == (30, 20)


def test_without_the_files_it_skips(tmp_path, capsys):
    assert V.main([str(tmp_path / "a"), str(tmp_path / "b")]) == 0
    assert "skipped" in capsys.readouterr().out
