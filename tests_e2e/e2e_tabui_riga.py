"""Tabui, the detail of a row (TP_Mine/js/44_tp_riga.js) on the low-resolution Dig of a synthetic series (TP_Mine/tests/lr_synth.py) with an MS2 file,
all opened in the Data view: the head is the summary of the closed row (no «Decided: —», no level sentence), no «RT … extracted … observed» line,
no «Data» button, no «Copy text» and no sentence under the charts (they are the help of the chart); «XIC in Data» and «MS2 in Data» reach the Data
view with the XIC and the MS2 spectrum already open; Identity is one compact line per candidate with «Neutral formula», «[M+H]+ formula», Δm/z and the
proofs as icons, «Details» closed; every element that is not plain has its help (data-tt); nothing overflows at 360 px. Builds the site and
static/wasm-feat (SKIP without cargo, wasm-bindgen or numpy); MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses one. Run from the repo root."""
import os, re, shutil, subprocess, sys, tempfile, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build the wasm of the engine and of mzFinder"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)
sys.path.insert(0, str(ROOT / "TP_Mine" / "tests")); import lr_synth  # noqa: E402

tmp = Path(tempfile.mkdtemp(prefix="tabui_riga_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
else:                                                      # fresh sources of mzFinder over the site that was built
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
series = lr_synth.write_series(tmp / "ser", times=(0, 15, 30, 60), blank=False)
EXPS = [{"precursor": 257.06, "ce": 10.0, "peaks": [(7.4, 0.07, 6e5)]}]                       # the first +O isomer (RT 7.4)
ms2 = lr_synth.write_ms2(tmp / "ser", EXPS, 30.0, name="lr_MS2-t30.mzML")
paths = [str(f["path"]) for f in series] + [str(tmp / "ser" / ms2["name"])]
SMILES = "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"
GONE = ("Decided: —", "Level 4", "proposed formula", "Derives from", "extracted", "Copy text", "In words")      # the sentences the detail no longer has
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).close(); break
    except Exception: srv.poll()
# the text of the open row without the descriptions of the charts for screen readers (`.sr-only`: the accessible copy of their help)
SEEN = "(() => { const c = document.querySelector('#tp-det').cloneNode(true); c.querySelectorAll('.sr-only').forEach(e => e.remove()); return c.textContent; })()"
# elements of the open row wider than the row or the window, outside the containers that scroll on their own
OVER = """(() => { const d = document.querySelector('#tp-det'), R = Math.min(d.getBoundingClientRect().right, innerWidth) + 1;
  const scrolls = e => { if (e.closest('.sr-only')) return true; for (let p = e.parentElement; p && p !== d; p = p.parentElement) { const o = getComputedStyle(p).overflowX; if (o === 'auto' || o === 'scroll') return true; } return false; };
  return [...d.querySelectorAll('*')].filter(e => e.getClientRects().length && e.getBoundingClientRect().right > R && !scrolls(e)).map(e => e.tagName + '.' + e.className).slice(0, 8); })()"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        ctx = b.new_context(viewport={"width": 1280, "height": 1000}, reduced_motion="reduce")
        pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
        pg.set_input_files("#pick", paths)
        pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(paths)}", timeout=120000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function(f"E.files.length >= {len(paths)} && E.pending.length === 0", timeout=240000)

        def dig():
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length > 1", timeout=60000)
            try: pg.wait_for_function("document.querySelector('#tp-tol').classList.contains('tp-est')", timeout=60000)
            except Exception: pass
            pg.fill("#tp-mol", SMILES)
            pg.click("#tp-go"); pg.wait_for_selector("#tp-t", timeout=360000)
            pg.click('.vt[data-v="tab"]'); pg.wait_for_selector("#tp-t tr.clk", timeout=60000)
        step("the Dig of the synthetic series gives the table", dig)

        def open_ms2_row():                                # the row whose detail has a representative MS2: two buttons above the charts
            ids = pg.evaluate("[...document.querySelectorAll('#tp-t tr.clk')].map(r => r.dataset.id)")
            for i in ids[:20]:
                pg.click(f'#tp-t tr.clk[data-id="{i}"]'); pg.wait_for_selector("#tp-det .tpr-ov svg", timeout=60000)
                try: pg.wait_for_function("document.querySelectorAll('#tp-det .tpr-go').length === 2", timeout=4000); return
                except Exception: pg.click(f'#tp-t tr.clk[data-id="{i}"]')      # closes it
            raise AssertionError("no row with the MS2 of the file")
        step("a row with MS2 opens", open_ms2_row)

        def gone():
            t = pg.evaluate(SEEN)
            assert not [g for g in GONE if g in t], [g for g in GONE if g in t]
            assert not re.search(r"RT [\d.]+ min \([\d.]+-[\d.]+\)", t), "no RT line under the charts"
            assert pg.locator("#tp-det .tdbtn, #tp-det .tpr-line, #tp-det #tpr-ms2-text, #tp-det .tpr-words").count() == 0
            assert pg.locator("#tp-det .tpt-head .tpr-sum").count() >= 3, pg.inner_text("#tp-det .tpt-head")
            assert pg.locator("#tp-det .tpr-go").all_inner_texts() == ["XIC in Data", "MS2 in Data"]
            tip = " ".join(pg.locator("#tp-det .tpr-ov svg").evaluate_all("l => l.map(s => s.getAttribute('data-tt-text') || '')"))
            assert "extracted" in tip and "Representative MS2" in tip, tip[:400]           # what the removed lines said is the help of the charts
        step("the head is the summary of the closed row; the removed sentences are nowhere, their content is in the help of the charts", gone)

        def identity():
            pg.click('#tp-det [data-tab="identity"]'); pg.wait_for_selector("#tp-det .tpr-cand", timeout=60000)
            th = [re.sub(r"\s+", "", x) for x in pg.locator("#tp-det .tpr-cand thead th").evaluate_all("l => l.map(e => e.textContent)")]
            assert "Neutralformula" in th and "[M+H]+formula" in th and "Δm/z(Da)" in th and "MS2" in th and "Formula" not in th, th
            assert pg.locator("#tp-det .tpr-cand .tpr-pk").count() >= 3, th
            row = pg.locator("#tp-det .tpr-cand tbody tr").first.inner_text()
            assert re.search(r"C\d+H\d+.*C\d+H\d+", row), row                              # the neutral and the [M+H]+ formula
            assert pg.get_attribute("#tp-det details.tpr-det", "open") is None and pg.locator("#tp-det .lr-idx").is_hidden(), "Details closed"
            pg.click("#tp-det details.tpr-det > summary"); pg.wait_for_selector("#tp-det .lr-idx", timeout=10000)
            pg.click("#tp-det details.tpr-det > summary")
        step("Identity: one line per candidate with the neutral and [M+H]+ formula, Δm/z and the proofs; Details closed", identity)

        def helps():
            bare = pg.evaluate("""[...document.querySelectorAll('#tp-det .tpr-sum, #tp-det .tpr-go, #tp-det .tpr-cand th > span, #tp-det .tpr-pk, #tp-det details.tpr-det > summary, #tp-det [role=tab]')]
              .filter(e => !e.hasAttribute('data-tt')).map(e => e.className + ':' + e.textContent.trim().slice(0, 20))""")
            assert not bare, bare
            assert pg.evaluate("[...document.querySelectorAll('#tp-det .tpr-pk')].every(e => (e.getAttribute('data-tt-text') || '').length > 5 && e.getAttribute('aria-label'))")
            pg.locator("#tp-det .tpr-pk").first.focus()
            pg.wait_for_function("(() => { const p = document.getElementById('tph-pop'), e = document.querySelector('#tp-det .tpr-pk'); return p && !p.hidden && p.textContent.includes(e.getAttribute('data-tt-text').slice(0, 12)); })()", timeout=5000)
            pg.keyboard.press("Escape")             # the popover of a proof says its own reason
        step("every element that is not plain text has its help (data-tt), the proofs say why", helps)

        def several():
            ids = pg.evaluate("[...document.querySelectorAll('#tp-t tr.clk')].filter(r => r.querySelector('.tp-alt')).map(r => r.dataset.id)")
            if not ids: print("  SKIP no row of the synthetic series has other identities of the same nominal m/z (e2e_tabui_tptab shows several)"); return
            pg.click(f'#tp-t tr.clk[data-id="{ids[0]}"]'); pg.click('#tp-det [data-tab="identity"]'); pg.wait_for_selector("#tp-det .tpr-cand", timeout=60000)
            n = pg.locator("#tp-det .tpr-cand tbody tr").count() + (int(pg.inner_text("#tp-det .tpr-more").strip("+")) if pg.locator("#tp-det .tpr-more").count() else 0)
            assert n >= 2, n
            dm = [x for x in pg.locator("#tp-det .tpr-cand tbody tr td.n").all_inner_texts() if x != "–"]
            assert [abs(float(x)) for x in dm[1:]] == sorted(abs(float(x)) for x in dm[1:]), ("the others ordered by |Δm/z|", dm)
        step("several candidates of the same nominal m/z: one line each, ordered", several)

        def narrow():
            pg.set_viewport_size({"width": 360, "height": 800}); lib.attendi(pg, "innerWidth === 360"); lib.fermo(pg, "#tp-det")
            for tab in ("overview", "identity", "samples", "quality"):
                pg.click(f'#tp-det [data-tab="{tab}"]'); lib.attendi(pg, f"document.querySelector('#tp-det [data-tab={tab}]').getAttribute('aria-selected') === 'true'"); lib.fermo(pg, "#tp-det")
                o = pg.evaluate(OVER); assert not o, (tab, o)
            pg.set_viewport_size({"width": 1280, "height": 1000}); lib.attendi(pg, "innerWidth === 1280")
        step("no overflow at 360 px in the head and the four tabs", narrow)

        def to_data():
            pg.click('#tp-det [data-tab="overview"]'); pg.wait_for_selector("#tp-det .tpr-ov svg", timeout=60000)
            try: pg.wait_for_function("document.querySelectorAll('#tp-det .tpr-go').length === 2", timeout=4000)
            except Exception: pg.click('#tp-t .tpt-tg[aria-expanded="true"]'); open_ms2_row()      # the open row has no MS2: the one that has
            mz = float(re.search(r"m/z\s+([\d.]+)", pg.inner_text("#tp-det .tpt-head")).group(1))
            pg.click("#tp-det .tpr-go >> nth=0")
            try: pg.wait_for_function("E.panels.some(p => p.type === 'xic' && p.el && p.el.offsetParent && (p.traces || []).some(t => Math.abs(t.mz - %r) < 0.6))" % mz, timeout=60000)
            except Exception: raise AssertionError((mz, pg.evaluate("[E.tab, E.panels.map(p => [p.type, p.tab, !!(p.el && p.el.offsetParent), (p.traces || []).map(t => t.mz)]), (document.querySelector('.toast, #toast, [role=status]') || {}).textContent]")))
            assert pg.locator("#tp-det").is_hidden(), "the Data view is shown"
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-det .tpr-go", timeout=60000)
            pg.click("#tp-det .tpr-go >> nth=1")
            try: pg.wait_for_function("E.tab === 'ms2' && tabPanels('ms2').some(q => q.type === 'spec' && Math.abs((q.prec || 0) - 257.06) < 0.6 && q.el.offsetParent)"
                                     " || E.tab === 'infusion' && E.panels.some(q => q.type === 'spec' && q.tab === 'infusion' && q.el.offsetParent && ((E.files[q.k] || {}).ms2_exps || []).some(x => Math.abs(x.prec - 257.06) < 0.6))", timeout=60000)   # a file of one precursor is an infusion
            except Exception: raise AssertionError(pg.evaluate("[E.tab, E.panels.map(p => [p.type, p.tab, p.prec, p.filt, p.k, !!(p.el && p.el.offsetParent)])]"))
        step("«XIC in Data» opens the XIC of the m/z and «MS2 in Data» the MS2 of the precursor, with no other step", to_data)

        def no_miss():
            assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("MZLAB_LR_MISS empty", no_miss)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
