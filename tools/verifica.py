"""One command that checks the whole program and prints a SHORT summary (made to save the reader's time and tokens).

    python3 tools/verifica.py                 # everything: JS syntax, pytest, every e2e that can run here
    python3 tools/verifica.py --rapida        # JS syntax + pytest + the e2e smoke test (e2e3): ~2 minutes
    python3 tools/verifica.py --solo e2e6,e2e18
    python3 tools/verifica.py --ultimi-falliti   # only the e2e that were FAIL in the previous run (.verifica/ultimo.md)
    python3 tools/verifica.py --setup         # first install what is missing (pytest, playwright + chromium), then check

Data, in this order: MZLAB_MZML (or legacy QQQ_MZML); the PRIVATE data repository (Freeesta/mzlab-dati: dati/3200 qtrap/mzML + dam/, real lab files; found by tools/dati_percorsi.py
next to this repository, in a parent folder, in the home or in /workspace, or with MZLAB_DATI=<path> or QQQ_DATI=<path>);
Federico's Mac folders (../Data/mzML); otherwise synthetic look-alikes written by tools/dati_sintetici.py into
.verifica/dati_sintetici/ (then the summary says "sintetici": a pass on synthetic data is weaker than on real data).
Some e2e only make sense with the real data or with a .dam method (MZLAB_DAM or QQQ_DAM): they are reported as SKIP, with why.

Output: one line per check on stdout (OK / FAIL / SKIP, seconds, first error lines), the same summary in
.verifica/ultimo.md, and the full output of every check in .verifica/log/<name>.log. Read the logs only for a FAIL.
Exit code 0 only if nothing failed. `.verifica/` is not committed (.gitignore).
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dati_percorsi import percorso  # the one resolver of the private data repository

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".verifica"
LOG = OUT / "log"
E2E = ROOT / "tests_e2e"

# What each e2e needs besides a browser. "veri" = the asserts depend on the real lab data (skipped with synthetic data);
# "dam" = needs a .dam method file (QQQ_DAM); "sito" = builds the static site (no network); "wasm" = needs a .wasm the CI builds in its own jobs (never skipped here, left out of `e2e-cambiati`);
# Keep this table up to date when you add an e2e (default: no needs).
NEEDS: dict[str, set[str]] = {
    "e2e9": {"dam"}, "e2e10": {"dam"}, "e2e15": {"dam"}, "e2e_dam_incoerente": {"dam"},
    "e2e_studenti": {"veri"},          # needs the whole series B (7 times, 6 standards): the synthetic files are only a few
    "e2e13": {"sito"}, "e2e_avvio": {"sito"}, "e2e_avvio_shell": {"sito"}, "e2e_budget_avvio": {"sito"}, "e2e_avvio_lingua": {"sito"}, "e2e_mzidx": {"sito"}, "e2e_indice_parallelo": {"sito"}, "e2e_rust": {"sito"}, "e2e_lr": {"sito"}, "e2e_wiff": {"sito"}, "e2e_nat_sul_posto": {"sito"}, "e2e_compressione": {"sito"}, "e2e_numpress_modulo": {"sito"}, "e2e_agilent": {"sito"}, "e2e_metodo": {"sito"}, "e2e_picchi_qualita": {"sito"}, "e2e_hr_rust": {"sito"}, "e2e_hr_mappa": {"sito"}, "e2e_hr_albero": {"sito"}, "e2e_libreria_wasm": {"sito"}, "e2e_libreria_cartella": {"sito"}, "e2e_libreria_scarica": {"sito"},  "e2e_tpmine1": {"sito"}, "e2e_tpmine_nativo": {"sito"}, "e2e_tpmine_diretto_nativo": {"sito"}, "e2e_progresso": {"sito"}, "e2e_carico_stabile": {"sito"}, "e2e_tpmine_lr_reload": {"sito"}, "e2e_tpmine_ms2_lr": {"sito"}, "e2e_pipeline": {"sito"}, "e2e_lettori": {"sito"}, "e2e_lettori_modulo": {"sito"}, "e2e_modulo_irraggiungibile": {"sito"}, "e2e_impronta_opfs": {"sito"}, "e2e_gz_flusso": {"sito"}, "e2e_shimadzu": {"sito"}, "e2e_waters": {"sito"}, "e2e_thermo_modulo": {"sito"}, "e2e_tpmine_diretta": {"sito"}, "e2e_clic_flusso": {"sito"}, "e2e_tpmine2": {"sito"}, "e2e_tpmine_albero": {"sito"}, "e2e_tpmine_rtalign": {"sito"}, "e2e_tpmine_ms2": {"sito"}, "e2e_untarget": {"sito"}, "e2e_identita": {"sito"},
    "e2e_flow_web": {"sito"}, "e2e_flow_web_drift": {"sito"}, "e2e_flow_web_estimate": {"sito"}, "e2e_flow_web_export": {"sito"}, "e2e_flow_web_group": {"sito"}, "e2e_flow_web_plausible": {"sito"}, "e2e_flow_web_ident": {"sito"}, "e2e_flow_web_ident2": {"sito"}, "e2e_flow_web_modelli": {"sito"}, "e2e_flow_web_prov": {"sito"}, "e2e_flow_web_stat": {"sito"}, "e2e_flow_web_struct": {"sito"}, "e2e_flow_web_moiety": {"sito"}, "e2e_flow_web_tabella": {"sito"}, "e2e_flow_centroid_web": {"sito"}, "e2e_flow_motore": {"sito"}, "e2e_lab": {"sito"}, "e2e_lab_vuoto": {"sito"}, "e2e_normative_scarica": {"sito"},      # the Flow tests of «This browser» build every mzFinder .wasm when site/static has none (lib.build_feat): they run where the CI builds it
    "e2e_tpmine_progresso": {"sito"}, "e2e_pannelli_stabili": {"sito"}, "e2e_memoria_sonda": {"sito"},
    "e2e_raw_riaggiunto": {"sito"}, "e2e_agilent_riaggiunto": {"sito"},
    "e2e_avvio_lento": {"sito"}, "e2e_flow_transitions": {"sito"}, "e2e_flow_neg_lr": {"wasm"}, "e2e_llm_consenso": {"wasm"}, "e2e6": {"wasm"}, "e2e_decimali": {"wasm"}, "e2e_disegno_chimica": {"wasm"}, "e2e_menu_info": {"wasm"}, "e2e_menu_link": {"wasm"},  # flow_transitions builds the .wasm of mzFinder (lib.py build_feat); wasm: mzFinder built with cargo (over 600 s on a cold runner), the llm module
    # the drawing tests (Ketcher + the drawing engine .wasm): green in the nightly, which has the CI-built .wasm, red in `e2e-cambiati`, which builds the site alone
    "e2e_disegno_analisi": {"wasm"}, "e2e_disegno_cip": {"wasm"}, "e2e_disegno_esporta": {"wasm"}, "e2e_disegno_verifica": {"wasm"}, "e2e_meccanismo": {"wasm"}, "e2e_parziali": {"wasm"}, "e2e_radicali": {"wasm"}, "e2e_radicali_ms": {"wasm"},
    "e2e_feat_na": {"wasm"}, "e2e_feat_worker": {"wasm"}, "e2e_frammenti": {"wasm"}, "e2e_integra_base": {"wasm"}, "e2e_ruoli_tempi": {"wasm"}, "e2e_taglio": {"wasm"},   # same: the feature-finder and cut-line tests, found red in `e2e-cambiati` one run at a time
    "e2e_tpmine_mem": {"sito", "veri"},          # 13 real HR files (1 GB) in the real worker: minutes
}
# the slowest e2e (seconds, measured in the CI): they start first, so the pool does not end waiting for one of them
SLOW = ["e2e_tpmine_lr_rust", "e2e_tpmine_ingresso", "e2e_tpmine_transf"]; SOLI = ["e2e_carica_grandi", "e2e_sw_versione"]      # SOLI run alone after the pool: carica_grandi fills the browser storage and memory on purpose, sw_versione checks that a deploy shows at the first reload inside the 3 s navigation budget of the worker (a loaded machine spends it); the tests beside them never starve
NOT_TESTS = {"lib", "synth"}          # helpers and measurements, not tests
# e2e3 plus the e2e that were red on main without anybody seeing it (the CI used to run only e2e3): download menu, buttons, settings, axes, spectra.
# e2e_cromato is not here yet: it fails in WebKit (the dialog of the integrations has no S/N box) and in Firefox (waterfall step), both only in those browsers.
# About 3 minutes together on a laptop; the whole set (python3 tools/verifica.py) stays the rule before every merge.
SMOKE = ["e2e3", "e2e_opfs_promote", "e2e8", "e2e18", "e2e19", "e2e20", "e2e_spettro", "e2e_hr_nearest", "e2e_hr_composizione", "e2e_pannelli2",
         "e2e_hr_isotopi", "e2e_picchi", "e2e_a11y_dati", "e2e_safari_ipad", "e2e_safari_mac", "e2e_safari_canvas", "e2e_safari_avviso"]
# Tabui, the guided tour, the header walk, the two-part catalog, the integration drags and the start-up budget join the smoke round on Chromium Linux only (the PR job): not yet measured on Firefox, WebKit and Windows.
SMOKE_CHROMIUM_LINUX = ["e2e_tabui_tptab", "e2e_tour", "e2e_lab", "e2e_giro_intestazione", "e2e_lingua_parti", "e2e_integrazioni", "e2e_budget_avvio"]
# Console messages that are known and harmless (not counted as browser errors).
BENIGN = [r"allow-scripts and allow-same-origin", r"/api/(ping|bye|live)", r"ERR_ABORTED", r"'reqfail', '[^']*/static/vendor/ketcher/[^']*', 'NS_BINDING_ABORTED'", r"JavaScript Error: \"Failed to load \S*\. A ServiceWorker intercepted the request and encountered an unexpected error\.\" \{file: \"[^\"]*/sw\.js\"", r"api/formula\?f=C2H6Qq", r"status of 400",
          r"Layout was forced before the page was fully loaded", r"cross-world service worker resource mismatch",
          r"IndexedDB transaction that was not yet complete has been aborted due to page navigation"]   # Firefox: notices about styles and about a reload while a write was in flight (the test reloads on purpose), not errors   # aborted requests (Chromium ERR_ABORTED, Firefox NS_BINDING_ABORTED and its service-worker notice) = page reloaded or closed by the test


def sh(cmd, log: Path, timeout: int, env=None, cwd=ROOT) -> tuple[int, str, float]:
    t0 = time.time()
    # the tests print m/z, deltas, symbols: on Windows the console is cp1252 and a print of «Δ» or «🔗» stopped the test with UnicodeEncodeError (CI job windows-chromium)
    env = {**(env if env is not None else os.environ), "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"}
    try:
        p = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=timeout)
        out, rc = (p.stdout or "") + (p.stderr or ""), p.returncode
    except subprocess.TimeoutExpired as e:
        txt = lambda v: v.decode("utf-8", "replace") if isinstance(v, bytes) else (v or "")
        out, rc = f"{txt(e.stdout)}\n{txt(e.stderr)}\nTIMEOUT dopo {timeout} s", 124
    except FileNotFoundError as e:
        out, rc = f"comando non trovato: {e}", 127
    log.write_text(out, encoding="utf-8")
    return rc, out, time.time() - t0


def has(mod: str) -> bool:
    return subprocess.run([sys.executable, "-c", f"import {mod}"], capture_output=True).returncode == 0


def get_env(suffix: str, default: str | None = None) -> str | None:
    """Read MZLAB_<suffix> or QQQ_<suffix> (MZLAB_* takes precedence)."""
    return os.environ.get(f"MZLAB_{suffix}", os.environ.get(f"QQQ_{suffix}", default))


def browser_name() -> str:
    """chromium (default), firefox or webkit (the engine of Safari): --browser or MZLAB_BROWSER / QQQ_BROWSER; tests_e2e/lib.py reads the same variable."""
    return get_env("BROWSER", "chromium")


def setup() -> None:
    pip = [sys.executable, "-m", "pip", "install", "-q"]
    for extra in ([], ["--break-system-packages"]):
        if subprocess.run(pip + extra + ["pytest", "playwright", "numpy"], capture_output=True).returncode == 0:
            break
    if has("playwright"):
        subprocess.run([sys.executable, "-m", "playwright", "install", "--with-deps", browser_name()], capture_output=True)
        subprocess.run([sys.executable, "-m", "playwright", "install", browser_name()], capture_output=True)


def browser_ok() -> bool:
    if not has("playwright"):
        return False
    code = "import sys; sys.path.insert(0, %r)\nimport chromium_cloud; chromium_cloud.abilita()\nfrom playwright.sync_api import sync_playwright\nwith sync_playwright() as p: p.%s.launch().close()" % (str(ROOT / "tools"), browser_name())
    return subprocess.run([sys.executable, "-c", code], capture_output=True, timeout=120).returncode == 0


def i18n(results) -> None:
    """Both interface languages: catalogs, keys, static HTML, glossary, no Italian outside the catalogs (tools/controlla_i18n.py)."""
    t0 = time.time()
    p = subprocess.run([sys.executable, str(ROOT / "tools" / "controlla_i18n.py")], capture_output=True, text=True, encoding="utf-8", errors="replace")
    out = (p.stdout or "") + (p.stderr or "")
    (LOG / "i18n.log").write_text(out, encoding="utf-8")
    results.append(("lingue (i18n)", "FAIL" if p.returncode else "OK", time.time() - t0, out.strip().splitlines()[:6] if p.returncode else []))


def codice(results) -> None:
    """The index of the code can be generated and covers every source file (tools/indice.py); the size limits do not get worse (tools/controlla.py)."""
    subprocess.run([sys.executable, str(ROOT / "tools" / "indice.py")], capture_output=True)          # refresh the generated docs/agenti/mappa.md
    for name, script, args in (("mappa del codice", "indice.py", ["--controlla"]), ("soglie di dimensione", "controlla.py", [])):
        t0 = time.time()
        p = subprocess.run([sys.executable, str(ROOT / "tools" / script)] + args, capture_output=True, text=True, encoding="utf-8", errors="replace")
        out = (p.stdout or "") + (p.stderr or "")
        (LOG / f"{script[:-3]}.log").write_text(out, encoding="utf-8")
        results.append((name, "FAIL" if p.returncode else "OK", time.time() - t0, out.strip().splitlines()[:6] if p.returncode else []))


def known_reds() -> dict[str, str]:
    """tools/noti_rossi.txt: `name | reason` per line, `#` comments. A FAIL of one of these is reported as NOTO (not blocking)."""
    f = ROOT / "tools" / "noti_rossi.txt"
    out: dict[str, str] = {}
    if f.exists():
        for l in f.read_text(encoding="utf-8").splitlines():
            if l.strip() and not l.lstrip().startswith("#"):
                n, _, why = l.partition("|")
                out[n.strip()] = why.strip()
    return out


KNOWN_GREEN_ROUNDS = 3


def known_greens(results, known: dict[str, str]):
    """A line of noti_rossi.txt whose e2e passes for NOTI_VERDI_GIRI runs in a row becomes a FAIL until the line is removed (streaks in .verifica/noti_verdi.json)."""
    f = OUT / "noti_verdi.json"
    try:
        series = json.loads(f.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        series = {}
    stato = {n: st for n, st, _dt, _notes in results}
    series = {n: c for n, c in series.items() if n in known}
    for n in known:
        if stato.get(n) == "OK":
            series[n] = series.get(n, 0) + 1
        elif stato.get(n) == "FAIL":
            series[n] = 0
    try:
        f.write_text(json.dumps(series), encoding="utf-8")
    except OSError:
        pass
    return [(f"noti_rossi: {n}", "FAIL", 0, [f"green for {c} runs in a row: remove the line from tools/noti_rossi.txt"]) for n, c in series.items() if c >= KNOWN_GREEN_ROUNDS]


def js_syntax(results) -> None:
    web = ROOT / "mzlab" / "web"
    tmp = OUT / "js"; tmp.mkdir(parents=True, exist_ok=True)
    if not shutil.which("node"):
        results.append(("sintassi JS", "SKIP", 0, ["node non installato"])); return
    t0, errs = time.time(), []
    files = [f for f in web.glob("*.js") if f.name not in ("elements.js",)]
    for f in files:
        src = f
        if f.name in ("browser-worker.js", "archivio.js", "qqq-db.js", "draw.js", "disegno-barra.js", "disegno-font.js", "disegno-cerca.js", "disegno-frammenti.js", "disegno-parziali.js", "disegno-taglio.js", "disegno-meccanismo.js", "disegno-radicale.js", "disegno-indigo.js", "disegno-menu.js", "disegno-selezione.js", "disegno-link.js", "disegno-molfile.js", "disegno-etichette.js", "parentesi.js", "sw.js", "rust-worker.js", "rust-bridge.js", "rust-scans.js", "migra.js", "manifesto.js", "pulizia.js", "rust-recycle.js", "rust-diff.js", "rust-centroidi.js", "rust-opfs.js", "rust-metodo.js", "mappa3d.js", "workflow.js", "fair.js", "perf.js", "xic-spettro.js", "xic-lente.js", "qualita.js", "peaks-worker.js"):        # module-style files: checked as .mjs (as the CI does)
            src = tmp / (f.stem + ".mjs"); shutil.copy(f, src)
        p = subprocess.run(["node", "--check", str(src)], capture_output=True, text=True)
        if p.returncode:
            errs.append(f"{f.relative_to(ROOT)}: " + (p.stderr.strip().splitlines() or ["?"])[-1])
    html = (web / "index.html").read_text(encoding="utf-8")
    inline = "\n".join(re.findall(r"<script(?: type=\"text/plain\" id=\"shell\")?>(.*?)</script>", html, re.S))
    page = tmp / "page.js"; page.write_text("\n".join((web / n).read_text(encoding="utf-8") for n in CORE_JS) + "\n" + inline, encoding="utf-8")
    p = subprocess.run(["node", "--check", str(page)], capture_output=True, text=True)
    if p.returncode:
        errs.append("explore.js e compagni + script inline di index.html: " + (p.stderr.strip().splitlines() or ["?"])[-1])
    (LOG / "sintassi_js.log").write_text("\n".join(errs) or "ok", encoding="utf-8")
    results.append(("sintassi JS", "FAIL" if errs else "OK", time.time() - t0, errs[:5]))


def pytest(results, timeout) -> None:
    if not has("pytest"):
        results.append(("pytest", "SKIP", 0, ["pytest non installato: python3 tools/verifica.py --setup"])); return
    # serial pytest needs 20+ min on 4 CPUs (1800 s with xdist); one serial cargo build up front: under xdist the tests that call cargo built together and one worker sat 1800 s on `--example native` (test_valida_hr.py); the cargo shim first in PATH as in test.yml, so no `cargo run` holds the lock for minutes (night 43)
    env = None if os.name == "nt" else {**os.environ, "PATH": str(ROOT / "tools" / "ci_cargo") + os.pathsep + os.environ.get("PATH", "")}; par = ["-n", "auto", "--dist", "loadfile"] if has("xdist") else []
    for pkg in (["-p", "mzlab-feat-wasm", "--example", "native"], ["-p", "mzlab-cli"]) if shutil.which("cargo") else []:
        rc, out, dt = sh(["cargo", "build", "--release", "--locked", *pkg], LOG / "pytest-cargo.log", 1500, env=env)
        if rc: results.append(("pytest (cargo build)", "FAIL", dt, out.strip().splitlines()[-6:])); return
    # faulthandler dumps the stack of a test that runs for over 7 min, so a hang names its test in the log (the dots do not)
    rc, out, dt = sh([sys.executable, "-m", "pytest", "-q", "-p", "no:cacheprovider", "-o", "faulthandler_timeout=420", "--durations=10", *par, "tests"], LOG / "pytest.log", timeout, env=env)
    last = [l for l in out.strip().splitlines() if re.search(r"passed|failed|error", l)][-1:] or out.strip().splitlines()[-1:]
    fails = [l for l in out.splitlines() if l.startswith(("FAILED", "ERROR"))][:6]
    results.append(("pytest", "OK" if rc == 0 else "FAIL", dt, (fails or []) + last if rc else last))


def data_dir() -> tuple[Path, str]:
    """Real lab mzML if found (MZLAB_MZML / QQQ_MZML, the data repository, the Mac folders), otherwise synthetic look-alikes."""
    for cand in (get_env("MZML"), percorso("dati/3200 qtrap/mzML"), ROOT.parent / "Data" / "mzML",
                 ROOT.parent / "esempio_conversione" / "mzml"):
        if cand and (Path(cand) / "B_FullMass-t0.mzML").exists():
            return Path(cand), "veri"
    return sintetici(), "sintetici"


def sintetici() -> Path:
    """The synthetic look-alikes (tools/dati_sintetici.py), written once into .verifica/dati_sintetici/."""
    syn = OUT / "dati_sintetici"
    if not (syn / "B_FullMass-t0.mzML").exists():
        sys.path.insert(0, str(ROOT / "tools"))
        import dati_sintetici
        dati_sintetici.make(syn)
    return syn


def con_golden(n: str) -> bool:
    """An e2e that answers from committed golden data (tests_e2e/golden_ref.py): they exist only for the synthetic files, never for real lab data."""
    return "golden_ref" in (E2E / f"{n}.py").read_text(encoding="utf-8")


def hrms_dirs() -> list[Path]:
    """Folders with high-resolution Orbitrap files: the two cuttings of the private data repository (HRMS/) and, on Federico's Mac, the whole files."""
    hrms_env = get_env("HRMS")
    c = [percorso("dati/HRMS"), Path(hrms_env) if hrms_env else None, ROOT.parent / "Data" / "HRMS"]
    return [d for d in c if d and d.is_dir() and any(d.glob("*.mzML"))]


