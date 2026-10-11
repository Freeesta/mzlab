import os, subprocess, time, sys, shutil
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
_TRACK = ("(()=>{if(window.__qqF!==undefined)return;window.__qqF=0;window.__qqT=Date.now();const f=window.fetch;"
          "window.fetch=function(...a){window.__qqF++;window.__qqT=Date.now();const done=()=>{window.__qqF--;window.__qqT=Date.now()};"
          "return f.apply(this,a).then(r=>{done();return r},e=>{done();throw e})}})()")
# Short timers pending (<= 0.4 s: debounces, redraws, toasts) and the time of the last DOM change are tracked as well: pg.wait_for_timeout(ms) below returns
# as soon as the page has been quiet for a moment (no fetch, no pending short timer, no DOM change) instead of sleeping for the whole `ms`.
_TRACK2 = ("(()=>{if(window.__qqP!==undefined)return;window.__qqP=0;const ids=new Map(),st=window.setTimeout,ct=window.clearTimeout;"
           "window.setTimeout=function(f,d,...a){d=+d||0;if(d>400||typeof f!=='function')return st.call(this,f,d,...a);window.__qqP++;window.__qqT=Date.now();"
           "const id=st.call(this,function(...b){if(ids.delete(id)){window.__qqP--;window.__qqT=Date.now()}return f.apply(this,b)},d,...a);ids.set(id,1);return id};"
           "window.clearTimeout=function(id){if(ids.delete(id)){window.__qqP--;window.__qqT=Date.now()}return ct.call(this,id)};"
           "new MutationObserver(()=>{window.__qqT=Date.now()}).observe(document,{subtree:true,childList:true,attributes:true,characterData:true})})()")
_TRACK = _TRACK + ";" + _TRACK2
_QUIET = ("(idle)=>{const k=document.getElementById('kframe');if(k&&k.getBoundingClientRect().width>0)return false;"
          "return document.getAnimations().length===0&&(window.__qqF||0)===0&&(window.__qqP||0)===0&&Date.now()-(window.__qqT||0)>=idle}")
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


def ready(pg, timeout=90000, settle=400):
    """Wait until the page is quiet: no loading screen, no request to the server in flight, nothing started or ended for `settle` ms."""
    import time as _t
    t0 = _t.time()
    while True:
        q = pg.evaluate("() => { const l = document.querySelector('#loading'); return (!l || l.hidden) && (window.__qqF || 0) === 0 ? Date.now() - (window.__qqT || 0) : -1; }")
        if q >= settle: return
        if _t.time() - t0 > timeout / 1000: raise TimeoutError("ready: the page is still loading")
        pg.wait_for_timeout(50)
def drawer(pg, tab="file", pin=True):
    """Open a tool of the toolbar drawer (the file list by default). Docked next to the charts. `pin` is kept for old callers."""
    pg.evaluate(f"(()=>{{BARRA.setTab('{tab}',{{noFocus:true,instant:true}})}})()"); pg.wait_for_timeout(300)
def shut(pg, sel):
    """Close a floating window by its × button. In the Dati view the calculator and the reference tables are tabs of the sidebar (no × there): then there is nothing to close."""
    if pg.is_visible(sel): pg.click(sel)
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
# tools/verifica.py runs several tests at once (MZLAB_E2E_PARALLEL=1 / QQQ_E2E_PARALLEL=1): then every Run takes a free port and its own work folder,
# because a few tests share the fixed ones (they were written to run one after the other)
PARALLEL = get_env("E2E_PARALLEL") == "1"
class Run:
    def __init__(s, port=8811, wd="/tmp/wd1", fresh=True, extra=()):
        if PARALLEL:
            port = free_port(); wd = f"{wd}_{port}"
        s.port, s.wd, s.errs = port, wd, []
        if fresh: shutil.rmtree(wd, ignore_errors=True)
        s.srv = subprocess.Popen([sys.executable, str(ROOT / "tools" / "dev_server.py"), "--workdir", wd, "--port", str(port), *extra],
                                 cwd=str(ROOT), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        # ready as soon as the server answers (it was a fixed 2 s wait)
        import urllib.request
        t0 = time.time()
        while time.time() - t0 < 15:
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1).read(); break
            except Exception:
                time.sleep(0.1)
    def page(s, p, tour=False):
        s.b = p.chromium.launch()
        pg = s.b.new_page(viewport={"width": 1500, "height": 2200}, reduced_motion="reduce")
        if not tour:
            pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: s.errs.append(("pageerror", str(e))))
        pg.on("console", lambda m: s.errs.append(("console." + m.type, m.text)) if m.type in ("error", "warning") and "GL Driver Message" not in m.text else None)       # software-GL performance hints of the 3D viewer are not page errors
        pg.on("requestfailed", lambda r: s.errs.append(("reqfail", r.url, r.failure)))
        pg.on("response", lambda r: s.errs.append(("http%d" % r.status, r.url)) if r.status >= 400 else None)
        pg.goto(f"http://127.0.0.1:{s.port}/")
        pg.wait_for_timeout(800)
        return pg
    def teoria(s, pg, name="00-uso.html", wait=800):
        """Text of a Teoria chapter, opened in a second tab of the same browser."""
        q = s.b.new_page(); q.goto(f"http://127.0.0.1:{s.port}/static/teoria/{name}"); q.wait_for_timeout(wait)
        t = q.inner_text("body"); q.close(); return t
    def close(s):
        try: s.b.close()
        except Exception: pass
        s.srv.terminate()
        s.log = s.srv.stdout.read()
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
