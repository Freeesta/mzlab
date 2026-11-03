"""tools/metriche.py and tools/metriche_ci.py with synthetic files and a fake GitHub API."""
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import metriche as m  # noqa: E402
import metriche_ci as mc  # noqa: E402


def test_comprimi_raw_and_gz():
    c = m.comprimi(b"abc" * 1000)
    assert c["raw"] == 3000 and 0 < c["gz"] < 100 and c["br"] in (None, c["br"])


def test_misura_wasm_headroom(tmp_path):
    (tmp_path / "wasm-feat" / "hr").mkdir(parents=True)
    (tmp_path / "wasm-feat" / "hr" / "x.wasm").write_bytes(b"\0" * 1000)
    (tmp_path / "wasm-feat" / "zz").mkdir()
    (tmp_path / "wasm-feat" / "zz" / "y.wasm").write_bytes(b"\0" * 10)
    out = m.misura_wasm(tmp_path)
    hr = out["wasm-feat/hr/x.wasm"]
    assert hr["raw"] == 1000 and hr["cap"] and hr["head_b"] == hr["cap"] - 1000 and 99 < hr["head_pct"] < 100
    assert out["wasm-feat/zz/y.wasm"]["cap"] is None and out["wasm-feat/zz/y.wasm"]["head_b"] is None


def test_avvio_and_test_counts_of_this_repo():
    a = m.misura_avvio()
    assert a["raw"] <= a["cap"] and a["head_b"] == a["cap"] - a["raw"] and a["files"] > 10
    t = m.conta_test()
    assert t["pytest"] > 100 and t["cargo"] > 100 and t["e2e"] > 10


def test_conta_test_synthetic(tmp_path):
    (tmp_path / "tests").mkdir()
    (tmp_path / "tests" / "test_a.py").write_text("def test_one():\n    pass\n\nclass T:\n    def test_two(self):\n        pass\n\ndef helper(): pass\n")
    (tmp_path / "crates").mkdir()
    (tmp_path / "crates" / "a.rs").write_text("#[test]\nfn a() {}\n#[cfg(test)]\n#[test]\nfn b() {}\n")
    (tmp_path / "tests_e2e").mkdir()
    (tmp_path / "tests_e2e" / "e2e_x.py").write_text("")
    assert m.conta_test(tmp_path) == {"pytest": 2, "cargo": 2, "e2e": 1}


def test_bench_input(tmp_path):
    f = tmp_path / "b.json"
    f.write_text(json.dumps({"a": 1}))
    assert m.carica_bench(str(f), ["b=2.5"]) == {"a": 1.0, "b": 2.5}
    with pytest.raises(SystemExit):
        m.carica_bench(None, ["novalue"])


def test_accoda_dedup_and_merge_by_sha(tmp_path):
    d = str(tmp_path)
    r1 = m.riga("site", "aaa", "2026-10-01T00:00:00Z", tests={"pytest": 1})
    r2 = m.riga("manual", "aaa", "2026-10-01T00:00:00Z", manual={"hrc": 1.0})
    r3 = m.riga("manual", "aaa", "2026-10-01T00:00:00Z", manual={"lr": 2.0})
    r4 = m.riga("site", "bbb", "2026-10-02T00:00:00Z", tests={"pytest": 2})
    assert m.accoda([r1, r2], d) == 2 and m.accoda([r1, r2], d) == 0   # same content again: nothing
    assert m.accoda([r3, r4], d) == 2
    com = m.leggi_commit(d)
    assert [c["sha"] for c in com] == ["aaa", "bbb"]
    assert com[0]["manual"] == {"hrc": 1.0, "lr": 2.0} and com[0]["tests"] == {"pytest": 1}
    assert len(list(tmp_path.glob("metriche-*.jsonl"))) == 1


def job(name, concl, t0, t1, status="completed"):
    return {"name": name, "status": status, "conclusion": concl, "started_at": t0, "completed_at": t1}


def test_ci_facts():
    runs = {"workflow_runs": [
        {"id": 1, "name": "test", "status": "completed", "conclusion": "success", "run_attempt": 2, "run_started_at": "2026-10-30T10:00:00Z", "updated_at": "2026-10-30T10:12:00Z"},
        {"id": 2, "name": "pages", "status": "completed", "conclusion": "failure", "run_attempt": 1, "run_started_at": "2026-10-30T10:00:00Z", "updated_at": "2026-10-30T10:20:00Z"},
        {"id": 3, "name": "other", "status": "completed", "conclusion": "failure", "run_attempt": 1}]}
    jobs = {1: {"jobs": [job("javascript", "success", "2026-10-30T10:00:10Z", "2026-10-30T10:01:10Z"), job("x", "skipped", None, None)]},
            2: {"jobs": [job("build", "failure", "2026-10-30T10:00:00Z", "2026-10-30T10:05:00Z")]}}

    def api(path):
        if path.startswith("/actions/runs?"):
            return runs
        if path.startswith("/actions/runs/"):
            return jobs[int(path.split("/")[3].split("?")[0])]
        if path == "/pulls/7":
            return {"created_at": "2026-10-30T08:00:00Z", "merged_at": "2026-10-30T09:00:00Z"}
        return [{"created_at": "2026-10-29T00:00:00Z", "merged_at": "2026-10-30T09:00:00Z"},
                {"created_at": "2026-10-20T00:00:00Z", "merged_at": "2026-10-27T00:00:00Z"},
                {"created_at": "2026-10-01T00:00:00Z", "merged_at": "2026-10-02T00:00:00Z"}, {"created_at": "2026-10-01T00:00:00Z", "merged_at": None}]
    ci = mc.facts("abc", "Fix thing (#7)", api, datetime(2026, 10, 30, 12, tzinfo=timezone.utc))
    assert ci["workflows"]["test"] == {"wall_s": 720, "attempts": 2, "conclusion": "success"} and "other" not in ci["workflows"]
    assert ci["required"] == {"javascript": 60} and "x" not in ci["jobs"]
    assert ci["failed_runs"] == 1 and ci["flaky_runs"] == 1 and ci["wall_s"] == 1200 and ci["complete"] is True
    assert ci["pr"] == 7 and ci["pr_open_to_merge_s"] == 3600
    assert ci["merged_24h"] == 1 and ci["merged_window"] == 2 and ci["open_to_merge_median_s"] == int((33 * 3600 + 7 * 86400) / 2)


def test_pr_number_and_incomplete_run():
    assert mc.pr_number("Title (#12)") == 12 and mc.pr_number("no number") is None
    out = mc.runs_of_commit("s", lambda p: {"workflow_runs": [{"id": 1, "name": "test", "status": "in_progress", "conclusion": None}]} if "head_sha" in p else {"jobs": []})
    assert out["complete"] is False and out["workflows"]["test"]["wall_s"] is None
