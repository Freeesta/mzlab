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
    righe = s.righe_run(RUN, lambda p: {"jobs": JOBS})
    assert righe[0]["tipo"] == "run" and righe[0]["pr"] == 5 and righe[0]["coda_s"] == 30 and righe[0]["jobs"]["pytest (3.11)"] == 600
    cat = {r["job"]: r["categoria"] for r in righe if r["tipo"] == "job_fallito"}
    assert cat == {"tipi (tsc)": "lint", "browser": "test", "rust": "timeout"}
    assert all(r["run_attempt"] == 2 for r in righe) and sum(r["tipo"] == "job" for r in righe) == 4


def test_pr_unita():
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
    r = s.righe_pr({"action": "closed", "pull_request": pr}, api)[0]
    assert (r["azione"], r["aperta_s"], r["merge_main"], r["verde_a_unione_s"]) == ("unita", 10800, 1, 1200)
    assert r["cause_commit"] == {"merge di main": 1, "altro": 1, "fmt/clippy": 1}
    assert (r["file_n"], r["aree"], r["solo_doc"], r["prima_review_s"]) == (2, {"docs": 1, "rust": 1}, False, 3600)
    p = s.righe_pr({"action": "synchronize", "pull_request": pr}, api)[0]
    assert (p["azione"], p["merge_main"], p["aree"]) == ("push", True, {"docs": 1})


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


def test_job_passi_cache_e_annullo():
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
    righe = s.righe_run_obj(run, api, lambda p: log if "/logs" in p else None)
    por_tipo = {r["tipo"]: r for r in righe}
    assert por_tipo["job"]["passi"] == [["Run actions/cache/restore@v4", 60, "success"]] and por_tipo["job"]["coda_s"] == 20
    assert (por_tipo["cache"]["esito_cache"], por_tipo["cache"]["mb"], por_tipo["cache"]["salvata"]) == ("miss", 272, "k-abc")
    assert (por_tipo["annullamento"]["minuti_runner"], por_tipo["annullamento"]["sostituito_da"]) == (5.0, "new")


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
    r = s.dati_ci.righe_junit(RUN["workflow_run"], {"sha": "abc"}, api, lambda p: buf.getvalue())[0]
    assert r["file"] == {"tests/test_a.py": [2, 8.5], "tests/test_b.py": [1, 0.5]} and r["lenti"] == [["tests.test_a::y", 7.0]] and r["python"] == "3.11"
    assert r["fasi"]["tests/test_a.py"] == [1.0, 7.5, 0.5] and r["worker_occupato_s"] == {"gw0": 0.5, "gw1": 9.0}


def test_wasm_dimensioni(tmp_path):
    import io
    import json
    import zipfile
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))
    import ci_wasm_dimensioni as w
    (tmp_path / "wasm-feat" / "hr").mkdir(parents=True)
    (tmp_path / "wasm-feat" / "hr" / "x.wasm").write_bytes(b"0" * 1000)
    m = w.misura(tmp_path)
    assert m["wasm-feat/hr/x.wasm"] == {"byte": 1000, "tetto": 1605632}
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("wasm-dimensioni.json", json.dumps(m))
    api = lambda p: {"artifacts": [{"id": 7, "name": "wasm-dimensioni"}]}  # noqa: E731
    r = s.dati_ci.righe_wasm(RUN["workflow_run"], {"sha": "abc"}, api, lambda p: buf.getvalue())[0]
    assert r["byte_totali"] == 1000 and r["margine_min_pct"] > 99


def test_recenti_cursore_carico_compatta_esporta(tmp_path):
    dati = tmp_path / "eventi.jsonl"
    vecchio = dict(RUN["workflow_run"], id=1, updated_at="2026-10-21T09:00:00Z")
    nuovo = dict(RUN["workflow_run"], id=2, updated_at="2026-10-21T10:30:30Z")
    altro = dict(RUN["workflow_run"], id=3, name="x", updated_at="2026-10-21T10:31:00Z")

    def api(p):
        if "status=completed" in p:
            return {"workflow_runs": [vecchio, nuovo, altro]}
        if "status=queued" in p or "status=in_progress" in p:
            return {"workflow_runs": [{"id": 9}]}
        if "/runs/9/jobs" in p:
            return {"jobs": [{"status": "queued"}, {"status": "in_progress"}, {"status": "completed"}]}
        return {"jobs": JOBS}
    (tmp_path / "cursore.json").write_text('{"ultimo": "2026-10-21T10:00:00Z"}')
    righe, cur = s.righe_recenti(str(dati), api, lambda p: None, s.datetime(2026, 10, 21, 11, 0, tzinfo=s.timezone.utc))
    assert cur == "2026-10-21T10:30:30Z" and {r["run_id"] for r in righe if r["tipo"] == "run"} == {2}
    car = [r for r in righe if r["tipo"] == "carico"][0]
    assert (car["job_in_coda"], car["job_in_corso"]) == (2, 2)
    assert s.accoda(righe, str(dati)) == len(righe) and s.leggi_tutti(str(dati))[0]["v"] == 1
    ev = s.leggi_tutti(str(dati))
    comp = s.dati_ci.compatta(ev, s.datetime(2026, 12, 1, tzinfo=s.timezone.utc))
    assert all("passi" not in r for r in comp) and any("passi_n" in r for r in comp)
    files = s.dati_ci.esporta(ev, str(tmp_path / "csv"))
    assert any(f.endswith("job.csv") for f in files) and "tipo" in (tmp_path / "csv" / "run.csv").read_text().splitlines()[0]
    out = []
    s.riepilogo(ev, out=out.append)
    assert any("Runner load" in o for o in out) and any("Jobs: queue" in o for o in out)


