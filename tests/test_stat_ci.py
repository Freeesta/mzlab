"""tools/stat_ci.py with synthetic events and a fake API: failed run, PR events, push to main with two dirty PRs, summary."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import stat_ci as s  # noqa: E402


def job(nome, concl, t0, t1, steps=()):
    return {"name": nome, "status": "completed", "conclusion": concl, "created_at": t0, "started_at": "2026-10-21T10:00:30Z",
            "completed_at": t1, "steps": [{"name": n, "conclusion": c} for n, c in steps]}


JOBS = [job("pytest (3.11)", "success", "2026-10-21T10:00:00Z", "2026-10-21T10:10:30Z"),
        job("tipi (tsc)", "failure", "2026-10-21T10:00:00Z", "2026-10-21T10:01:30Z", [("tsc", "failure")]),
        job("browser", "failure", "2026-10-21T10:00:00Z", "2026-10-21T10:20:30Z", [("Smoke tests", "failure")]),
        job("rust", "timed_out", "2026-10-21T10:00:00Z", "2026-10-21T10:30:30Z")]
RUN = {"workflow_run": {"id": 7, "name": "test", "conclusion": "failure", "head_sha": "abc", "head_branch": "claude/x", "run_attempt": 2,
                        "run_started_at": "2026-10-21T10:00:00Z", "updated_at": "2026-10-21T10:30:30Z", "pull_requests": [{"number": 5}]}}


def test_run_con_job_falliti():
    righe = s.righe_run(RUN, lambda p: {"jobs": JOBS})
    assert righe[0]["tipo"] == "run" and righe[0]["pr"] == 5 and righe[0]["coda_s"] == 30 and righe[0]["jobs"]["pytest (3.11)"] == 600
    cat = {r["job"]: r["categoria"] for r in righe[1:]}
    assert cat == {"tipi (tsc)": "lint", "browser": "test", "rust": "timeout"}
    assert all(r["run_attempt"] == 2 for r in righe)


def test_pr_unita():
    pr = {"number": 5, "created_at": "2026-10-21T08:00:00Z", "merged_at": "2026-10-21T11:00:00Z", "closed_at": "2026-10-21T11:00:00Z",
          "merged": True, "head": {"sha": "abc"}}

    def api(p):
        if "/commits?" in p:
            return [{"parents": [1], "commit": {"message": "start"}},
                    {"parents": [1, 2], "commit": {"message": "Merge branch 'main' into claude/x"}},
                    {"parents": [1], "commit": {"message": "fix"}}, {"parents": [1], "commit": {"message": "cargo fmt"}}]
        return {"check_runs": [{"id": 1, "name": "a", "status": "completed", "conclusion": "success", "completed_at": "2026-10-21T10:30:00Z"},
                               {"id": 2, "name": "a", "status": "completed", "conclusion": "success", "completed_at": "2026-10-21T10:40:00Z"}]}
    r = s.righe_pr({"action": "closed", "pull_request": pr}, api)[0]
    assert (r["azione"], r["aperta_s"], r["merge_main"], r["verde_a_unione_s"]) == ("unita", 10800, 1, 1200)
    assert r["cause_commit"] == {"merge di main": 1, "altro": 1, "fmt/clippy": 1}
    assert s.righe_pr({"action": "synchronize", "pull_request": pr}, api)[0]["azione"] == "push"


def test_conflitti_e_dedup(tmp_path):
    dati = tmp_path / "e.jsonl"

    def api(p):
        if p == "/commits/m1":
            return {"commit": {"message": "Merge pull request #9 from x"}, "files": [{"filename": f} for f in ("a.js", "b.js", "c.js")]}
        if p.startswith("/pulls?state=open"):
            return [{"number": 1}, {"number": 2}, {"number": 3}, {"number": 4}, {"number": 5}]
        if p.startswith("/pulls/") and "/files" in p:
            return [{"filename": f} for f in (("a.js", "b.js", "z.js") if "/1/" in p else ("c.js",) if "/2/" in p else ("q.js",))]
        n = int(p.split("/")[2])
        letture[n] = letture.get(n, 0) + 1
        # 4: dirty only at the first read (false conflict); 5: dirty at both reads (real, no file in common)
        sporco = n in (1, 2, 5) or (n == 4 and letture[n] == 1)
        return {"mergeable": None if n == 3 else True, "mergeable_state": "dirty" if sporco else "clean"}
    letture: dict = {}
    pause = []
    ev = {"ref": "refs/heads/main", "after": "m1"}
    righe = s.righe_conflitti(ev, str(dati), api, pause.append)
    assert [(r["pr"], r["file_comuni"], r["pr_unita"]) for r in righe] == [(1, ["a.js", "b.js"], 9), (2, ["c.js"], 9), (5, [], 9)]
    assert s.RILETTURA_S in pause
    assert s.accoda(righe, str(dati)) == 3 and s.accoda(righe, str(dati)) == 0
    assert s.righe_conflitti(ev, str(dati), api, lambda _: None) == []
    assert s.righe_conflitti({"ref": "refs/heads/x", "after": "m1"}, str(dati), api) == []


def test_riepilogo(tmp_path, capsys):
    ev = s.righe_run(RUN, lambda p: {"jobs": JOBS})
    ev += [{"ts_utc": "2026-10-21T11:00:00Z", "tipo": "pr", "pr": 5, "azione": "unita", "aperta_s": 10800, "merge_main": 2, "verde_a_unione_s": 600},
           {"ts_utc": "2026-10-21T12:00:00Z", "tipo": "conflitto", "pr": 5, "file_comuni": ["a.js", "b.js"]}]
    out = []
    s.riepilogo(ev, out=out.append)
    txt = "\n".join(out)
    assert "tipi (tsc) [lint]" in txt and "conflicts: a.js" in txt and "Top 10 bottlenecks" in txt
    assert s.colli(ev)[0][0].startswith("job failed: rust") or s.colli(ev)[0][2] >= s.colli(ev)[-1][2]
