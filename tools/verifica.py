"""One command that checks the whole program and prints a SHORT summary (made to save the reader's time and tokens).

    python3 tools/verifica.py                 # everything: JS syntax, pytest, every e2e that can run here
    python3 tools/verifica.py --rapida        # JS syntax + pytest + the e2e smoke test (e2e3): ~2 minutes
    python3 tools/verifica.py --solo e2e6,e2e18
    python3 tools/verifica.py --ultimi-falliti   # only the e2e that were FAIL in the previous run (.verifica/ultimo.md)
    python3 tools/verifica.py --setup         # first install what is missing (pytest, playwright + chromium), then check

Data, in this order: MZLAB_MZML (or legacy QQQ_MZML); the PRIVATE data repository "mzlab-dati" or "QqQ-lab-dati" (Freeesta/mzlab-dati or Freeesta/QqQ-lab-dati: mzML/ + dam/, real lab
files; found by itself next to this repository, in a parent folder, in the home or in /workspace, or with MZLAB_DATI=<path> or QQQ_DATI=<path>);
Federico's Mac folders (../Data/mzML); otherwise synthetic look-alikes written by tools/dati_sintetici.py into
.verifica/dati_sintetici/ (then the summary says "sintetici": a pass on synthetic data is weaker than on real data).
Some e2e only make sense with the real data or with a .dam method (MZLAB_DAM or QQQ_DAM): they are reported as SKIP, with why.

Output: one line per check on stdout (OK / FAIL / SKIP, seconds, first error lines), the same summary in
.verifica/ultimo.md, and the full output of every check in .verifica/log/<name>.log. Read the logs only for a FAIL.
Exit code 0 only if nothing failed. `.verifica/` is not committed (.gitignore).
"""
from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / ".verifica"
LOG = OUT / "log"
E2E = ROOT / "tests_e2e"

# What each e2e needs besides a browser. "veri" = the asserts depend on the real lab data (skipped with synthetic data);
# "dam" = needs a .dam method file (QQQ_DAM); "sito" = builds the static site with Pyodide (network to jsDelivr or PYODIDE_DIR);
# Keep this table up to date when you add an e2e (default: no needs).
NEEDS: dict[str, set[str]] = {
    "e2e9": {"dam"}, "e2e10": {"dam"}, "e2e15": {"dam"}, "e2e_dam_incoerente": {"dam"},
    "e2e_studenti": {"veri"},          # needs the whole series B (7 times, 6 standards): the synthetic files are only a few
    "e2e13": {"sito"}, "e2e_avvio": {"sito"}, "e2e_mzidx": {"sito"}, "e2e_rust": {"sito"}, "e2e_lr": {"sito"}, "e2e_picchi_qualita": {"sito"}, "e2e_pool": {"sito"}, "e2e_libreria_wasm": {"sito"}, "e2e_libreria_cartella": {"sito"}, "e2e_libreria_scarica": {"sito"},  "e2e_tpmine1": {"sito"}, "e2e_tpmine2": {"sito"}, "e2e_tpmine_albero": {"sito"}, "e2e_tpmine_rtalign": {"sito"}, "e2e_tpmine_ms2": {"sito"}, "e2e_untarget": {"sito"}, "e2e_identita": {"sito"},
    "e2e_tpmine_mem": {"sito", "veri"},          # 13 real HR files (1 GB) in the real worker: minutes
}
NOT_TESTS = {"lib", "synth", "lat_arrows", "make_examples"}          # helpers and measurements, not tests
# e2e3 plus the e2e that were red on main without anybody seeing it (the CI used to run only e2e3): download menu, buttons, settings, axes, spectra.
# e2e_cromato is not here yet: it fails in WebKit (the dialog of the integrations has no S/N box) and in Firefox (waterfall step), both only in those browsers.
# About 3 minutes together on a laptop; the whole set (python3 tools/verifica.py) stays the rule before every merge.
SMOKE = ["e2e3", "e2e8", "e2e18", "e2e19", "e2e20", "e2e_spettro", "e2e_hr_nearest", "e2e_hr_composizione", "e2e_pannelli2",
         "e2e_hr_isotopi", "e2e_picchi"]
# Console messages that are known and harmless (not counted as browser errors).
BENIGN = [r"allow-scripts and allow-same-origin", r"/api/(ping|bye|live)", r"ERR_ABORTED", r"api/formula\?f=C2H6Qq", r"status of 400",
          r"Layout was forced before the page was fully loaded"]   # Firefox: a notice about styles, not an error   # aborted requests = page reloaded or closed by the test


