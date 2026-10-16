import os, subprocess, time, sys, shutil, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
# The page has a CSP without unsafe-eval: Playwright's wait_for_function evaluates strings with eval, so it is replaced by a polling version that
# goes through page.evaluate (CDP, not blocked). Same signature for the cases used in the tests (expression string, timeout).
def _wff(self, expression, arg=None, polling=None, timeout=30000):
    import time
    t0 = time.time()
    while True:
        try:
            v = self.evaluate("(() => { return (" + expression + "); })()") if isinstance(expression, str) and not expression.strip().startswith(("()", "function")) else self.evaluate(expression, arg)
            if v: return v
        except Exception as e:
            if "Execution context was destroyed" not in str(e) and "Target" not in str(e): last = e
        if (time.time() - t0) * 1000 > timeout: raise TimeoutError("wait_for_function: " + str(expression)[:120])
        self.wait_for_timeout(100)
from playwright.sync_api import Page as _Page
_Page.wait_for_function = _wff
# Every page counts its fetches in flight (window.__qqF) and the time of the last start/end (window.__qqT): ready() below waits for them
# instead of a fixed pause (the fixed pauses were more than half of the time of the e2e).
# A page that replaces window.fetch (browser.js answers `api/...` itself, with the Rust engine) is counted too: the assignment goes through a setter that wraps it.
_TRACK = ("(()=>{if(window.__qqF!==undefined)return;window.__qqF=0;window.__qqT=Date.now();"
          "const wrap=f=>function(...a){window.__qqF++;(window.__qqU=window.__qqU||[]).push(String((a[0]&&a[0].url)||a[0]));window.__qqT=Date.now();const done=()=>{window.__qqF--;window.__qqT=Date.now()};"
          "return f.apply(this,a).then(r=>{done();return r},e=>{done();throw e})};let cur=wrap(window.fetch);"
          "Object.defineProperty(window,'fetch',{configurable:true,get:()=>cur,set:f=>{cur=wrap(f)}})})()")
# The URL of every fetch is kept in window.__qqU (the Rust engine answers `api/...` in the page, so Playwright's request events never see those calls).
# Short timers pending (<= 0.4 s: debounces, redraws, toasts) and the time of the last DOM change are tracked as well: pg.wait_for_timeout(ms) below returns
# as soon as the page has been quiet for a moment (no fetch, no pending short timer, no DOM change) instead of sleeping for the whole `ms`.
_TRACK2 = ("(()=>{if(window.__qqP!==undefined)return;window.__qqP=0;const ids=new Map(),st=window.setTimeout,ct=window.clearTimeout;"
           "window.setTimeout=function(f,d,...a){d=+d||0;if(d>400||typeof f!=='function')return st.call(this,f,d,...a);window.__qqP++;window.__qqT=Date.now();"
           "const id=st.call(this,function(...b){if(ids.delete(id)){window.__qqP--;window.__qqT=Date.now()}return f.apply(this,b)},d,...a);ids.set(id,1);return id};"
           "window.clearTimeout=function(id){if(ids.delete(id)){window.__qqP--;window.__qqT=Date.now()}return ct.call(this,id)};"
           "new MutationObserver(()=>{window.__qqT=Date.now()}).observe(document,{subtree:true,childList:true,attributes:true,characterData:true})})()")
_TRACK = _TRACK + ";" + _TRACK2
_QUIET = ("(idle)=>{const k=document.getElementById('kframe');if(k&&k.getBoundingClientRect().width>0)return false;"
          "return document.getAnimations().length===0&&(window.__qqF||0)===0&&(typeof E==='undefined'||!E.pending||E.pending.length===0)&&(window.__qqP||0)===0&&Date.now()-(window.__qqT||0)>=idle}")
