"""«File and instrument» and «List of the scans» of the information bar of the bench: one grid for every section (labels column of the same width), «Scans by type»
readable from 320 to 900 px (no cell narrower than 6 ch, count and RT never wrap, no word broken), MS levels with a superscript (copied as «MSn»), empty rows left
out, placeholder software with its tooltip; the scan count in the header, fixed decimals per column. Synthetic Orbitrap file; the layout checks use a file-info answer made up here."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_banco_file.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
J = {"file": "timolol_infusion_MSn.mzML", "start": "2024-05-03T10:12:00Z", "rt": [1.58, 43.5], "mz_range": {"1": [100, 600], "2": [50, 330], "3": [50, 260], "4": [50, 200]},
     "scans": 9000, "levels": {"1": 900, "2": 4000, "3": 3000, "4": 1100}, "instrument": "Orbitrap Fusion", "serial": "", "analyzers": ["orbitrap", "FTMS"],
     "components": ["electrospray ionization", "orbitrap"], "software": ["custom unreleased software tool 1.0.0"], "source_files": [], "conversion": ["MSConvert"],
     "filters": [{"key": "FTMS + ms3 317 > 261 @cid30", "n": 90, "rt0": 1.58, "rt1": 43.5}, {"key": "FTMS + ms4 317 > 261 > 244 @cid30 @cid35", "n": 1234, "rt0": 1.6, "rt1": 43.5},
                 {"key": "msn:317@cid25>261@cid30>244@cid35", "n": 7, "rt0": 2, "rt1": 3}, {"key": "FTMS + c ESI Full ms [100.0000-600.0000]", "n": 900, "rt0": 1.58, "rt1": 43.5}]}
# JS run in the page: layout facts of the made-up answer, measured in the real panel
MEASURE = """() => {
  const b = document.querySelector('#hri-body'), ch = (() => { const t = document.createElement('span'); t.style.cssText = 'position:absolute;visibility:hidden;width:6ch'; b.appendChild(t); const w = t.getBoundingClientRect().width; t.remove(); return w; })();
  const lines = e => { const r = document.createRange(); r.selectNodeContents(e); return new Set([...r.getClientRects()].map(x => Math.round(x.top))).size; };
  // a word of the type name is on one line unless it is wider than the whole name cell (then it may break anywhere)
  const broken = [...document.querySelectorAll('#hri-body .fi-w')].filter(e => lines(e) > 1 && e.getBoundingClientRect().width < e.parentElement.getBoundingClientRect().width - 1).map(e => e.textContent);
  const dt = [...document.querySelectorAll('#hri-body .fi-kv dt')].map(e => Math.round(e.getBoundingClientRect().width));
  return { ch, dt, narrow: [...document.querySelectorAll('#hri-body .fi-tn,#hri-body .fi-c,#hri-body .fi-rt')].filter(e => e.getBoundingClientRect().width < ch - 0.5).length,
    wrapped: [...document.querySelectorAll('#hri-body .fi-c,#hri-body .fi-rt')].filter(e => lines(e) > 1).length, broken, over: b.scrollWidth - b.clientWidth, n: document.querySelectorAll('#hri-body .fi-types li').length };
}"""
D = synth("hrfile")
r = Run(port=8992, wd="/tmp/wd_hrfile")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1200})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        tab = lambda t: (pg.evaluate(f"BARRA.setTab('{t}')"), frames(pg, 3))
        def real():
            tab("fil"); pg.wait_for_selector("#hri-body .fi-kv", timeout=15000)
            dt = pg.evaluate("[...document.querySelectorAll('#hri-body .fi-kv dt')].map(e=>Math.round(e.getBoundingClientRect().width))"); assert len(dt) >= 4 and len(set(dt)) == 1, dt
            assert pg.evaluate("[...document.querySelectorAll('#hri-body .fi-kv')].map(e=>Math.round(e.getBoundingClientRect().left)).every((v,i,a)=>v===a[0])"), "the same grid in every section"
            assert pg.evaluate("document.querySelectorAll('#hri-body .fi-types li').length") >= 1 and "Orbitrap Exploris 120" in pg.inner_text("#hri-body")
            assert pg.evaluate("document.querySelectorAll('#hri-body .fi-kv sup').length") >= 2, "MS levels with a superscript"
        step("file and instrument: one grid for every section, superscript levels, scans by type as a list", real)
        def mock():
            for theme in ("light", "dark"):
                pg.evaluate(f"document.documentElement.dataset.theme='{theme}'")
                for w in (320, 420, 900):
                    pg.add_style_tag(content=f"#dfiles{{width:{w}px!important;min-width:{w}px!important}}")
                    pg.evaluate("j=>{document.querySelector('#hri-body').innerHTML=BFIL.fileHtml(j)}", J); frames(pg, 2)
                    m = pg.evaluate(MEASURE); ctx = (theme, w, m)
                    assert m["n"] == 4 and m["narrow"] == 0 and m["wrapped"] == 0 and not m["broken"] and m["over"] <= 1 and len(set(m["dt"])) == 1, ctx
        step("scans by type from 320 to 900 px, light and dark: no cell under 6 ch, count and RT on one line, no word broken", mock)
        def texts():
            pg.evaluate("j=>{document.querySelector('#hri-body').innerHTML=BFIL.fileHtml(j)}", J)
            t = pg.evaluate("document.querySelector('#hri-body').innerText"); tc = pg.evaluate("document.querySelector('#hri-body').textContent")
            assert "Serial number" not in t and "Numero di serie" not in t, "an empty row is left out"
            assert "Orbitrap, FTMS" in t and "Electrospray ionization, Orbitrap" in t and "Orbitrap Fusion" in t, t
            assert "MS3: 3000" in tc and "MS1: 900" in tc and "MS4: 50.00" in tc, "plain MSn when copied"
            assert pg.evaluate("document.querySelector('#hri-body .fi-ph').title") != "" and "custom unreleased software tool 1.0.0" in t
            assert "RT 1.58–43.50 min" in t and "1234" in t
        step("empty serial left out, names capitalised (FTMS untouched), placeholder software with its tooltip, copy gives MSn", texts)
        def lst():
            tab("lst"); pg.wait_for_selector("#hri-list .r", timeout=15000)
            h = pg.inner_text("#sb-title"); n = pg.evaluate("E.panels.find(p=>p.type==='chrom'&&p.tab==='full')._a.sr[0].x.length")
            assert "·" in h and str(n) in h, (h, n)
            assert pg.evaluate("document.querySelector('#hri-body').firstElementChild.id") == "hri-list" and pg.evaluate("document.querySelector('#hri-list').title||document.querySelector('#hri-list').dataset.tip") != "", "no line above the table"
            assert pg.evaluate("[...document.querySelectorAll('#hri-list .r')].slice(0,12).every(r=>/^\\d+\\.\\d{3}$/.test(r.children[1].innerText)&&/^\\d+\\.\\d{4}$/.test(r.children[4].innerText))"), "fixed decimals"
            assert pg.evaluate("getComputedStyle(document.querySelector('#hri-list .r span')).fontVariantNumeric") == "tabular-nums" or "tabular" in pg.evaluate("getComputedStyle(document.querySelector('#hri-list .r')).fontVariantNumeric")
            tab("hdr"); assert "·" not in pg.inner_text("#sb-title") or str(n) not in pg.inner_text("#sb-title"), "the count leaves with the tab"
        step("list of the scans: count beside the title, no line above the table, fixed decimals per column", lst)
        assert not [e for e in r.errs if e[0] != "console.warning"], r.errs[:5]
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(("  ok  " if s == "ok" else " FAIL ") + n + ("" if s == "ok" else "  → " + s))
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