def sh(cmd, log: Path, timeout: int, env=None, cwd=ROOT) -> tuple[int, str, float]:
    t0 = time.time()
    # the tests print m/z, deltas, symbols: on Windows the console is cp1252 and a print of «Δ» or «🔗» stopped the test with UnicodeEncodeError (CI job windows-chromium)
    env = {**(env if env is not None else os.environ), "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"}
    try:
        p = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=timeout)
        out, rc = (p.stdout or "") + (p.stderr or ""), p.returncode
    except subprocess.TimeoutExpired as e:
        out, rc = f"{(e.stdout or b'').decode('utf-8', 'replace') if isinstance(e.stdout, bytes) else (e.stdout or '')}\nTIMEOUT dopo {timeout} s", 124
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
    code = "from playwright.sync_api import sync_playwright\nwith sync_playwright() as p: p.%s.launch().close()" % browser_name()
    return subprocess.run([sys.executable, "-c", code], capture_output=True, timeout=120).returncode == 0


def i18n(results) -> None:
    """Both interface languages: catalogs, keys, static HTML, glossary, no Italian outside the catalogs (tools/controlla_i18n.py)."""
    t0 = time.time()
    p = subprocess.run([sys.executable, str(ROOT / "tools" / "controlla_i18n.py")], capture_output=True, text=True, encoding="utf-8", errors="replace")
    out = (p.stdout or "") + (p.stderr or "")
    (LOG / "i18n.log").write_text(out, encoding="utf-8")
    results.append(("lingue (i18n)", "FAIL" if p.returncode else "OK", time.time() - t0, out.strip().splitlines()[:6] if p.returncode else []))


def codice(results) -> None:
    """The map of the code is up to date (tools/indice.py) and the size limits do not get worse (tools/controlla.py)."""
    for name, script, args in (("mappa del codice", "indice.py", ["--controlla"]), ("soglie di dimensione", "controlla.py", [])):
        t0 = time.time()
        p = subprocess.run([sys.executable, str(ROOT / "tools" / script)] + args, capture_output=True, text=True, encoding="utf-8", errors="replace")
        out = (p.stdout or "") + (p.stderr or "")
        (LOG / f"{script[:-3]}.log").write_text(out, encoding="utf-8")
        results.append((name, "FAIL" if p.returncode else "OK", time.time() - t0, out.strip().splitlines()[:6] if p.returncode else []))


def noti_rossi() -> dict[str, str]:
    """tools/noti_rossi.txt: `name | reason` per line, `#` comments. A FAIL of one of these is reported as NOTO (not blocking)."""
    f = ROOT / "tools" / "noti_rossi.txt"
    out: dict[str, str] = {}
    if f.exists():
        for l in f.read_text(encoding="utf-8").splitlines():
            if l.strip() and not l.lstrip().startswith("#"):
                n, _, why = l.partition("|")
                out[n.strip()] = why.strip()
    return out


def teoria(results) -> None:
    """Teoria chapters: structure, accessibility, markers, noindex; with MZLAB_DATI also forbidden names and copying (tools/controlla_teoria.py)."""
    t0 = time.time()
    p = subprocess.run([sys.executable, str(ROOT / "tools" / "controlla_teoria.py")], capture_output=True, text=True, encoding="utf-8", errors="replace")
    out = (p.stdout or "") + (p.stderr or "")
    (LOG / "teoria.log").write_text(out, encoding="utf-8")
    results.append(("Teoria (capitoli)", "FAIL" if p.returncode else "OK", time.time() - t0, out.strip().splitlines()[:6] if p.returncode else []))