_real_wft = _Page.wait_for_timeout
_STAT = [0, 0]          # milliseconds asked for and really waited by the quiet pauses (MZLAB_E2E_STATS=1 prints them at the end)
import atexit
atexit.register(lambda: print("PAUSE STATS asked %.1f s, waited %.1f s" % (_STAT[0] / 1000, _STAT[1] / 1000)) if get_env("E2E_STATS") else None)
def _quiet_wft(self, ms):
    """Pause of at most `ms`: it ends earlier when the page has been quiet for max(100 ms, ms/10). Pauses of 150 ms or less stay real (the polling loops use them).
    MZLAB_E2E_PAUSE=exact brings back the fixed pauses (to tell a real bug from a missing wait); hold(pg, ms) is a pause that is always real."""
    if ms <= 150 or get_env("E2E_PAUSE") == "exact": return _real_wft(self, ms)
    import time
    idle, t0 = max(100, ms // 10), time.time()
    _STAT[0] += ms
    try:
        _real_wft(self, 60)
        while (time.time() - t0) * 1000 < ms:
            try:
                if self.evaluate(_QUIET, idle): return
            except Exception:
                return _real_wft(self, max(0, ms - int((time.time() - t0) * 1000)))
            _real_wft(self, 40)
    finally:
        _STAT[1] += (time.time() - t0) * 1000
        if get_env("E2E_STATS") == "2" and (time.time() - t0) * 1000 > ms * 0.8:
            try: print("SLOW PAUSE", ms, self.evaluate("[window.__qqF, window.__qqP, Date.now() - window.__qqT]"))
            except Exception: pass
_Page.wait_for_timeout = _quiet_wft
# Files given to the start screen (#pick): the call returns when all of them are in the list (the tests used to sleep and hope), as stage() does.
_real_sif = _Page.set_input_files
def _sif(self, selector, files, **k):
    _real_sif(self, selector, files, **k)
    n = sum(1 for f in files if str(f).lower().endswith(".mzml")) if isinstance(files, (list, tuple)) else 0       # a .dam goes to the other box
    if selector == "#pick" and n:
        try: self.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length>={n}", timeout=20000)
        except TimeoutError: pass
        _tempi(self)
AUTO_TEMPI = True       # False: the test looks at the raw proposals (e2e_ruoli_tempi)
def _tempi(self):
    """Two or more samples: «Open» wants every time written and confirmed (roles and times of the loading table). The tests that are not about it do it here, before they open."""
    if not AUTO_TEMPI: return
    try:
        if not self.evaluate("typeof RUOLI !== 'undefined' && typeof ST !== 'undefined' && !document.getElementById('start').hidden && ST.files.length >= 2"): return
        self.wait_for_function("RUOLI.busy() === 0 && ST.files.every(f => f.prop)", timeout=20000)
        self.evaluate("""() => { let t = 0; ST.files.forEach(f => { if (RUOLI.timed(f)) { if (f.time == null) { t += 10; f.time = t; } } }); renderFiles(); }""")
    except Exception: pass
_real_click = _Page.click
def _click(self, selector, *a, **k):
    if isinstance(selector, str) and (selector == "#opbtn" or "Carica dati" in selector or "Load data" in selector): _tempi(self)
    return _real_click(self, selector, *a, **k)
_Page.click = _click
_real_ast = _Page.add_script_tag
def _add_script_tag(self, *a, **k):
    """The page forbids inline scripts (CSP script-src without 'unsafe-inline'): a file or a text is run with evaluate, as from the console, not as a <script> tag."""
    path, content = k.get("path"), k.get("content")
    if path or content:
        self.evaluate(content if content else open(path, encoding="utf-8").read())
        return None
    return _real_ast(self, *a, **k)
_Page.add_script_tag = _add_script_tag
_Page.set_input_files = _sif
def hold(pg, ms):
    """A pause that is really `ms` long (something that only time can bring, with nothing in the page to wait for)."""
    _real_wft(pg, ms)
from playwright.sync_api import Browser as _Br, BrowserContext as _Ctx
_np, _nc = _Br.new_page, _Br.new_context
def _new_page(self, *a, **k):
    k.setdefault("locale", "it-IT")        # Playwright starts in en-US, which would now give English to every test written in Italian (e2e_inglese uses en-US on purpose)
    pg = _np(self, *a, **k); pg.add_init_script(_TRACK); return pg
def _new_context(self, *a, **k):
    k.setdefault("locale", "it-IT")
    c = _nc(self, *a, **k); c.add_init_script(_TRACK); return c
_Br.new_page, _Br.new_context = _new_page, _new_context
def stage(pg, files, timeout=90000):
    """Give the files to the start screen and wait until ALL of them are in the list: with a fixed pause, on a loaded machine «Carica dati» loaded only the ones staged so far."""
    pg.set_input_files("#pick", [str(f) for f in files])
    pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length>={len(files)}", timeout=timeout)


def attendi(pg, expr, timeout=30000):
    """Wait until the JavaScript expression is truthy: the state-based way to wait (never a fixed pause, never a raised timeout)."""
    pg.wait_for_function(expr, timeout=timeout)


def fermo(pg, sel, ms=300, timeout=30000):
    """Wait until the first element matching `sel` has had the same position and size for `ms` milliseconds (it is not moving or growing any more)."""
    pg.evaluate("window.__fermo = null")
    pg.wait_for_function("function (a) { const e = document.querySelector(a.sel); if (!e) return false; const r = e.getBoundingClientRect(), k = [r.x, r.y, r.width, r.height].map(v => Math.round(v * 10)).join(), t = performance.now(); "
                         "const w = window.__fermo && window.__fermo.sel === a.sel ? window.__fermo : (window.__fermo = {sel: a.sel, k, t}); if (w.k !== k) { w.k = k; w.t = t; } return t - w.t >= a.ms; }",
                         arg={"sel": sel, "ms": ms}, timeout=timeout)


def saved(pg, timeout=60000):
    """Save the view now and wait until the notebook (which a reload resumes from) holds the session: it is written in the background."""
    pg.evaluate("uiSave(true)")
    pg.wait_for_function("fetch('api/notebook').then(r => r.json()).then(n => (n.session || []).length >= 1)", timeout=timeout)


def ctrl_wheel(pg, dx, dy):
    """Ctrl/⌘ + wheel (the pinch of the trackpad): the zoom of the charts; the wheel alone scrolls the page."""
    pg.keyboard.down("Control"); pg.mouse.wheel(dx, dy); pg.keyboard.up("Control")
def ready(pg, timeout=90000, settle=400):
    """Wait until the page is quiet: no loading screen, no request to the server in flight, nothing started or ended for `settle` ms."""
    import time as _t
    t0 = _t.time()
    while True:
        q = pg.evaluate("() => { const l = document.querySelector('#loading'); return (!l || l.hidden) && (window.__qqF || 0) === 0 && (typeof E === 'undefined' || !E.pending || E.pending.length === 0) ? Date.now() - (window.__qqT || 0) : -1; }")
        if q >= settle:
            bad = pg.evaluate("() => (typeof STATO === 'undefined' ? [] : STATO.verifica())")
            if bad: raise AssertionError("page state invariants broken: " + "; ".join(bad))
            return
        if _t.time() - t0 > timeout / 1000: raise TimeoutError("ready: the page is still loading")
        pg.wait_for_timeout(50)
def drawer(pg, tab="file", pin=True):
    """Open a tool of the toolbar drawer (the file list by default). Docked next to the charts. `pin` is kept for old callers."""
    pg.evaluate(f"(()=>{{BARRA.setTab('{tab}',{{noFocus:true,instant:true}})}})()"); pg.wait_for_timeout(300)
def open_kind(pg, kind):
    """As a student does: click, in the file list, the first file of an experiment type ('full', 'ms2', 'mrm'): that type becomes the current one."""
    k = pg.evaluate("kind => (E.files.find(f => f.kind === kind && !f.gone) || {}).k", kind)
    assert k is not None, f"no {kind} file loaded"
    if not pg.is_visible("#flst"): pg.click("#stbar [data-cmd=file]")      # the file list is a drawer: open it first
    pg.wait_for_selector(f"#flst .fl[data-k='{k}'] .nm", state="visible")
    pg.click(f"#flst .fl[data-k='{k}'] .nm")
def shut(pg, sel):
    """Close a floating window by its × button. In the Dati view the calculator and the reference tables are tabs of the sidebar (no × there): then there is nothing to close."""
    if pg.is_visible(sel): pg.click(sel)
def a11y_open(pg):
    """The accessibility submenu of the settings is closed at first: open it before touching font, contrast or palette."""
    pg.evaluate("(d => { if (d) d.open = true; })(document.querySelector('#uip-a11y'))")
def set_pal(pg, v):
    a11y_open(pg); pg.select_option("#uip-pal", v)
def get_env(suffix: str, default=None):
    """Read MZLAB_<suffix> or legacy QQQ_<suffix> (MZLAB_* takes precedence)."""
    return os.environ.get(f"MZLAB_{suffix}", os.environ.get(f"QQQ_{suffix}", default))


# Browser of the tests: MZLAB_BROWSER / QQQ_BROWSER=chromium (default) | firefox | webkit (the engine of Safari); tools/verifica.py --browser sets it.
# Every script asks for `p.chromium`: here it is pointed at the chosen browser, so no test has to change.
BROWSER = get_env("BROWSER", "chromium")
if BROWSER != "chromium":
    from playwright.sync_api import Playwright as _PW
    _PW.chromium = property(lambda self: getattr(self, BROWSER))
FILES = ["B_FullMass-t0", "B_FullMass-t15", "B_FullMass-t60", "B_MS2-t15", "B_MRM-t0"]
# Paths are relative to this folder: code = parent of tests_e2e, mzML = mzlab-dati/mzML (or $MZLAB_MZML / $QQQ_MZML)
HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
def _find_mz():
    env = get_env("MZML")
    if env and Path(env).exists(): return Path(env)
    for base in (ROOT.parent, ROOT.parent.parent, Path.home(), Path("/workspace"), Path("/workspaces"), Path("/repos"), Path("/tmp")):
        for name in ("mzlab-dati", "QqQ-lab-dati"):
            for sub in ("mzML", "Data/mzML"):
                c = base / name / sub
                if (c / "B_FullMass-t0.mzML").exists(): return c
    return ROOT.parent / "esempio_conversione" / "mzml"

def _find_dam():
    env = get_env("DAM")
    if env and Path(env).exists(): return Path(env)
    name = "Lab_inq_FullMass_pos_max480.dam"
    for base in (ROOT.parent, ROOT.parent.parent, Path.home(), Path("/workspace"), Path("/workspaces"), Path("/repos"), Path("/tmp")):
        for rname in ("mzlab-dati", "QqQ-lab-dati"):
            for sub in ("dam", "Data/dam", "Data/dam - Metodi"):
                c = base / rname / sub / name
                if c.exists(): return c
    return ROOT.parent / "QqQ" / "Metodi inquinanti" / name

MZ = _find_mz()
DAM = _find_dam()
SH = str(HERE / "shots") + "/"
os.makedirs(SH, exist_ok=True)
def mz(f): return str(MZ / f"{f}.mzML")
def free_port() -> int:
    import socket
    with socket.socket() as so:
        so.bind(("127.0.0.1", 0)); return so.getsockname()[1]
def copia(src, dst):
    """Copy a file into the shared site/ only when it differs, by an atomic rename: tests that run together (every one makes sure the site has the current sources) never see a half-written file."""
    src, dst = Path(src), Path(dst)
    b = src.read_bytes()
    if dst.exists() and dst.read_bytes() == b: return
    t = dst.with_name(f".{dst.name}.{os.getpid()}.tmp"); t.write_bytes(b); os.replace(t, dst)
def scrivi(dst, text):
    """As copia(), for a text."""
    dst = Path(dst); b = text.encode("utf-8")
    if dst.exists() and dst.read_bytes() == b: return
    t = dst.with_name(f".{dst.name}.{os.getpid()}.tmp"); t.write_bytes(b); os.replace(t, dst)
def build_feat(static, env=None):
    """Give the copy of the site the .wasm of mzFinder: copied from site/static when built there, else tools/build_feat_wasm.sh, one process at a time: it shares cargo's target/ and its artefacts, so parallel tests would clash."""
    import fcntl, subprocess, tempfile
    ready = ROOT / "site" / "static"
    if (ready / "wasm-feat" / "manifest.json").exists():          # already built in the shared site (CI, local runs): copy it instead of building again
        for d in ("wasm-feat", "wasm-lettori"):
            if (ready / d).is_dir(): shutil.copytree(ready / d, Path(static) / d, dirs_exist_ok=True)
        return
    with open(os.path.join(tempfile.gettempdir(), "mzlab-build-feat.lock"), "w") as lk:
        fcntl.flock(lk, fcntl.LOCK_EX)
        subprocess.run(["tools/build_feat_wasm.sh", str(static)], cwd=ROOT, check=True, env=env, capture_output=True)


# tools/verifica.py runs several tests at once (MZLAB_E2E_PARALLEL=1 / QQQ_E2E_PARALLEL=1): then every Run takes a free port and its own work folder,
# because a few tests share the fixed ones (they were written to run one after the other)
PARALLEL = get_env("E2E_PARALLEL") == "1"
class Run:
    def __init__(s, port=8811, wd="/tmp/wd1", fresh=True):
        if PARALLEL:
            port = free_port(); wd = f"{wd}_{port}"
        s.port, s.wd, s.errs = port, wd, []
        if fresh: shutil.rmtree(wd, ignore_errors=True)
        s._log = tempfile.TemporaryFile(mode="w+", encoding="utf-8")       # the built site with the Rust engine (tools/prepara_sito.py): what the students get; a log file, not a pipe: nobody drains a pipe and the server blocks on its own request log when it fills
        s.srv = subprocess.Popen([sys.executable, str(Path(__file__).with_name("servi.py")), str(port), str(ROOT / "site")], cwd=str(ROOT),
                                 stdout=s._log, stderr=subprocess.STDOUT, text=True)
        # ready as soon as the server answers (it was a fixed 2 s wait)
        import urllib.request
        t0 = time.time()
        while time.time() - t0 < 15:
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1).read(); break
            except Exception:
                time.sleep(0.1)
    def page(s, p, tour=False, qaqc=False):
        s.b = p.chromium.launch()
        pg = s.b.new_page(viewport={"width": 1500, "height": 2200}, reduced_motion="reduce")
        if not tour:
            pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
        if qaqc:       # QA/QC is dormant: only this switch (as ?qaqc=1) makes it visible
            pg.add_init_script("try{localStorage.setItem('qqq.dev.qaqc','1');}catch(_){}")
        pg.on("pageerror", lambda e: s.errs.append(("pageerror", str(e))))
        pg.on("console", lambda m: s.errs.append(("console." + m.type, m.text)) if m.type in ("error", "warning") and "GL Driver Message" not in m.text else None)       # software-GL performance hints of the 3D viewer are not page errors
        pg.on("requestfailed", lambda r: s.errs.append(("reqfail", r.url, r.failure)))
        pg.on("response", lambda r: s.errs.append(("http%d" % r.status, r.url)) if r.status >= 400 else None)
        pg.goto(f"http://127.0.0.1:{s.port}/")
        pg.wait_for_timeout(800)
        return pg
    def teoria(s, pg, name="12-dati.html", wait=800):
        """Text of a Teoria chapter, opened in a second tab of the same browser."""
        q = s.b.new_page(); q.goto(f"http://127.0.0.1:{s.port}/static/teoria/{name}"); q.wait_for_timeout(wait)
        t = q.inner_text("body"); q.close(); return t
    def close(s):
        try: s.b.close()
        except Exception: pass
        s.srv.terminate()
        s._log.seek(0); s.log = s._log.read()
        shutil.rmtree(s.wd, ignore_errors=True)       # the uploaded files (80 MB each for the high-resolution ones) are not kept: the disk of a session is limited
    def report(s):
        print("--- SERVER LOG (tail)\n", s.log[-1500:])
        print("--- BROWSER ERRORS", len(s.errs))
        for e in s.errs: print(e)

