"""tools/ci_metrics.py with synthetic events and a fake API: failed run, PR events, push to main with two dirty PRs, summary."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
import ci_metrics as s  # noqa: E402
import ci_metrics_file  # noqa: E402
s.ci_metrics_file = ci_metrics_file


def job(nome, concl, t0, t1, steps=()):
    return {"id": abs(hash(nome)) % 10**6, "name": nome, "status": "completed", "conclusion": concl, "created_at": t0, "started_at": "2026-10-21T10:00:30Z",
            "completed_at": t1, "steps": [{"name": n, "conclusion": c} for n, c in steps]}


JOBS = [job("pytest (3.11)", "success", "2026-10-21T10:00:00Z", "2026-10-21T10:10:30Z"),
        job("tipi (tsc)", "failure", "2026-10-21T10:00:00Z", "2026-10-21T10:01:30Z", [("tsc", "failure")]),
        job("browser", "failure", "2026-10-21T10:00:00Z", "2026-10-21T10:20:30Z", [("Smoke tests", "failure")]),
        job("rust", "timed_out", "2026-10-21T10:00:00Z", "2026-10-21T10:30:30Z")]
RUN = {"workflow_run": {"id": 7, "name": "test", "conclusion": "failure", "head_sha": "abc", "head_branch": "claude/x", "run_attempt": 2,
                        "run_started_at": "2026-10-21T10:00:00Z", "updated_at": "2026-10-21T10:30:30Z", "pull_requests": [{"number": 5}]}}


def test_run_con_job_falliti():
    rows = s.run_rows(RUN, lambda p: {"jobs": JOBS})
    assert rows[0]["tipo"] == "run" and rows[0]["pr"] == 5 and rows[0]["coda_s"] == 30 and rows[0]["jobs"]["pytest (3.11)"] == 600
    cat = {r["job"]: r["categoria"] for r in rows if r["tipo"] == "job_fallito"}
    assert cat == {"tipi (tsc)": "lint", "browser": "test", "rust": "timeout"}
    assert all(r["run_attempt"] == 2 for r in rows) and sum(r["tipo"] == "job" for r in rows) == 4


def test_pr_unit_record():
    pr = {"number": 5, "created_at": "2026-10-21T08:00:00Z", "merged_at": "2026-10-21T11:00:00Z", "closed_at": "2026-10-21T11:00:00Z",
          "merged": True, "head": {"sha": "abc"}}

    def api(p):
        if p.startswith("/commits/") and "check-runs" not in p:
            return {"commit": {"message": "Merge branch 'main' into x"}, "parents": [1, 2], "files": [{"filename": "docs/a.md"}]}
        if "/files" in p:
            return [{"filename": "docs/a.md"}, {"filename": "crates/x/src/lib.rs"}]
        if "/reviews" in p:
            return [{"submitted_at": "2026-10-21T09:00:00Z"}]
        if "/commits?" in p:
            return [{"parents": [1], "commit": {"message": "start"}},
                    {"parents": [1, 2], "commit": {"message": "Merge branch 'main' into claude/x"}},
                    {"parents": [1], "commit": {"message": "fix"}}, {"parents": [1], "commit": {"message": "cargo fmt"}}]
        return {"check_runs": [{"id": 1, "name": "a", "status": "completed", "conclusion": "success", "completed_at": "2026-10-21T10:30:00Z"},
                               {"id": 2, "name": "a", "status": "completed", "conclusion": "success", "completed_at": "2026-10-21T10:40:00Z"},
                               {"id": 3, "name": "registra", "status": "in_progress", "conclusion": None, "completed_at": None}]}
    r = s.pr_rows({"action": "closed", "pull_request": pr}, api)[0]
    assert (r["azione"], r["aperta_s"], r["merge_main"], r["verde_a_unione_s"]) == ("unita", 10800, 1, 1200)
    assert r["cause_commit"] == {"merge di main": 1, "altro": 1, "fmt/clippy": 1}
    assert (r["file_n"], r["aree"], r["solo_doc"], r["prima_review_s"]) == (2, {"docs": 1, "rust": 1}, False, 3600)
    p = s.pr_rows({"action": "synchronize", "pull_request": pr}, api)[0]
    assert (p["azione"], p["merge_main"], p["aree"]) == ("push", True, {"docs": 1})


def test_conflicts_and_dedup(tmp_path):
    dati = tmp_path / "e.jsonl"

    def api(p):
        if p == "/commits/m1":
            return {"commit": {"message": "Merge pull request #9 from x"}, "files": [{"filename": f} for f in ("a.js", "b.js", "c.js")]}
        if p.startswith("/pulls?state=open"):
            return [{"number": 1}, {"number": 2}, {"number": 3}, {"number": 4}, {"number": 5}]
        if p.startswith("/pulls/") and "/files" in p:
            return [{"filename": f} for f in (("a.js", "b.js", "z.js") if "/1/" in p else ("c.js",) if "/2/" in p else ("q.js",))]
        n = int(p.split("/")[2])
        reads[n] = reads.get(n, 0) + 1
        # 4: dirty only at the first read (false conflict); 5: dirty at both reads (real, no file in common)
        sporco = n in (1, 2, 5) or (n == 4 and reads[n] == 1)
        return {"mergeable": None if n == 3 else True, "mergeable_state": "dirty" if sporco else "clean"}
    reads: dict = {}
    pause = []
    ev = {"ref": "refs/heads/main", "after": "m1"}
    rows = s.conflict_rows(ev, str(dati), api, pause.append)
    assert [(r["pr"], r["file_comuni"], r["pr_unita"]) for r in rows] == [(1, ["a.js", "b.js"], 9), (2, ["c.js"], 9), (5, [], 9)]
    assert s.RILETTURA_S in pause
    assert s.enqueue(rows, str(dati)) == 3 and s.enqueue(rows, str(dati)) == 0
    assert s.conflict_rows(ev, str(dati), api, lambda _: None) == []
    assert s.conflict_rows({"ref": "refs/heads/x", "after": "m1"}, str(dati), api) == []


def test_summary(tmp_path, capsys):
    ev = s.run_rows(RUN, lambda p: {"jobs": JOBS})
    ev += [{"ts_utc": "2026-10-21T11:00:00Z", "tipo": "pr", "pr": 5, "azione": "unita", "aperta_s": 10800, "merge_main": 2, "verde_a_unione_s": 600},
           {"ts_utc": "2026-10-21T12:00:00Z", "tipo": "conflitto", "pr": 5, "file_comuni": ["a.js", "b.js"]}]
    out = []
    s.riepilogo(ev, out=out.append)
    txt = "\n".join(out)
    assert "tipi (tsc) [lint]" in txt and "conflicts: a.js" in txt and "Top 10 bottlenecks" in txt
    assert s.colli(ev)[0][0].startswith("job failed: rust") or s.colli(ev)[0][2] >= s.colli(ev)[-1][2]


def test_job_steps_cache_and_cancel():
    j = {"id": 11, "name": "pytest (3.11)", "status": "completed", "conclusion": "cancelled", "created_at": "2026-10-21T10:00:00Z",
         "started_at": "2026-10-21T10:00:20Z", "completed_at": "2026-10-21T10:05:20Z",
         "steps": [{"name": "Run actions/cache/restore@v4", "conclusion": "success", "started_at": "2026-10-21T10:00:30Z", "completed_at": "2026-10-21T10:01:30Z"}]}
    run = dict(RUN["workflow_run"], conclusion="cancelled", created_at="2026-10-21T10:00:00Z")
    log = ("2026-10-21T10:01:00Z Cache not found for input keys: k-abc, k-\n2026-10-21T10:04:00Z Cache saved with key: k-abc\n"
           "2026-10-21T10:04:01Z Cache Size: ~272 MB (285212672 B)\n").encode()

    def api(p):
        if "/jobs" in p:
            return {"jobs": [j]}
        if "branch=" in p:
            return {"workflow_runs": [{"name": "test", "head_sha": "new", "created_at": "2026-10-21T10:02:00Z"}, {"name": "test", "head_sha": "abc", "created_at": "2026-10-21T10:03:00Z"}]}
        return {}
    rows = s.run_obj_rows(run, api, lambda p: log if "/logs" in p else None)
    by_type = {r["tipo"]: r for r in rows}
    assert by_type["job"]["passi"] == [["Run actions/cache/restore@v4", 60, "success"]] and by_type["job"]["coda_s"] == 20
    assert (by_type["cache"]["esito_cache"], by_type["cache"]["mb"], by_type["cache"]["salvata"]) == ("miss", 272, "k-abc")
    assert (by_type["annullamento"]["minuti_runner"], by_type["annullamento"]["sostituito_da"]) == (5.0, "new")


def test_junit_per_file():
    import io
    import zipfile
    xml = ('<testsuites><testsuite><testcase classname="tests.test_a.T" name="x" time="1.5"/><testcase classname="tests.test_a" name="y" time="7"/>'
           '<testcase classname="tests.test_b" name="z" time="0.5"/></testsuite></testsuites>')
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("junit-3.11.xml", xml)
        z.writestr("tempi-test.json", '{"tests/test_a.py":{"setup":1.0,"call":7.5,"teardown":0.5,"n":2,"worker":["gw1"]},'
                   '"tests/test_b.py":{"setup":0.0,"call":0.5,"teardown":0.0,"n":1,"worker":["gw0"]}}')
    api = lambda p: {"artifacts": [{"id": 5, "name": "junit-3.11"}, {"id": 6, "name": "other"}]}  # noqa: E731
    r = s.dati_ci.junit_rows(RUN["workflow_run"], {"sha": "abc"}, api, lambda p: buf.getvalue())[0]
    assert r["file"] == {"tests/test_a.py": [2, 8.5], "tests/test_b.py": [1, 0.5]} and r["lenti"] == [["tests.test_a::y", 7.0]] and r["python"] == "3.11"
    assert r["fasi"]["tests/test_a.py"] == [1.0, 7.5, 0.5] and r["worker_occupato_s"] == {"gw0": 0.5, "gw1": 9.0}


def test_wasm_sizes(tmp_path):
    import io
    import json
    import zipfile
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
    import ci_wasm_dimensioni as w
    (tmp_path / "wasm-feat" / "hr").mkdir(parents=True)
    (tmp_path / "wasm-feat" / "hr" / "x.wasm").write_bytes(b"0" * 1000)
    m = w.misura(tmp_path)
    assert m["wasm-feat/hr/x.wasm"] == {"byte": 1000, "tetto": 1712128}
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("wasm-dimensioni.json", json.dumps(m))
    api = lambda p: {"artifacts": [{"id": 7, "name": "wasm-dimensioni"}]}  # noqa: E731
    r = s.dati_ci.wasm_rows(RUN["workflow_run"], {"sha": "abc"}, api, lambda p: buf.getvalue())[0]
    assert r["byte_totali"] == 1000 and r["margine_min_pct"] > 99


def test_recent_cursor_load_compact_export(tmp_path):
    dati = tmp_path / "eventi.jsonl"
    old = dict(RUN["workflow_run"], id=1, updated_at="2026-10-21T09:00:00Z")
    new = dict(RUN["workflow_run"], id=2, updated_at="2026-10-21T10:30:30Z")
    other = dict(RUN["workflow_run"], id=3, name="x", updated_at="2026-10-21T10:31:00Z")

    def api(p):
        if "status=completed" in p:
            return {"workflow_runs": [old, new, other]}
        if "status=queued" in p or "status=in_progress" in p:
            return {"workflow_runs": [{"id": 9}]}
        if "/runs/9/jobs" in p:
            return {"jobs": [{"status": "queued"}, {"status": "in_progress"}, {"status": "completed"}]}
        return {"jobs": JOBS}
    (tmp_path / "cursore.json").write_text('{"ultimo": "2026-10-21T10:00:00Z"}')
    rows, cur = s.recent_rows(str(dati), api, lambda p: None, s.datetime(2026, 10, 21, 11, 0, tzinfo=s.timezone.utc))
    assert cur == "2026-10-21T10:30:30Z" and {r["run_id"] for r in rows if r["tipo"] == "run"} == {2}
    car = [r for r in rows if r["tipo"] == "carico"][0]
    assert (car["job_in_coda"], car["job_in_corso"]) == (2, 2)
    assert s.enqueue(rows, str(dati)) == len(rows) and s.read_all(str(dati))[0]["v"] == 1
    ev = s.read_all(str(dati))
    comp = s.dati_ci.compact(ev, s.datetime(2026, 12, 1, tzinfo=s.timezone.utc))
    assert all("passi" not in r for r in comp) and any("passi_n" in r for r in comp)
    files = s.dati_ci.export(ev, str(tmp_path / "csv"))
    assert any(f.endswith("job.csv") for f in files) and "tipo" in (tmp_path / "csv" / "run.csv").read_text().splitlines()[0]
    out = []
    s.riepilogo(ev, out=out.append)
    assert any("Runner load" in o for o in out) and any("Jobs: queue" in o for o in out)


def test_conflict_files_merge_tree(tmp_path):
    import subprocess

    def sh(cwd, *a):
        return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *a], cwd=cwd, capture_output=True, text=True, check=True).stdout
    o, c = tmp_path / "o.git", tmp_path / "c"
    sh(tmp_path, "init", "-q", "--bare", "-b", "main", str(o))
    sh(tmp_path, "clone", "-q", str(o), str(c))
    for f in ("a.txt", "b.txt"):
        (c / f).write_text("base\n")
    sh(c, "add", "-A")
    sh(c, "commit", "-qm", "base")
    sh(c, "push", "-q", "origin", "HEAD:main")
    sh(c, "checkout", "-q", "-b", "pr")
    (c / "a.txt").write_text("pr\n")
    (c / "b.txt").write_text("pr\n")
    sh(c, "commit", "-qam", "pr")
    sh(c, "push", "-q", "origin", "HEAD:refs/pull/3/head")
    sh(c, "checkout", "-q", "main")
    (c / "a.txt").write_text("main\n")
    sh(c, "commit", "-qam", "main")
    sh(c, "push", "-q", "origin", "HEAD:main")
    assert s.dati_ci.file_conflict(3, str(c)) == ["a.txt"]


def test_typed_and_daily_timeout():
    long = job("tipi (tsc)", "cancelled", "2026-10-21T10:00:00Z", "2026-10-21T10:05:30Z", [("tsc", "cancelled")])
    corto = job("browser", "cancelled", "2026-10-21T10:00:00Z", "2026-10-21T10:01:00Z")
    assert s.category(long) == ("timeout", "tsc")
    assert s.category(long, run_annullato=True)[0] == "annullato"
    assert s.category(corto)[0] == "annullato"
    ev = [{"ts_utc": "2026-10-21T10:00:00Z", "tipo": "run", "run_attempt": 2, "coda_s": 40},
          {"ts_utc": "2026-10-21T10:01:00Z", "tipo": "job_fallito", "categoria": "timeout"},
          {"ts_utc": "2026-10-21T08:00:00Z", "tipo": "pr", "azione": "aperta"},
          {"ts_utc": "2026-10-21T11:00:00Z", "tipo": "pr", "azione": "unita", "aperta_s": 10800, "verde_a_unione_s": 600, "commit_n": 4}]
    g = s.dati_ci.daily(ev)
    assert len(g) == 1 and (g[0]["run"], g[0]["rilanci"], g[0]["coda_media_s"], g[0]["rossi_timeout"]) == (1, 1, 40, 1)
    assert (g[0]["pr_aperte_nel_giorno"], g[0]["pr_unite"], g[0]["verde_a_unione_media_s"], g[0]["commit_per_pr_media"]) == (1, 1, 600, 4.0)


def test_rust_ok_e_aggregato():
    ok = job("rust-ok", "failure", "2026-10-21T10:00:00Z", "2026-10-21T10:00:34Z", [("Every Rust job passed, or all were skipped because no Rust path changed", "failure")])
    assert s.category(ok)[0] == "aggregato"


def test_cursor_never_goes_back(tmp_path):
    old, new = tmp_path / "a.json", tmp_path / "b.json"
    old.write_text('{"ultimo": "2026-10-22T05:00:00Z"}')
    new.write_text('{"ultimo": "2026-10-22T03:00:00Z"}')
    s.advance_cursor(str(new), str(old))
    assert "05:00" in old.read_text()
    new.write_text('{"ultimo": "2026-10-22T06:00:00Z"}')
    s.advance_cursor(str(new), str(old))
    assert "06:00" in old.read_text()


def test_critical_path_by_kind():
    def pr(sha, doc, aree):
        return {"tipo": "pr", "azione": "unita", "sha": sha, "solo_doc": doc, "aree": aree}

    def run(sha, wf, duration, jobs):
        return {"tipo": "run", "workflow": wf, "main": False, "sha": sha, "run_attempt": 1, "esito": "success", "ts_utc": "2026-10-22T08:00:00Z", "durata_s": duration, "jobs": jobs}

    ev = [pr("a", True, {}), run("a", "test", 60, {"pytest": 30}),
          pr("b", False, {"rust": 3}), run("b", "test", 360, {"pytest": 360}), run("b", "rust", 480, {"rust-wasm": 470})]
    r = s.dati_ci.critical_path(ev)
    assert r["docs"]["mediana"] == 1.0 and r["rust"]["mediana"] == 8.0
    assert r["rust"]["lenti"][0] == ("rust-wasm", 7.8)
    assert s.dati_ci.critical_path(ev, "2026-10-23") == {}


def test_fill_green_only_once():
    rows = [{"tipo": "pr", "azione": "unita", "pr": 1, "sha": "a", "ts_utc": "2026-10-21T11:00:00Z", "verde_a_unione_s": None},
             {"tipo": "pr", "azione": "unita", "pr": 2, "sha": "b", "ts_utc": "2026-10-21T11:00:00Z", "verde_a_unione_s": 5},
             {"tipo": "pr", "azione": "unita", "pr": 3, "sha": "c", "ts_utc": "2026-10-21T11:00:00Z", "verde_a_unione_s": None}]
    calls = []

    def api(p):
        calls.append(p)
        if "/commits/c/" in p:
            raise OSError("gone")
        return {"check_runs": [{"id": 1, "name": "a", "status": "completed", "conclusion": "success", "completed_at": "2026-10-21T10:30:00Z"},
                               {"id": 2, "name": "registra", "status": "in_progress", "conclusion": None, "completed_at": None}]}
    assert s.fill_green(rows, api) == 2
    assert [r["verde_a_unione_s"] for r in rows] == [1800, 5, None] and rows[2]["verde_provato"]
    assert s.fill_green(rows, api) == 0 and len(calls) == 2


def _row(i, **kw):
    return {"tipo": "run", "run_id": i, "run_attempt": 1, "ts_utc": "2026-10-01T00:00:00Z", **kw}


def test_monthly_rotation_and_old_file_frozen(tmp_path, monkeypatch):
    dati = str(tmp_path / "eventi.jsonl")
    s.write_rows(dati, [_row(1), _row(2)])           # legacy single file
    ottobre = s.datetime(2026, 10, 30, tzinfo=s.timezone.utc)
    novembre = s.datetime(2026, 11, 1, tzinfo=s.timezone.utc)
    assert s.active(dati, ottobre).endswith("eventi-2026-10.jsonl")
    assert s.active(dati, novembre).endswith("eventi-2026-11.jsonl")
    monkeypatch.setattr(s.dati_ci, "adesso_dt", lambda: ottobre)
    assert s.enqueue([_row(2), _row(3)], dati) == 1      # run 2 is already in the legacy file
    assert len(s.leggi(dati)) == 2                        # legacy file untouched
    assert [r["run_id"] for r in s.read_all(dati)] == [1, 2, 3]
    monkeypatch.setattr(s.dati_ci, "adesso_dt", lambda: novembre)
    assert s.enqueue([_row(3), _row(4)], dati) == 1      # dedup across months
    assert [Path(p).name for p in s.segmenti(dati)] == ["eventi.jsonl", "eventi-2026-10.jsonl", "eventi-2026-11.jsonl"]


def test_next_part_past_the_limit(tmp_path, monkeypatch):
    dati = str(tmp_path / "eventi.jsonl")
    mese = s.datetime(2026, 10, 5, tzinfo=s.timezone.utc)
    monkeypatch.setattr(s.ci_metrics_file, "LIMIT_B", 100)
    s.write_rows(str(tmp_path / "eventi-2026-10.jsonl"), [_row(i, x="y" * 50) for i in range(5)])
    assert s.active(dati, mese).endswith("eventi-2026-10-2.jsonl")
    s.write_rows(str(tmp_path / "eventi-2026-10-2.jsonl"), [_row(9, x="y" * 200)])
    assert s.active(dati, mese).endswith("eventi-2026-10-3.jsonl")
    assert [Path(p).name for p in s.segmenti(dati)] == ["eventi-2026-10.jsonl", "eventi-2026-10-2.jsonl"]


def test_commands_skip_frozen_files(tmp_path, monkeypatch):
    dati = tmp_path / "eventi.jsonl"
    s.write_rows(str(dati), [{"tipo": "pr", "azione": "unita", "pr": 1, "sha": "a", "ts_utc": "2026-10-01T00:00:00Z"}])
    monkeypatch.setattr(s.ci_metrics_file, "LIMIT_B", 10)                # the legacy file counts as oversized
    calls = []
    monkeypatch.setattr(s, "fill_green", lambda rows, massimo=30: calls.append(1) or 0)
    monkeypatch.setattr(sys, "argv", ["ci_metrics.py", "riempi-verde", "--dati", str(dati)])
    before = dati.read_bytes()
    assert s.main() == 0 and not calls and dati.read_bytes() == before


def _pr(ts, pr, action, **kw):
    return {"ts_utc": ts, "tipo": "pr", "pr": pr, "azione": action, "sha": f"s{pr}", **kw}


def test_critical_flow_includes_site_rust_and_desktop():
    assert {"test", "rust", "sito-rust", "desktop"} <= set(s.dati_ci.WORKFLOW_CRITICI) <= set(s.dati_ci.WORKFLOW_MISURATI)
    run = dict(RUN["workflow_run"], id=4, name="sito-rust", updated_at="2026-10-21T10:30:30Z")

    def api(p):
        return {"workflow_runs": [run]} if "status=completed" in p else {"workflow_runs": []} if "status=" in p else {"jobs": JOBS}
    rows, _ = s.recent_rows("/nonexistent/eventi.jsonl", api, lambda p: None, s.datetime(2026, 10, 21, 11, 0, tzinfo=s.timezone.utc))
    assert any(r["tipo"] == "run" and r["workflow"] == "sito-rust" for r in rows)
    ev = [_pr("t", 1, "unita", sha="a", solo_doc=False, aree={}),
          {"tipo": "run", "workflow": "test", "main": False, "sha": "a", "run_attempt": 1, "esito": "success", "ts_utc": "2026-10-22T08:00:00Z", "durata_s": 300, "jobs": {"pytest": 300}},
          {"tipo": "run", "workflow": "sito-rust", "main": False, "sha": "a", "run_attempt": 1, "esito": "success", "ts_utc": "2026-10-22T08:00:00Z", "durata_s": 2100, "jobs": {"sito-e2e": 2100}}]
    r = s.dati_ci.critical_path(ev)["site/other"]
    assert r["mediana"] == 35.0 and r["lenti"][0] == ("sito-e2e", 35.0)


def test_queue_snapshot_and_summary():
    ora = "2026-11-03T12:00:00Z"
    prs = {1: ("behind", "2026-11-03T06:00:00Z", False), 2: ("dirty", "2026-11-02T12:00:00Z", False), 3: ("clean", "2026-11-03T10:00:00Z", False), 4: ("clean", "2026-11-03T11:00:00Z", True)}

    def api(p):
        if p.startswith("/pulls?"):
            return [{"number": n, "created_at": c, "draft": d, "head": {"sha": f"s{n}"}} for n, (_, c, d) in prs.items()]
        if p.startswith("/pulls/"):
            return {"mergeable": True, "mergeable_state": prs[int(p.rsplit("/", 1)[1])][0]}
        n = p.split("/")[2][1:]       # /commits/s<N>/check-runs
        return {"check_runs": [{"id": 1, "name": "test", "status": "completed", "conclusion": "success", "completed_at": f"2026-11-03T0{n}:30:00Z"}]}
    (row,) = s.giorno_ci.queue_rows(api, ora, lambda x: None)
    assert (row["aperte"], row["indietro"], row["sporche"], row["verdi_in_attesa"]) == (3, 1, 1, 2)
    assert row["piu_vecchia_aperta_s"] == 24 * 3600 and row["piu_vecchia_verde_s"] == 10 * 3600 + 1800
    out = []
    s.riepilogo([row], out=out.append)
    assert any("Queue of open PRs" in o for o in out) and any("behind 1" in o for o in out)


def test_real_reds_without_cancelled():
    def red(cat, pr):
        return {"ts_utc": "2026-11-03T10:00:00Z", "tipo": "job_fallito", "pr": pr, "categoria": cat, "job": "j", "durata_s": 5}

    def run(pr):
        return {"ts_utc": "2026-11-03T09:00:00Z", "tipo": "run", "pr": pr, "workflow": "test", "esito": "failure", "run_attempt": 1, "coda_s": 0, "jobs": {}}
    ev = [run(1), run(2), run(3), run(4), red("annullato", 1), red("aggregato", 2), red("test", 3), red("timeout", 3)]
    assert s.giorno_ci.prs_with_real_red(ev) == (1, 4)
    g = s.dati_ci.daily(ev)[0]
    assert (g["rossi_veri"], g["pr_con_rosso_vero"], g["rossi_annullato"]) == (2, 1, 1)
    out = []
    s.riepilogo(ev, out=out.append)
    txt = "\n".join(out)
    assert "PRs with a real red: 1 of 4 (25%)" in txt and "2 of 4 failed jobs left out" in txt and "[annullato]" not in txt
    assert all("annullato" not in n for n, _, _ in s.colli(ev))


def test_daily_median_max_and_open_at_end_of_day(tmp_path):
    ev = [_pr("2026-11-01T08:00:00Z", 1, "aperta"), _pr("2026-11-01T09:00:00Z", 2, "aperta"), _pr("2026-11-01T10:00:00Z", 3, "aperta"),
          _pr("2026-11-02T08:00:00Z", 1, "unita", aperta_s=3600 * 24, verde_a_unione_s=600),
          _pr("2026-11-02T09:00:00Z", 2, "unita", aperta_s=3600 * 2, verde_a_unione_s=1200),
          _pr("2026-11-02T10:00:00Z", 3, "chiusa", aperta_s=5), _pr("2026-11-02T11:00:00Z", 4, "aperta"),
          _pr("2026-11-02T12:00:00Z", 5, "unita", aperta_s=3600 * 6, verde_a_unione_s=60)]
    g = {r["giorno"]: r for r in s.dati_ci.daily(ev)}
    assert "pr_aperte" not in g["2026-11-01"]
    assert (g["2026-11-01"]["pr_aperte_nel_giorno"], g["2026-11-01"]["pr_aperte_fine_giorno"]) == (3, 3)
    d2 = g["2026-11-02"]
    assert (d2["pr_aperte_nel_giorno"], d2["pr_aperte_fine_giorno"], d2["pr_unite"]) == (1, 1, 3)
    assert (d2["aperta_mediana_s"], d2["aperta_max_s"], d2["aperta_media_s"]) == (6 * 3600, 24 * 3600, 38400)
    assert (d2["verde_a_unione_mediana_s"], d2["verde_a_unione_max_s"]) == (600, 1200)


def test_export_coo_row(tmp_path):
    ev = [_pr("2026-11-02T08:00:00Z", 1, "unita", aperta_s=3600 * 4), _pr("2026-11-02T09:00:00Z", 2, "unita", aperta_s=90 * 60),
          _pr("2026-11-02T10:00:00Z", 9, "aperta")]
    files = s.dati_ci.export(ev, str(tmp_path))
    coo = tmp_path / "giornaliero-coo.csv"
    assert str(coo) in files
    assert coo.read_text().splitlines() == ["date,merged,median_open_to_merge_min,max_open_to_merge_min,open_prs_end,chats_running,note",
                                           "2026-11-02,2,165.0,240.0,1,,"]