def test_file_conflitto_merge_tree(tmp_path):
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
    assert s.dati_ci.file_conflitto(3, str(c)) == ["a.txt"]


def test_timeout_tipizzato_e_giornaliero():
    lungo = job("tipi (tsc)", "cancelled", "2026-10-21T10:00:00Z", "2026-10-21T10:05:30Z", [("tsc", "cancelled")])
    corto = job("browser", "cancelled", "2026-10-21T10:00:00Z", "2026-10-21T10:01:00Z")
    assert s.categoria(lungo) == ("timeout", "tsc")
    assert s.categoria(lungo, run_annullato=True)[0] == "annullato"
    assert s.categoria(corto)[0] == "annullato"
    ev = [{"ts_utc": "2026-10-21T10:00:00Z", "tipo": "run", "run_attempt": 2, "coda_s": 40},
          {"ts_utc": "2026-10-21T10:01:00Z", "tipo": "job_fallito", "categoria": "timeout"},
          {"ts_utc": "2026-10-21T08:00:00Z", "tipo": "pr", "azione": "aperta"},
          {"ts_utc": "2026-10-21T11:00:00Z", "tipo": "pr", "azione": "unita", "aperta_s": 10800, "verde_a_unione_s": 600, "commit_n": 4}]
    g = s.dati_ci.giornaliero(ev)
    assert len(g) == 1 and (g[0]["run"], g[0]["rilanci"], g[0]["coda_media_s"], g[0]["rossi_timeout"]) == (1, 1, 40, 1)
    assert (g[0]["pr_aperte"], g[0]["pr_unite"], g[0]["verde_a_unione_media_s"], g[0]["commit_per_pr_media"]) == (1, 1, 600, 4.0)


def test_rust_ok_e_aggregato():
    ok = job("rust-ok", "failure", "2026-10-21T10:00:00Z", "2026-10-21T10:00:34Z", [("Every Rust job passed, or all were skipped because no Rust path changed", "failure")])
    assert s.categoria(ok)[0] == "aggregato"


def test_cursore_non_arretra(tmp_path):
    vecchio, nuovo = tmp_path / "a.json", tmp_path / "b.json"
    vecchio.write_text('{"ultimo": "2026-10-22T05:00:00Z"}')
    nuovo.write_text('{"ultimo": "2026-10-22T03:00:00Z"}')
    s.avanza_cursore(str(nuovo), str(vecchio))
    assert "05:00" in vecchio.read_text()
    nuovo.write_text('{"ultimo": "2026-10-22T06:00:00Z"}')
    s.avanza_cursore(str(nuovo), str(vecchio))
    assert "06:00" in vecchio.read_text()


def test_percorso_critico_per_tipo():
    def pr(sha, doc, aree):
        return {"tipo": "pr", "azione": "unita", "sha": sha, "solo_doc": doc, "aree": aree}

    def run(sha, wf, durata, jobs):
        return {"tipo": "run", "workflow": wf, "main": False, "sha": sha, "run_attempt": 1, "esito": "success", "ts_utc": "2026-10-22T08:00:00Z", "durata_s": durata, "jobs": jobs}

    ev = [pr("a", True, {}), run("a", "test", 60, {"pytest": 30}),
          pr("b", False, {"rust": 3}), run("b", "test", 360, {"pytest": 360}), run("b", "rust", 480, {"rust-wasm": 470})]
    r = s.dati_ci.percorso_critico(ev)
    assert r["docs"]["mediana"] == 1.0 and r["rust"]["mediana"] == 8.0
    assert r["rust"]["lenti"][0] == ("rust-wasm", 7.8)
    assert s.dati_ci.percorso_critico(ev, "2026-10-23") == {}