def js_syntax(results) -> None:
    web = ROOT / "mzlab" / "web"
    tmp = OUT / "js"; tmp.mkdir(parents=True, exist_ok=True)
    if not shutil.which("node"):
        results.append(("sintassi JS", "SKIP", 0, ["node non installato"])); return
    t0, errs = time.time(), []
    files = [f for f in web.glob("*.js") if f.name not in ("elements.js",)] + list((web / "teoria").glob("*.js"))
    for f in files:
        src = f
        if f.name in ("browser-worker.js", "archivio.js", "draw.js", "sw.js", "rust-worker.js", "rust-bridge.js", "mappa3d.js", "workflow.js", "fair.js", "perf.js", "qualita.js", "peaks-worker.js"):        # module-style files: checked as .mjs (as the CI does)
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
    rc, out, dt = sh([sys.executable, "-m", "pytest", "-q", "-p", "no:cacheprovider", "tests"], LOG / "pytest.log", timeout)
    last = [l for l in out.strip().splitlines() if re.search(r"passed|failed|error", l)][-1:] or out.strip().splitlines()[-1:]
    fails = [l for l in out.splitlines() if l.startswith(("FAILED", "ERROR"))][:6]
    results.append(("pytest", "OK" if rc == 0 else "FAIL", dt, (fails or []) + last if rc else last))
    priv = ROOT / "TP_Mine"
    if (priv / "tests").is_dir():
        env = dict(os.environ, PYTHONPATH=f"{ROOT}{os.pathsep}{priv / 'py'}")
        rc, out, dt = sh([sys.executable, "-m", "pytest", "-q", "-p", "no:cacheprovider", str(priv / "tests")], LOG / "pytest_tpmine.log", timeout, env=env)
        last = out.strip().splitlines()[-1:] if out.strip() else []
        fails = [l for l in out.splitlines() if l.startswith(("FAILED", "ERROR"))][:4]
        results.append(("pytest TP Mine", "OK" if rc == 0 else "FAIL", dt, fails + last if rc else last))


def dati_repo() -> Path | None:
    """The private data repository mzlab-dati / QqQ-lab-dati (real lab files: mzML/ and dam/), wherever it was cloned."""
    env = get_env("DATI")
    cands = [Path(env)] if env else []
    for base in (ROOT.parent, ROOT.parent.parent, Path.home(), Path("/workspace"), Path("/workspaces"), Path("/repos"), Path("/tmp")):
        for name in ("mzlab-dati", "QqQ-lab-dati"):
            cands += [base / name, base / "Data" / name]
            try:
                cands += sorted(base.glob(f"*/{name}"))
            except OSError:
                pass
    return next((c for c in cands if (c / "mzML" / "B_FullMass-t0.mzML").exists()), None)


def data_dir() -> tuple[Path, str]:
    """Real lab mzML if found (MZLAB_MZML / QQQ_MZML, the data repository, the Mac folders), otherwise synthetic look-alikes."""
    repo = dati_repo()
    for cand in (get_env("MZML"), repo / "mzML" if repo else None, ROOT.parent / "Data" / "mzML",
                 ROOT.parent / "esempio_conversione" / "mzml"):
        if cand and (Path(cand) / "B_FullMass-t0.mzML").exists():
            return Path(cand), "veri"
    syn = OUT / "dati_sintetici"
    if not (syn / "B_FullMass-t0.mzML").exists():
        sys.path.insert(0, str(ROOT / "tools"))
        import dati_sintetici
        dati_sintetici.make(syn)
    return syn, "sintetici"


def hrms_dirs() -> list[Path]:
    """Folders with high-resolution Orbitrap files: the two cuttings of the private data repository (HRMS/) and, on Federico's Mac, the whole files."""
    repo = dati_repo()
    hrms_env = get_env("HRMS")
    c = [repo / "HRMS" if repo else None, Path(hrms_env) if hrms_env else None, ROOT.parent / "Data" / "HRMS"]
    return [d for d in c if d and d.is_dir() and any(d.glob("*.mzML"))]


def prova_hr(results) -> None:
    """Technical numbers of the real Orbitrap files (tools/prova_hr.py); nothing to check, only information."""
    dirs = hrms_dirs()
    if not dirs:
        results.append(("prova_hr", "SKIP", 0, ["nessuna cartella HRMS (mzlab-dati/HRMS o ../Data/HRMS)"])); return
    for d in dirs:
        rc, out, dt = sh([sys.executable, str(ROOT / "tools" / "prova_hr.py"), str(d)], LOG / "prova_hr.log", 900)
        results.append((f"prova_hr {d.parent.name}/{d.name}", "OK" if rc == 0 and "ERRORE" not in out else "FAIL", dt,
                        [l[:200] for l in out.strip().splitlines()]))