# ---- .xlsx reading without libraries (zip + XML), plus openpyxl as a second opinion when it is installed
def xlsx_rows(path, n=1):
    """Rows of worksheet n as lists of ('n', float) | ('s', text) | None, read straight from xl/worksheets/sheetN.xml."""
    import zipfile, re
    import xml.etree.ElementTree as ET
    ns = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}
    z = zipfile.ZipFile(path); assert z.testzip() is None
    root = ET.fromstring(z.read(f"xl/worksheets/sheet{n}.xml")); rows = []
    for r in root.iter("{%s}row" % ns["m"]):
        row = {}
        for c in r.findall("m:c", ns):
            col = 0
            for ch in re.match(r"[A-Z]+", c.get("r")).group(): col = col * 26 + ord(ch) - 64
            if c.get("t") == "inlineStr": row[col - 1] = ("s", "".join(t.text or "" for t in c.iter("{%s}t" % ns["m"])))
            else:
                v = c.find("m:v", ns); row[col - 1] = ("n", float(v.text)) if v is not None else None
        rows.append([row.get(i) for i in range(max(row) + 1)] if row else [])
    return rows
def xlsx_sheets(path):
    import zipfile, re
    z = zipfile.ZipFile(path); return re.findall(r'<sheet name="([^"]*)"', z.read("xl/workbook.xml").decode("utf-8"))
def xlsx_openpyxl(path):
    try: import openpyxl
    except ImportError: return None
    import shutil, tempfile
    tmp = os.path.join(tempfile.mkdtemp(), "t.xlsx"); shutil.copy(path, tmp)      # downloads have no extension, openpyxl wants one
    return openpyxl.load_workbook(tmp, data_only=True)


def dlmenu(pg, scope, kind):
    """«Scarica ▾» of a panel: open the menu, then choose the PNG image or the Excel data (kind = 'png' | 'xlsx')."""
    pg.locator(f"{scope} [data-a=dl]").first.click()
    pg.locator("#ctx div", has_text="Immagine" if kind == "png" else "Excel").first.click()


def axe_path():
    """Path of axe.min.js ($AXE_JS or the axe-playwright-python package), None when axe-core is not available."""
    p = os.environ.get("AXE_JS")
    if p and os.path.exists(p): return p
    try:
        import axe_playwright_python
        q = os.path.join(os.path.dirname(axe_playwright_python.__file__), "axe.min.js")
        return q if os.path.exists(q) else None
    except ImportError:
        return None
