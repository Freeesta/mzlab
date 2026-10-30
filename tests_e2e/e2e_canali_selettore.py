"""One picker for what a chromatogram plots (canali.js): groups, column names, «TIC + 1», detectors and logs as rows, the pump pressure is drawn when picked,
the keyboard, the targets, and no separate «Channels» button."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_canali_selettore.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "canali_sel_tmp"; D.mkdir(parents=True, exist_ok=True)
F = D / "PUMP_t0.mzML"; ds.full_scan(F, 0, np.random.default_rng(3), pressure=True)
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
r = Run(port=8977, wd="/tmp/wd_canali_sel")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)]); pg.wait_for_selector("#pick", state="attached")
        pg.click("text=Carica dati"); ready(pg)
        attendi(pg, f"{CH}.el.querySelector('[data-o=kind]')")
        def one_button():
            n = pg.evaluate(f"{CH}.el.querySelectorAll('[data-o=canali]').length"); assert n == 0, n
            assert pg.evaluate(f"{CH}.el.querySelector('select[data-o=kind]')") is None
            assert pg.evaluate(f"{CH}.el.querySelector('[data-o=kind] .can-bt').textContent") == "TIC"
        step("a single picker, shown as «TIC»; no Channels button", one_button)
        def groups():
            pg.click(".pnl.chrom [data-o=kind]")
            attendi(pg, "document.querySelector('.can-pop:not([hidden]) [role=treeitem]')")
            g = pg.evaluate("[...document.querySelectorAll('.can-pop:not([hidden]) [role=group]')].map(e => e.getAttribute('aria-label'))")
            assert g[0] == "Spettrometria di massa" and "Rivelatori" in g, g
            h = pg.evaluate("[...document.querySelectorAll('.can-pop:not([hidden]) .can-hd span')].map(e => e.textContent)")
            assert h[:4] == ["Canale", "Esperimento", "Punti", "Tempo (min)"], h
            ms1 = pg.evaluate("(document.querySelector('.can-pop:not([hidden]) [data-omo^=\"ms|1\"]') || {}).title || ''")
            assert "rilevamento" in ms1.lower(), ms1
        step("groups, column names and the hover text of MS¹", groups)
        def label():
            pg.click(".can-pop:not([hidden]) [data-omo^='ms|1']"); ready(pg)
            t = pg.evaluate(f"{CH}.el.querySelector('[data-o=kind] .can-bt').textContent"); assert t.endswith("+ 1"), t
        step("two channels: the label reads «… + 1»", label)
        def pressure():
            attendi(pg, "document.querySelector('.can-pop:not([hidden])')")
            pg.click(".can-pop:not([hidden]) [data-kind=press]"); ready(pg); frames(pg, 8)
            k = pg.evaluate(f"{CH}.kind"); assert k == "press", k
            v = pg.evaluate(f"(()=>{{const a={CH}._a;return [a.sr.length, a.sr[0].y.length]}})()"); assert v[0] == 1 and v[1] > 10, v
            assert pg.evaluate(f"{CH}.el.querySelector('[data-o=kind] .can-bt').textContent") == "Pressione della pompa"
        step("the pump pressure is drawn when picked", pressure)
        def back():
            pg.click(".pnl.chrom [data-o=kind]"); attendi(pg, "document.querySelector('.can-pop:not([hidden])')")
            pg.click(".can-pop:not([hidden]) [data-omo='tic']"); ready(pg); frames(pg, 6)
            k = pg.evaluate(f"{CH}.kind"); assert k == "tic", k
        step("going back to a channel of scans", back)
        def keys():
            pg.keyboard.press("Escape")
            pg.focus(".pnl.chrom [data-o=kind]"); pg.keyboard.press("ArrowDown")
            attendi(pg, "document.querySelector('.can-pop:not([hidden]) .can-row:focus')")
            pg.keyboard.press("Escape"); assert pg.evaluate("document.querySelector('.can-pop').hidden")
            assert pg.evaluate("document.activeElement.dataset.o") == "kind"
        step("keyboard: ArrowDown opens, Escape closes and returns the focus", keys)
        def size():
            pg.click(".pnl.chrom [data-o=kind]"); attendi(pg, "document.querySelector('.can-pop:not([hidden])')")
            h = pg.evaluate("Math.min(...[...document.querySelectorAll('.can-pop:not([hidden]) .can-row')].filter(e => e.offsetParent).map(e => e.getBoundingClientRect().height))"); assert h >= 24, h
            b = pg.evaluate(f"{CH}.el.querySelector('[data-o=kind]').getBoundingClientRect().height"); assert b >= 24, b
            pg.keyboard.press("Escape")
        step("targets of at least 24 px", size)
        def shift():
            w0 = pg.evaluate(f"{CH}.el.querySelector('[data-o=kind]').getBoundingClientRect().width")
            pg.evaluate(f"(()=>{{const p={CH}; p.kind='press'; ctl(p)}})()")
            w1 = pg.evaluate(f"{CH}.el.querySelector('[data-o=kind]').getBoundingClientRect().width"); assert w0 >= 100 and w1 >= 100, (w0, w1)
        step("the button keeps a stable minimum width", shift)
        def miss():
            assert pg.evaluate("(window.MZLAB_LR_MISS||[]).length") == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("no request left unanswered", miss)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