def hr_round(results) -> None:
    """Technical numbers of the real Orbitrap files (tools/prova_hr.py); nothing to check, only information."""
    dirs = hrms_dirs()
    if not dirs:
        results.append(("prova_hr", "SKIP", 0, ["nessuna cartella HRMS (mzlab-dati/HRMS o ../Data/HRMS)"])); return
    for d in dirs:
        rc, out, dt = sh([sys.executable, str(ROOT / "tools" / "prova_hr.py"), str(d)], LOG / "prova_hr.log", 900)
        results.append((f"prova_hr {d.parent.name}/{d.name}", "OK" if rc == 0 and "ERRORE" not in out else "FAIL", dt,
                        [l[:200] for l in out.strip().splitlines()]))


def dam_file() -> Path | None:
    name = "Lab_inq_FullMass_pos_max480.dam"            # e2e15 also wants Lab_inq_MRM_Flufe.dam in the same folder
    for cand in (get_env("DAM"), percorso(f"dati/3200 qtrap/dam/{name}"), ROOT.parent / "Data" / "dam - Metodi" / name,
                 ROOT.parent / "QqQ" / "Metodi inquinanti" / name):
        if cand and Path(cand).exists():
            return Path(cand)
    return None


def judge(out: str, rc: int) -> list[str]:
    """Problems found in an e2e output: failed steps, Python errors, real browser errors."""
    probs = []
    for l in out.splitlines():
        s = l.strip()
        if re.search(r"['\"]FAIL|^FAIL\b|\bFAIL ", s) or s.startswith(("Traceback", "AssertionError", "TimeoutError", "TIMEOUT")):
            probs.append(s[:220])
    m = re.search(r"BROWSER ERRORS (\d+)\n(.*?)(?:\n\S|\Z)", out, re.S)
    if m and int(m.group(1)):
        errs = [e for e in out[m.end(1):].splitlines()[1:int(m.group(1)) + 1] if e.strip() and not any(re.search(b, e) for b in BENIGN)]
        probs += ["errore del browser: " + e.strip()[:200] for e in errs]
    if rc and not probs:
        probs.append(f"uscita con codice {rc}: " + (out.strip().splitlines() or [""])[-1][:200])
    return probs