def test_riempi_verde_una_volta_sola():
    righe = [{"tipo": "pr", "azione": "unita", "pr": 1, "sha": "a", "ts_utc": "2026-10-21T11:00:00Z", "verde_a_unione_s": None},
             {"tipo": "pr", "azione": "unita", "pr": 2, "sha": "b", "ts_utc": "2026-10-21T11:00:00Z", "verde_a_unione_s": 5},
             {"tipo": "pr", "azione": "unita", "pr": 3, "sha": "c", "ts_utc": "2026-10-21T11:00:00Z", "verde_a_unione_s": None}]
    chiamate = []

    def api(p):
        chiamate.append(p)
        if "/commits/c/" in p:
            raise OSError("gone")
        return {"check_runs": [{"id": 1, "name": "a", "status": "completed", "conclusion": "success", "completed_at": "2026-10-21T10:30:00Z"},
                               {"id": 2, "name": "registra", "status": "in_progress", "conclusion": None, "completed_at": None}]}
    assert s.riempi_verde(righe, api) == 2
    assert [r["verde_a_unione_s"] for r in righe] == [1800, 5, None] and righe[2]["verde_provato"]
    assert s.riempi_verde(righe, api) == 0 and len(chiamate) == 2


def _riga(i, **kw):
    return {"tipo": "run", "run_id": i, "run_attempt": 1, "ts_utc": "2026-10-01T00:00:00Z", **kw}


def test_rotazione_mensile_e_file_vecchio_congelato(tmp_path, monkeypatch):
    dati = str(tmp_path / "eventi.jsonl")
    s.scrivi_righe(dati, [_riga(1), _riga(2)])           # legacy single file
    ottobre = s.datetime(2026, 10, 30, tzinfo=s.timezone.utc)
    novembre = s.datetime(2026, 11, 1, tzinfo=s.timezone.utc)
    assert s.attivo(dati, ottobre).endswith("eventi-2026-10.jsonl")
    assert s.attivo(dati, novembre).endswith("eventi-2026-11.jsonl")
    monkeypatch.setattr(s.dati_ci, "adesso_dt", lambda: ottobre)
    assert s.accoda([_riga(2), _riga(3)], dati) == 1      # run 2 is already in the legacy file
    assert len(s.leggi(dati)) == 2                        # legacy file untouched
    assert [r["run_id"] for r in s.leggi_tutti(dati)] == [1, 2, 3]
    monkeypatch.setattr(s.dati_ci, "adesso_dt", lambda: novembre)
    assert s.accoda([_riga(3), _riga(4)], dati) == 1      # dedup across months
    assert [Path(p).name for p in s.segmenti(dati)] == ["eventi.jsonl", "eventi-2026-10.jsonl", "eventi-2026-11.jsonl"]


def test_parte_successiva_oltre_il_limite(tmp_path, monkeypatch):
    dati = str(tmp_path / "eventi.jsonl")
    mese = s.datetime(2026, 10, 5, tzinfo=s.timezone.utc)
    monkeypatch.setattr(s.ci_metrics_file, "LIMITE_B", 100)
    s.scrivi_righe(str(tmp_path / "eventi-2026-10.jsonl"), [_riga(i, x="y" * 50) for i in range(5)])
    assert s.attivo(dati, mese).endswith("eventi-2026-10-2.jsonl")
    s.scrivi_righe(str(tmp_path / "eventi-2026-10-2.jsonl"), [_riga(9, x="y" * 200)])
    assert s.attivo(dati, mese).endswith("eventi-2026-10-3.jsonl")
    assert [Path(p).name for p in s.segmenti(dati)] == ["eventi-2026-10.jsonl", "eventi-2026-10-2.jsonl"]


def test_comandi_saltano_i_file_congelati(tmp_path, monkeypatch):
    dati = tmp_path / "eventi.jsonl"
    s.scrivi_righe(str(dati), [{"tipo": "pr", "azione": "unita", "pr": 1, "sha": "a", "ts_utc": "2026-10-01T00:00:00Z"}])
    monkeypatch.setattr(s.ci_metrics_file, "LIMITE_B", 10)                # the legacy file counts as oversized
    chiamate = []
    monkeypatch.setattr(s, "riempi_verde", lambda righe, massimo=30: chiamate.append(1) or 0)
    monkeypatch.setattr(sys, "argv", ["ci_metrics.py", "riempi-verde", "--dati", str(dati)])
    before = dati.read_bytes()
    assert s.main() == 0 and not chiamate and dati.read_bytes() == before
