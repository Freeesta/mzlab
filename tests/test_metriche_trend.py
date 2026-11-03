"""tools/metriche_trend.py with a synthetic history: flattening, the 7/30 day comparison, noise threshold, text and HTML report."""
import json
import re
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import metriche as m  # noqa: E402
import metriche_trend as t  # noqa: E402

T0 = datetime(2026, 9, 1, tzinfo=timezone.utc)


def rec(day, raw=1000, head=10.0, pytest=100, failed=0, wall=600, bench=None):
    r = {"sha": f"s{day}", "ts": (T0 + timedelta(days=day)).strftime("%Y-%m-%dT%H:%M:%SZ"),
         "wasm": {"a.wasm": {"raw": raw, "gz": raw // 3, "br": None, "head_pct": head}, "b.wasm": {"raw": 500, "gz": 200, "br": None, "head_pct": 50.0}},
         "tests": {"pytest": pytest}, "ci": {"complete": True, "wall_s": wall, "failed_runs": failed, "required": {"x": 100, "y": 300}}}
    if bench is not None:
        r["bench"] = {"first_draw_ms": bench}
    return r


def test_flatten():
    f = t.flatten(rec(0))
    assert f["wasm.total_raw"] == 1500 and f["wasm.total_gz"] == 533 and "wasm.total_br" not in f and f["wasm.min_head_pct"] == 10.0
    assert f["tests.pytest"] == 100 and f["ci.required_max_s"] == 300 and f["ci.wall_s"] == 600
    assert "ci.wall_s" not in t.flatten({**rec(0), "ci": {"complete": False, "wall_s": 5}})


def test_verdicts_and_noise():
    assert t.verdict("wasm.total_raw", 1000, 900) == "better"       # smaller is better
    assert t.verdict("wasm.total_raw", 1000, 1100) == "worse"
    assert t.verdict("wasm.total_raw", 1000, 1020) == "flat"        # 2 % is noise
    assert t.verdict("tests.pytest", 100, 120) == "better" and t.verdict("tests.pytest", 100, 80) == "worse"
    assert t.verdict("wasm.min_head_pct", 10.0, 9.8) == "flat" and t.verdict("wasm.min_head_pct", 10.0, 8.0) == "worse"
    assert t.verdict("ci.failed_runs", 0, 1) == "worse" and t.verdict("ci.failed_runs", 2, 0) == "better"
    assert t.verdict("bench.first_draw_ms", 800, 600) == "better" and t.verdict("bench.speed_per_s", 5, 9) == "better"
    assert t.verdict("manual.something_odd", 1, 2) == "tracked" and t.verdict("tests.pytest", None, 1) == "n/a"


def test_analyse_7_and_30_days():
    commits = [rec(d, raw=1000 + d * 10, head=20 - d * 0.1, pytest=100 + d) for d in range(0, 41)]
    rows = {r["name"]: r for r in t.analyse(commits)}
    r = rows["wasm.total_raw"]
    assert r["last"] == 1900 and r["d7"] == 70 and r["d30"] == 300 and r["v7"] == "worse" and r["v30"] == "worse"
    assert rows["tests.pytest"]["v7"] == "better" and rows["ci.wall_s"]["v7"] == "flat"
    assert rows["wasm.min_head_pct"]["d7"] < 0
    short = {r["name"]: r for r in t.analyse(commits[-5:])}
    assert short["wasm.total_raw"]["v7"] == "n/a" and short["wasm.total_raw"]["d30"] is None


def test_reports():
    commits = [rec(d, raw=1000 + d * 10, bench=800 - d) for d in range(0, 41)]
    rows = t.analyse(commits)
    txt = t.text_report(rows)
    assert re.search(r"7 days ago, WORSE: .*wasm\.total_raw", txt) and "BETTER:" in txt and "bench.first_draw_ms" in txt
    page = t.html_report(rows, "now")
    assert page.startswith("<!doctype html>") and "<polyline" in page and "http" not in page.replace("<!doctype html>", "") and "<script" not in page
    assert t.text_report([]) == "No metrics recorded yet."


def test_cli_report_from_files(tmp_path, capsys):
    d = tmp_path / "d"
    d.mkdir()
    with open(d / "metriche-2026-09.jsonl", "w") as f:
        for day in (0, 10, 20, 30, 40):
            f.write(json.dumps({"v": 1, "part": "site", "recorded": f"2026-10-0{1 + day // 20}T00:00:00Z", **{k: v for k, v in rec(day).items() if k != "ci"}}) + "\n")
    out = tmp_path / "r.html"
    assert m.main(["--rapporto", "--dati", str(d), "--html", str(out)]) == 0
    assert "tests.pytest" in capsys.readouterr().out and out.read_text().startswith("<!doctype html>")