CORE_JS = ("explore.js", "pannelli.js", "grafici.js", "picchi.js", "eventi.js")       # explore.js split by topic, in the order of index.html


from verifica_aree import DOCS, NO_E2E, changed_tests, covers  # noqa: E402,F401  (the --cambiati table lives in verifica_aree.py)


def last_failed() -> set[str]:
    """Names of the e2e that were FAIL in the previous run, read from .verifica/ultimo.md (the lines «- FAIL e2e6 (50 s): ...»)."""
    try:
        text = (OUT / "ultimo.md").read_text(encoding="utf-8")
    except OSError:
        return set()
    return {m.group(1) for m in re.finditer(r"^- FAIL +(e2e\w*)", text, re.M) if (E2E / f"{m.group(1)}.py").exists()}


def parte_di(names: list[str], k: int, n: int) -> list[str]:
    """Part k of n (1-based) of the e2e `names`: the slow ones first, dealt round-robin, so the parts take about the same time and are the same on every call."""
    rank = [x for x in SLOW if x in names] + [x for x in names if x not in SLOW]
    return [x for i, x in enumerate(rank) if i % n == k - 1]


def skip_why(need: set[str], *, dam: bool, src: str, only: bool, sito_pronto: bool) -> str:
    """Why an e2e with these NEEDS is skipped ("" = it runs). The `sito` ones run only when named (--solo) or when the CI says the full site is in place (--sito-pronto)."""
    return ("serve un file .dam (MZLAB_DAM / QQQ_DAM)" if "dam" in need and not dam else
            "solo con i dati veri del laboratorio" if "veri" in need and src != "veri" else
            "costruisce il sito con Pyodide: lancialo a parte (--solo)" if "sito" in need and not only and not sito_pronto else "")