def dam_file() -> Path | None:
    repo = dati_repo()
    name = "Lab_inq_FullMass_pos_max480.dam"            # e2e15 also wants Lab_inq_MRM_Flufe.dam in the same folder
    for cand in (get_env("DAM"), repo / "dam" / name if repo else None, ROOT.parent / "Data" / "dam - Metodi" / name,
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


# --cambiati: which e2e cover which files. A changed file that matches no rule (the core: explore.js, index.html, app.py, api.py,
# explore.py, tabs.js, ...) means "all of them". Documents only: no e2e. Keep it short and update it with a new e2e of a new area.
AREE = [
    ("mzlab/web/teoria/", {"e2e7", "e2e_pratica", "e2e_telefono", "e2e_header", "e2e_nome", "e2e_teoria_scura", "e2e_tema_lampo", "e2e_accessibilita", "e2e_teoria_mobile", "e2e_indirizzi", "e2e_teoria_cerca", "e2e_teoria_strumenti", "e2e_teoria_nav"}),
    ("mzlab/web/tavola-periodica.js", {"e2e28", "e2e6", "e2e_teoria_strumenti"}),
    ("mzlab/web/draw.js", {"e2e_rifiniture", "e2e24", "e2e6", "e2e_decimali", "e2e_ketcher_grandi", "e2e_strumenti_ketcher", "e2e_tocco", "e2e_tocco_disegno", "e2e_lingua_disegno"}),
    ("tools/tema_testa.py", {"e2e_tema_lampo"}), ("mzlab/web/telefono.js", {"e2e_telefono"}),
    ("mzlab/web/telefono-scheda.js", {"e2e_telefono", "e2e_indirizzi"}),
    ("tools/build_site.py", set()), ("tests_e2e/e2e_indirizzi.py", {"e2e_indirizzi"}), ("tests_e2e/e2e_teoria_mobile.py", {"e2e_teoria_mobile"}),
    ("mzlab/web/tables.js", {"e2e28", "e2e_perdite", "e2e6"}), ("mzlab/web/elements.js", {"e2e28", "e2e6"}),
    ("mzlab/web/perdite.js", {"e2e_perdite"}), ("mzlab/web/calcola.js", {"e2e_calc"}), ("mzlab/web/ordine.js", {"e2e_ordine", "e2e_lista_hr"}), ("mzlab/web/conversione.js", {"e2e_conversione"}), ("mzlab/web/cromato.js", {"e2e_cromato"}),
    ("mzlab/web/visualizza.js", {"e2e_hr_display", "e2e_hr_xic", "e2e_spettro_hr"}), ("mzlab/web/hr.js", {"e2e_hr_base", "e2e_hr_ppm", "e2e_hr_xic", "e2e_hr_ui", "e2e_hr_display", "e2e_hr_isotopi"}), ("mzlab/web/dda.js", {"e2e_hr_dda", "e2e_hr_dda2", "e2e_hr_nearest", "e2e_hr_vista", "e2e_spettro_hr"}), ("mzlab/web/banco.js", {"e2e_intestazioni", "e2e_hr_vista", "e2e_banco_barra", "e2e_banco_celle", "e2e_hr_pannelli", "e2e_banco_info", "e2e_hr_display", "e2e_hr_isotopi"}), ("mzlab/web/liste.js", {"e2e_contaminanti", "e2e_hr_pannelli", "e2e_spettro_hr"}), ("tests_e2e/e2e_hr_pannelli.py", {"e2e_hr_pannelli"}), ("mzlab/web/elementare.js", {"e2e_hr_formule", "e2e_hr_composizione", "e2e_hr_pannelli"}), ("mzlab/web/composizione.js", {"e2e_hr_formule", "e2e_hr_composizione", "e2e_hr_msntree"}), ("tests_e2e/e2e_hr_formule.py", {"e2e_hr_formule"}), ("mzlab/web/visualizza.js", {"e2e_hr_display", "e2e_banco_barra"}), ("tests_e2e/e2e_hr_display.py", {"e2e_hr_display"}), ("mzlab/chem/composition.py", {"e2e_hr_composizione", "e2e_hr_formule"}), ("mzlab/chem/isoscore.py", {"e2e_hr_composizione", "e2e_hr_formule"}), ("mzlab/chem/subformulas.py", {"e2e_hr_vista"}),
    ("mzlab/reader/profile.py", {"e2e_hr_base", "e2e_hr_ppm", "e2e_hr_xic", "e2e_hr_dda", "e2e_hr_ui"}),
    ("mzlab/web/libreria", {"e2e_libreria", "e2e_hr_identificazione", "e2e_libreria_wasm", "e2e_libreria_cartella", "e2e_libreria_scarica"}), ("mzlab/web/livelli", {"e2e_libreria", "e2e_hr_identificazione", "e2e_ricerca_ms2"}), ("mzlab/web/touch.js", {"e2e_tocco", "e2e_tocco_disegno"}), ("tests_e2e/e2e_cursore.py", {"e2e_cursore"}), ("mzlab/web/perf.js", {"e2e_perf"}),
    ("mzlab/web/origine.js", {"e2e_origine"}), ("mzlab/ionfamily.py", {"e2e_origine"}),
    ("mzlab/web/aggiorna.js", {"e2e_rifiniture"}), ("mzlab/web/settings.js", {"e2e_rifiniture", "e2e22", "e2e25", "e2e_lingua"}), ("mzlab/web/lang/", {"e2e_lingua"}), ("mzlab/web/i18n.js", {"e2e_lingua"}),
    ("mzlab/web/sottrazione.js", {"e2e_sottrazione"}), ("mzlab/differenza.py", {"e2e_sottrazione"}),
    ("tools/controlla_i18n.py", set()), ("tools/controlla_teoria.py", set()), ("tools/nuovo_capitolo.py", set()), ("mzlab/web/spettro.js", {"e2e_spettro", "e2e_assi"}),
    ("mzlab/web/mappa.js", {"e2e_mappa", "e2e_mappa3d", "e2e_pannelli2", "e2e_mappa_mirino", "e2e_mappa_nav", "e2e_mappa_nav3d", "e2e_mappa_estrai", "e2e_mappa_estrai_hr", "e2e_mappa_correzione"}), ("mzlab/web/pannelli.js", {"e2e_mappa_correzione", "e2e_intestazioni"}), ("mzlab/web/mappa-estrai.js", {"e2e_mappa_estrai", "e2e_mappa_estrai_hr"}), ("tests_e2e/e2e_mappa_estrai.py", {"e2e_mappa_estrai"}), ("tests_e2e/e2e_mappa_estrai_hr.py", {"e2e_mappa_estrai_hr"}), ("tests_e2e/e2e_mappa_correzione.py", {"e2e_mappa_correzione"}), ("tests_e2e/e2e_mappa3d_errori.py", {"e2e_mappa3d_errori"}), ("mzlab/web/reset.js", {"e2e_reset"}), ("tests_e2e/e2e_reset.py", {"e2e_reset"}), ("mzlab/web/settings.js", {"e2e_reset"}), ("tests_e2e/e2e_mappa_nav.py", {"e2e_mappa_nav"}), ("mzlab/web/mirino.js", {"e2e_mappa", "e2e_mappa_mirino"}), ("tests_e2e/e2e_mappa_mirino.py", {"e2e_mappa_mirino"}), ("mzlab/web/mappa3d.js", {"e2e_mappa3d", "e2e_mappa_nav3d", "e2e_mappa3d_errori"}), ("tests_e2e/e2e_mappa_nav3d.py", {"e2e_mappa_nav3d"}), ("tests_e2e/e2e_mappa3d.py", {"e2e_mappa3d"}), ("tests_e2e/e2e_mappa.py", {"e2e_mappa"}),
    ("mzlab/web/scroll.js", {"e2e_scroll", "e2e8", "e2e_cursore", "e2e_asse_x"}), ("mzlab/web/spettro.js", {"e2e_asse_x", "e2e_spettro"}), ("tests_e2e/e2e_asse_x.py", {"e2e_asse_x"}), ("mzlab/web/xlsx.js", {"e2e6", "e2e8", "e2e15", "e2e18"}),
    ("mzlab/web/molui.js", {"e2e_molui", "e2e_tpmine2"}),
    ("mzlab/web/tpmine-loader.js", {"e2e_tpmine1", "e2e_tpmine2"}), ("TP_Mine/", {"e2e_tpmine1", "e2e_tpmine2", "e2e_tpmine_mem", "e2e_tpmine_albero", "e2e_tpmine_rtalign", "e2e_tpmine_ms2", "e2e_untarget", "e2e_identita"}),
    ("mzlab/web/browser", {"e2e13", "e2e_rust", "e2e_lr", "e2e_avvio", "e2e_mzidx", "e2e_pool"}), ("mzlab/web/lr", {"e2e_lr"}), ("mzlab/web/archivio", {"e2e_mzidx", "e2e_pool"}), ("mzlab/web/idx", {"e2e_pool"}), ("mzlab/reader/mzidx", {"e2e_mzidx"}), ("mzlab/web/rust-", {"e2e_rust", "e2e_lr"}), ("mzlab/web/qualita", {"e2e_picchi_qualita"}), ("mzlab/web/peaks-worker", {"e2e_picchi_qualita"}), ("tests_e2e/e2e_picchi_qualita.py", {"e2e_picchi_qualita"}), ("crates/", {"e2e_rust", "e2e_libreria_wasm"}), ("mzlab/web/sw.js", {"e2e13", "e2e_avvio"}), ("mzlab/browser.py", {"e2e13", "e2e_mzidx"}), ("tools/build_site.py", {"e2e13"}),
    ("tools/genera_", set()), ("tools/prova_hr.py", set()), ("tools/validate_ionfamily.py", set()), ("tests/", set()),
    ("mzlab/web/tour.js", {"e2e_tour"}),
    ("mzlab/web/paletta.js", {"e2e_strumenti"}), ("mzlab/web/barra.js", {"e2e_strumenti", "e2e_pannello", "e2e_cassetto", "e2e_barra_hint", "e2e_barra", "e2e_calc", "e2e_perdite", "e2e_contaminanti", "e2e_banco_info", "e2e_tour", "e2e_spettro_hr"}), ("tests_e2e/e2e_strumenti.py", {"e2e_strumenti"}), ("tests_e2e/e2e_spettro_hr.py", {"e2e_spettro_hr"}),
    ("mzlab/web/pannelli.js", {"e2e_sottrazione", "e2e_intestazioni", "e2e_intestazione_cromato", "e2e_header", "e2e_pannelli", "e2e_pannelli2", "e2e_fisso", "e2e_blocco1", "e2e_blocco2", "e2e_blocco3", "e2e_tour", "e2e_tocco", "e2e_cursore", "e2e_asse_x"}),
    ("mzlab/web/grafici.js", {"e2e_sottrazione", "e2e_cromato", "e2e_mappa", "e2e_mappa3d", "e2e_nitido", "e2e_assi", "e2e_integrazioni", "e2e_picchi", "e2e_tour", "e2e_asse_x"}),
    ("mzlab/web/picchi.js", {"e2e_picchi_qualita", "e2e_picchi", "e2e_integrazioni", "e2e_spettro", "e2e_contaminanti", "e2e_assi", "e2e_tour", "e2e_cursore", "e2e_spettro_hr"}),
    ("mzlab/web/eventi.js", {"e2e_calc", "e2e_scroll", "e2e_spettro", "e2e_cromato", "e2e_tocco", "e2e_barra", "e2e_tour", "e2e_cursore", "e2e_spettro_hr"}),
    ("mzlab/web/qaqc.js", {"e2e_qaqc", "e2e_qaqc_dormiente", "e2e_tour"}), ("mzlab/web/features.js", {"e2e_qaqc", "e2e_qaqc_dormiente", "e2e_strumenti"}), ("mzlab/web/profilo.js", {"e2e_statistica"}), ("mzlab/web/tabella.js", {"e2e_statistica"}), ("mzlab/web/bianco.js", {"e2e_statistica"}), ("mzlab/web/riferimento.js", {"e2e_differenza_hr", "e2e_tour"}), ("tests_e2e/e2e_differenza_hr.py", {"e2e_differenza_hr"}), ("mzlab/web/pca.js", {"e2e_statistica"}), ("mzlab/statistica.py", {"e2e_statistica"}), ("tools/dati_sintetici.py", {"e2e_statistica"}), ("mzlab/web/stat.js", {"e2e_statistica", "e2e_strumenti", "e2e_tour"}), ("mzlab/web/campioni.js", {"e2e_statistica", "e2e_workflow"}), ("tests_e2e/e2e_statistica.py", {"e2e_statistica"}), ("mzlab/web/rapporto.js", {"e2e_qaqc", "e2e_workflow"}), ("mzlab/qaqc.py", {"e2e_qaqc"}),
    ("mzlab/web/workflow.js", {"e2e_workflow"}), ("mzlab/web/gradiente.js", {"e2e_gradiente", "e2e_workflow"}), ("mzlab/web/tdati.js", {"e2e_tabelle_dati"}), ("mzlab/web/tdati-pannelli.js", {"e2e_tabelle_dati"}), ("tests_e2e/e2e_tabelle_dati.py", {"e2e_tabelle_dati"}), ("mzlab/web/tdati.js", {"e2e_axe_grafici"}), ("mzlab/web/mappa.js", {"e2e_tabelle_dati"}), ("tests_e2e/e2e_axe_grafici.py", {"e2e_axe_grafici"}), ("mzlab/analog.py", {"e2e_gradiente"}), ("tests_e2e/e2e_gradiente.py", {"e2e_gradiente"}), ("mzlab/web/fair.js", {"e2e_workflow", "e2e_fair"}), ("tests_e2e/e2e_fair.py", {"e2e_fair"}), ("tests/test_fair.py", {"e2e_fair"}), ("mzlab/workflow.py", {"e2e_workflow"}), ("mzlab/web/version.js", {"e2e_workflow"}),
]
DOCS = (".md", ".txt", "LICENSE", ".github/", ".gitignore", ".gitattributes", "pyproject.toml")


def changed_tests(files: set[str] | None = None) -> tuple[set[str] | None, str]:
    """E2E to run for the files changed against origin/main (committed or not): (names, explanation); None = all; {"-"} = none."""
    def git(*a):
        p = subprocess.run(["git", *a], cwd=ROOT, capture_output=True, text=True)
        return p.stdout.split("\n") if p.returncode == 0 else []
    if files is None:
        files = {f for f in git("diff", "--name-only", "origin/main...HEAD") + [l[3:] for l in git("status", "--porcelain")] if f.strip()}
    if not files:
        return {"-"}, "nessun file cambiato rispetto a origin/main: nessun e2e"
    tests: set[str] = set()
    for f in sorted(files):
        if f.endswith(DOCS) or any(f.startswith(d) for d in DOCS if d.endswith("/")):
            continue
        if f.startswith("tests_e2e/"):
            n = Path(f).stem
            if n in ("lib", "lib_hr", "synth"):
                return None, f"{f} è un aiuto comune: tutti gli e2e"
            if n.startswith("e2e"):
                tests.add(n)
            continue
        hit = [t for pre, t in AREE if f.startswith(pre)]
        if not hit:
            return None, f"{f} è del nucleo: tutti gli e2e"
        for t in hit:
            tests |= t
    if not tests:
        return {"-"}, "solo documenti o file senza e2e: nessun e2e"
    return tests | ({"e2e3"} if any(f.startswith("mzlab/") for f in files) else set()), "e2e: " + ", ".join(sorted(tests))


def last_failed() -> set[str]:
    """Names of the e2e that were FAIL in the previous run, read from .verifica/ultimo.md (the lines «- FAIL e2e6 (50 s): ...»)."""
    try:
        text = (OUT / "ultimo.md").read_text(encoding="utf-8")
    except OSError:
        return set()
    return {m.group(1) for m in re.finditer(r"^- FAIL +(e2e\w*)", text, re.M) if (E2E / f"{m.group(1)}.py").exists()}


def e2e(results, only, timeout, kind, jobs: int = 1) -> None:
    names = sorted(p.stem for p in E2E.glob("e2e*.py") if p.stem not in NOT_TESTS)
    if only:
        names = [n for n in names if n in only]
    if not browser_ok():
        results.append(("e2e", "SKIP", 0, ["Playwright/Chromium non disponibili: python3 tools/verifica.py --setup"])); return
    mz, src = data_dir()
    dam = dam_file()
    env = dict(os.environ, MZLAB_MZML=str(mz), QQQ_MZML=str(mz), PYTHONUNBUFFERED="1")
    if dam:
        env["MZLAB_DAM"] = str(dam)
        env["QQQ_DAM"] = str(dam)
    if jobs > 1:
        env["MZLAB_E2E_PARALLEL"] = "1"
        env["QQQ_E2E_PARALLEL"] = "1"          # tests_e2e/lib.py: every test takes a free port and its own work folder
    todo, out_by = [], {}
    for n in names:
        need = NEEDS.get(n, set())
        why = ("serve un file .dam (MZLAB_DAM / QQQ_DAM)" if "dam" in need and not dam else
               "solo con i dati veri del laboratorio" if "veri" in need and src != "veri" else
               "costruisce il sito con Pyodide: lancialo a parte (--solo)" if "sito" in need and not only else "")
        if why:
            out_by[n] = (n, "SKIP", 0, [why])
        else:
            todo.append(n)
    def one(n):
        rc, out, dt = sh([sys.executable, str(E2E / f"{n}.py")], LOG / f"{n}.log", timeout, env=env, cwd=ROOT)
        probs = judge(out, rc)
        return (n, "FAIL" if probs else "OK", dt, probs[:4])
    # the site tests (they build or serve the site on fixed ports) always run alone, after the others
    alone = [n for n in todo if "sito" in NEEDS.get(n, set())]
    from concurrent.futures import ThreadPoolExecutor
    with ThreadPoolExecutor(max_workers=max(1, jobs)) as ex:
        for r in ex.map(one, [n for n in todo if n not in alone]):
            out_by[r[0]] = r
    for n in alone:
        out_by[n] = one(n)
    results.extend(out_by[n] for n in names)
    results.append(("dati usati per gli e2e", "INFO", 0, [f"{src}: {mz}" + (f" · .dam: {dam.name}" if dam else " · nessun .dam")]))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--rapida", action="store_true", help="sintassi + pytest + e2e3")
    ap.add_argument("--solo", default="", help="solo questi e2e, separati da virgola (es. e2e6,e2e18)")
    ap.add_argument("--senza-e2e", action="store_true")
    ap.add_argument("--browser", choices=("chromium", "firefox", "webkit"), help="browser dei test e2e (predefinito chromium; webkit = il motore di Safari)")
    ap.add_argument("--fumo", action="store_true", help="solo gli e2e di fumo (SMOKE), senza sintassi JS e pytest: è il giro del job «browser» della CI")
    ap.add_argument("--teoria", action="store_true", help="in più, il controllo dei capitoli della Teoria (tools/controlla_teoria.py; con MZLAB_DATI anche nomi vietati e copia dalle fonti)")
    ap.add_argument("--setup", action="store_true", help="installa pytest, playwright e chromium se mancano")
    ap.add_argument("--timeout", type=int, default=600, help="secondi per ogni e2e (predefinito 600)")
    ap.add_argument("--paralleli", type=int, default=max(1, min(6, (os.cpu_count() or 2) - 1)),
                    help="e2e eseguiti insieme (predefinito: CPU - 1, al massimo 6; 1 = uno alla volta)")
    ap.add_argument("--ultimi-falliti", action="store_true", help="rilancia solo gli e2e che erano FAIL nel giro precedente (.verifica/ultimo.md)")
    ap.add_argument("--cambiati", action="store_true", help="solo gli e2e che riguardano i file cambiati rispetto a origin/main (vedi AREE); con file del nucleo li fa tutti")
    ap.add_argument("--tutto", action="store_true", help="stampa anche le righe OK (altrimenti solo FAIL/SKIP e il conteggio)")
    a = ap.parse_args()
    if a.browser:
        os.environ["MZLAB_BROWSER"] = a.browser
        os.environ["QQQ_BROWSER"] = a.browser
    LOG.mkdir(parents=True, exist_ok=True)
    if a.setup:
        setup()
    results: list = []
    t0 = time.time()
    if not a.fumo and not a.ultimi_falliti:
        js_syntax(results)
        i18n(results)
        codice(results)
        if a.teoria:
            teoria(results)
        pytest(results, 1200)
        if not a.rapida:
            prova_hr(results)
    if not a.senza_e2e:
        only = {s.strip().removesuffix(".py") for s in a.solo.split(",") if s.strip()} or (set(SMOKE) if a.rapida or a.fumo else set())
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
            e2e(results, only, a.timeout, "", a.paralleli)
    noti = noti_rossi()
    results = [(n, "NOTO", dt, ["known: " + noti[n]] + notes[:1]) if st == "FAIL" and n in noti else
               (n, "FAIL", dt, (["NUOVO"] if n.startswith("e2e") else []) + notes) if st == "FAIL" else (n, st, dt, notes)
               for n, st, dt, notes in results]
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
    if a.tutto:
        print(text)
    else:          # fewer tokens for whoever reads it: only what is not OK, plus how many are OK (the full list is in .verifica/ultimo.md)
        ok = [r for r in results if r[1] == "OK"]
        print("\n".join([lines[0], ""] + [l for l in lines[2:] if not l.startswith("- OK")] +
                         [f"OK: {len(ok)} controlli (elenco in .verifica/ultimo.md)"]))
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
