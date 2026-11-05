"""Members of a maker's file left in the browser storage by a copy that never finished (a tab closed during a .wiff upload, synthetic tools/wiff_misto.py
pair and stand-in members; no laboratory data): at the next start the engine removes the `nat~` members no sample points to and says «loading of X was
interrupted, load it again»; members another tab is still copying (it holds the Web Lock "mzlab-nat:<dataset>") stay; with no such members nothing is
said; the samples already loaded keep their members and stay readable.
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_nativi_orfani.py"""
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="nat_orfani_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_misto.py"), str(D)], check=True, capture_output=True)
PAIR = [str(D / "misto.wiff"), str(D / "misto.wiff.scan")]
N = len(json.loads((D / "tipi.json").read_text()))
INIT = "try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
# every short notice of the page, kept (the element hides itself after a few seconds)
NOTES = "new MutationObserver(() => { const e = document.getElementById('rust-note'); if (e && !e.hidden) (window.__notes = window.__notes || []).includes(e.textContent) || window.__notes.push(e.textContent); }).observe(document.documentElement, { subtree: true, childList: true, characterData: true, attributes: true })"
DONE = f"ST.files.length === {N} && ST.files.every(f => f.kind) && UPBUSY === 0 && OPENNOW.busy() === 0"
FILES = "async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('files'); const o = []; for await (const k of d.keys()) o.push(k); return o.sort(); }"
PUT = """async n => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('files');
  const w = await (await d.getFileHandle(n, { create: true })).createWritable(); await w.write(new Uint8Array(64)); await w.close(); }"""
HOLD = "k => { window.__free = null; return new Promise(ok => navigator.locks.request('mzlab-nat:' + k, { mode: 'shared' }, () => { ok(); return new Promise(r => { window.__free = r; }); })); }"
X, Y = "nat~x.wiff-aaaaaa~x.wiff", "nat~y.wiff-bbbbbb~y.wiff"

r = Run(port=8830 + 151, wd="/tmp/wd_nat_orfani")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); c = b.new_context(viewport={"width": 1400, "height": 900}); c.set_default_timeout(120000); c.add_init_script(INIT)
        c.add_init_script("document.addEventListener('DOMContentLoaded', () => { " + NOTES + " })")
        def opened(pg):
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#drop", state="attached"); attendi(pg, "window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=120000)
            return pg
        A = opened(c.new_page())
        A.set_input_files("#pick", PAIR); attendi(A, DONE, timeout=120000)
        kept = [n for n in A.evaluate(FILES) if n.startswith("nat~")]
        step("the pair is loaded and its members are in the storage", lambda: (len(kept) == 2) or (_ for _ in ()).throw(AssertionError(A.evaluate(FILES))))
        # two orphan datasets: x nobody copies, y another tab (A) is copying now
        A.evaluate(PUT, X); A.evaluate(PUT, Y); A.evaluate(HOLD, "y.wiff-bbbbbb")
        B = opened(c.new_page())
        msg = lambda pg, n: pg.evaluate("n => I18N.t('load.natInterrupted', { name: n })", n)
        def two_tabs():
            attendi(B, "(window.__notes || []).length > 0", timeout=30000)
            have = B.evaluate(FILES)
            assert X not in have, have
            assert Y in have, ("a member another tab is copying was removed", have)
            assert all(k in have for k in kept), ("a member a sample uses was removed", have)
            notes = B.evaluate("window.__notes")
            assert notes == [msg(B, "x.wiff")], notes
        step("two tabs: the orphan nobody copies goes and is named; the one another tab is copying stays", two_tabs)
        step("the samples already loaded keep their members and stay readable", lambda: attendi(B, f"ST.files.length === {N}", timeout=60000) or None)
        A.evaluate("window.__free()")
        B.close(); B = opened(c.new_page())
        def after_release():
            attendi(B, "(window.__notes || []).length > 0", timeout=30000)
            assert Y not in B.evaluate(FILES), B.evaluate(FILES)
            assert B.evaluate("window.__notes") == [msg(B, "y.wiff")], B.evaluate("window.__notes")
        step("once the other tab lets go, the next start removes its orphan and names it", after_release)
        B.close(); B = opened(c.new_page())
        def silent():
            attendi(B, f"ST.files.length === {N}", timeout=60000)
            B.wait_for_timeout(1500)
            assert not B.evaluate("(window.__notes || []).filter(t => t === I18N.t('load.natInterrupted', { name: 'x.wiff' }) || /x\\.wiff|y\\.wiff|misto/.test(t))"), B.evaluate("window.__notes")
            assert all(k in B.evaluate(FILES) for k in kept), B.evaluate(FILES)
        step("no orphan: nothing said, nothing removed", silent)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
bad = [n for n, s in steps if s != "ok"]
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report() if bad else None
sys.exit(1 if bad else 0)