def e2e(results, only, timeout, kind, jobs: int = 1, parte: tuple[int, int] | None = None, sito_pronto: bool = False) -> None:
    names = sorted(p.stem for p in E2E.glob("e2e*.py") if p.stem not in NOT_TESTS)
    if only:
        names = [n for n in names if n in only]
    if parte:
        keep = set(parte_di(names, *parte))
        names = [n for n in names if n in keep]
    if not browser_ok():
        results.append(("e2e", "SKIP", 0, ["Playwright/Chromium non disponibili: python3 tools/verifica.py --setup"])); return
    mz, src = data_dir()
    dam = dam_file()
    env = dict(os.environ, MZLAB_MZML=str(mz), QQQ_MZML=str(mz), PYTHONUNBUFFERED="1", RUSTUP_AUTO_INSTALL="0")  # tools/prepara_sito.py installs the toolchain once; a test may not
    if dam:
        env["MZLAB_DAM"] = str(dam)
        env["QQQ_DAM"] = str(dam)
    if jobs > 1:
        env["MZLAB_E2E_PARALLEL"] = "1"
        env["QQQ_E2E_PARALLEL"] = "1"          # tests_e2e/lib.py: every test takes a free port and its own work folder
    todo, out_by = [], {}
    for n in names:
        need = NEEDS.get(n, set())
        why = skip_why(need, dam=bool(dam), src=src, only=bool(only), sito_pronto=sito_pronto)
        if why:
            out_by[n] = (n, "SKIP", 0, [why])
        else:
            todo.append(n)
    env_sint = dict(env, MZLAB_MZML=str(sintetici()), QQQ_MZML=str(sintetici())) if src == "veri" and any(con_golden(n) for n in names) else env
    def one(n):
        rc, out, dt = sh([sys.executable, str(E2E / f"{n}.py")], LOG / f"{n}.log", timeout, env=env_sint if con_golden(n) else env, cwd=ROOT)
        probs = judge(out, rc)
        return (n, "FAIL" if probs else "OK", dt, probs[:4])
    if todo:           # the page the tests open is the built site with the Rust engine (tests_e2e/lib.py Run)
        rc, _out, dt = sh([sys.executable, str(ROOT / "tools" / "prepara_sito.py")], LOG / "prepara_sito.log", 3600, env=env)   # cold cache: engine, libraries, 9 reader families
        if rc:
            results.append(("sito per gli e2e (tools/prepara_sito.py)", "FAIL", dt, ["vedi .verifica/log/prepara_sito.log"])); return
    if shutil.which("cargo") and any("cargo" in (E2E / f"{n}.py").read_text(encoding="utf-8") for n in todo):      # the tests that call `cargo run` would each wait on cargo's build-folder lock, or rebuild a stale tree, inside the timeout of their own: build the two binaries once
        for pkg in (["-p", "mzlab-cli"], ["-p", "mzlab-feat-wasm", "--example", "native"]):
            rc, _out, dt = sh(["cargo", "build", "--release", "-q", *pkg], LOG / "precompila.log", 3000, env=env)
            if rc:
                results.append((f"cargo build {' '.join(pkg)} (prima degli e2e)", "FAIL", dt, ["vedi .verifica/log/precompila.log"])); return
    # every test takes a free port and its own work folder (tests_e2e/lib.py), the site tests too: they all run in the same pool, the slowest first
    from concurrent.futures import ThreadPoolExecutor
    slow = [n for n in SLOW if n in todo]
    with ThreadPoolExecutor(max_workers=max(1, jobs)) as ex:
        for r in ex.map(one, slow + [n for n in todo if n not in slow and n not in SOLI]):
            out_by[r[0]] = r
    out_by.update({n: one(n) for n in SOLI if n in todo}); results.extend(out_by[n] for n in names)
    results.append(("dati usati per gli e2e", "INFO", 0, [f"{src}: {mz}" + (f" · .dam: {dam.name}" if dam else " · nessun .dam")]))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--rapida", action="store_true", help="sintassi + pytest + e2e3")
    ap.add_argument("--solo", default="", help="solo questi e2e, separati da virgola (es. e2e6,e2e18)")
    ap.add_argument("--senza-e2e", action="store_true")
    ap.add_argument("--browser", choices=("chromium", "firefox", "webkit"), help="browser dei test e2e (predefinito chromium; webkit = il motore di Safari)")
    ap.add_argument("--fumo", action="store_true", help="solo gli e2e di fumo (SMOKE), senza sintassi JS e pytest: è il giro del job «browser» della CI")
    ap.add_argument("--setup", action="store_true", help="installa pytest, playwright e chromium se mancano")
    ap.add_argument("--timeout", type=int, default=600, help="secondi per ogni e2e (predefinito 600)")
    ap.add_argument("--paralleli", type=int, default=max(1, min(6, (os.cpu_count() or 2) - 1)),
                    help="e2e eseguiti insieme (predefinito: CPU - 1, al massimo 6; 1 = uno alla volta)")
    ap.add_argument("--ultimi-falliti", action="store_true", help="rilancia solo gli e2e che erano FAIL nel giro precedente (.verifica/ultimo.md)")
    ap.add_argument("--parte", default="", metavar="K/N", help="only part K of N of the complete run (the CI of notte.yml): part 1 also does syntax, texts, pytest and prova_hr, every part a share of the e2e")
    ap.add_argument("--sito-pronto", action="store_true", help="the built site with every .wasm is in place (notte.yml): the e2e that need `sito` run in --parte mode too")
    ap.add_argument("--cambiati", action="store_true", help="solo gli e2e che riguardano i file cambiati rispetto a origin/main (vedi AREE); con file del nucleo li fa tutti")
    ap.add_argument("--mostra-ok", "--tutto", dest="mostra_ok", action="store_true", help="stampa anche le righe OK (altrimenti solo FAIL/SKIP e il conteggio)")
    a = ap.parse_args()
    if a.browser:
        os.environ["MZLAB_BROWSER"] = a.browser
        os.environ["QQQ_BROWSER"] = a.browser
    LOG.mkdir(parents=True, exist_ok=True)
    if a.setup:
        setup()
    results: list = []
    t0 = time.time()
    parte = tuple(int(x) for x in a.parte.split("/")) if a.parte else None
    if parte and not (len(parte) == 2 and 1 <= parte[0] <= parte[1]):
        ap.error("--parte wants K/N with 1 <= K <= N")
    if not a.fumo and not a.ultimi_falliti and not a.solo and (not parte or parte[0] == 1):      # --solo = e2e only: `--solo e2e_rust` after part 5 ran a second pytest past 1800 s
        js_syntax(results)
        i18n(results)
        codice(results)
        pytest(results, 1800)
        if not a.rapida:
            hr_round(results)
    if not a.senza_e2e:
        only = {s.strip().removesuffix(".py") for s in a.solo.split(",") if s.strip()} or (set(SMOKE) | (set(SMOKE_CHROMIUM_LINUX) if browser_name() == "chromium" and sys.platform.startswith("linux") else set()) if a.rapida or a.fumo else set())
        if a.ultimi_falliti and not only:
            only = last_failed()
            if not only:
                results.append(("e2e", "INFO", 0, ["nessun e2e FAIL nel giro precedente (.verifica/ultimo.md)"]))
        run = True
        if a.cambiati and not only:
            sel, why = changed_tests()
            results.append(("e2e scelti da --cambiati", "INFO", 0, [why]))
            run, only = sel != {"-"}, (sel if sel not in (None, {"-"}) else set())
        if run and not (a.ultimi_falliti and not only):
            e2e(results, only, a.timeout, "", a.paralleli, parte, a.sito_pronto)
    known = known_reds()
    extra = known_greens(results, known)
    results = [(n, "NOTO", dt, ["known: " + known[n]] + notes[:1]) if st == "FAIL" and n in known else
               (n, "FAIL", dt, (["NUOVO"] if n.startswith("e2e") else []) + notes) if st == "FAIL" else (n, st, dt, notes)
               for n, st, dt, notes in results] + extra
    fails = [r for r in results if r[1] == "FAIL"]
    lines = [f"# Verifica QqQ lab ({time.strftime('%Y-%m-%d %H:%M')}, {time.time() - t0:.0f} s, browser {browser_name()}): "
             + ("TUTTO OK" if not fails else f"{len(fails)} FAIL"), ""]
    for name, st, dt, notes in results:
        lines.append(f"- {st:4} {name}" + (f" ({dt:.0f} s)" if dt else "") + (": " + " | ".join(notes) if notes and (st != "OK" or name.startswith("prova_hr")) else ""))
    timed = sorted((r for r in results if r[0].startswith("e2e") and r[1] in ("OK", "FAIL") and r[2]), key=lambda r: -r[2])[:5]
    if timed:
        lines += ["", "I 5 e2e più lenti: " + ", ".join(f"{r[0]} {r[2]:.0f} s" for r in timed)]
    lines += ["", "Log completi: .verifica/log/<nome>.log (leggili solo per i FAIL)."]
    text = "\n".join(lines)
    (OUT / "ultimo.md").write_text(text + "\n", encoding="utf-8")
    if a.mostra_ok:
        print(text)
    else:          # fewer tokens for whoever reads it: only what is not OK, plus how many are OK (the full list is in .verifica/ultimo.md)
        ok = [r for r in results if r[1] == "OK"]
        print("\n".join([lines[0], ""] + [l for l in lines[2:] if not l.startswith("- OK")] +
                         [f"OK: {len(ok)} controlli (elenco in .verifica/ultimo.md)"]))
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
