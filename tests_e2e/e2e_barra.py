"""A1: the toolbar above the panels. Whole file name in the file menu, no «Altro» / «Unisci gli XIC», a side-panel icon for the file list."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_barra.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
def blob(name, src): return {"name": name, "mimeType": "application/octet-stream", "buffer": open(mz(src), "rb").read()}
LONG = "Campione_con_un_nome_davvero_molto_lungo_per_la_prova_B_FullMass-t30 (2)"
r = Run(port=8836, wd="/tmp/wd_barra")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1280, "height": 900})
        pg.set_input_files("#pick", [blob("B_FullMass-t30 (2).mzML", "B_FullMass-t15"), blob(LONG + ".mzML", "B_FullMass-t0")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(800)
        pg.evaluate("soloFile(E.files[E.cur])"); attendi(pg, "E.browse")      # the file menu works with one file shown
        sel = lambda: pg.evaluate("(() => { const s = Q('#fsel'); return {t: s.options[s.selectedIndex].textContent, w: s.offsetWidth, title: s.title, opts: [...s.options].map(o => o.textContent), bar: Q('#tools').offsetWidth}; })()")
        def name():
            pg.evaluate("E.cur = tabFiles().find(f => /t30/.test(f.label) && !/Campione/.test(f.label)).k; renderNav()")
            attendi(pg, "Q('#fsel').offsetWidth >= 220")
            s = sel(); assert "B_FullMass-t30 (2)" in s["t"], s
            assert s["w"] >= 220 and s["w"] <= 0.4 * 1280 + 2, s
            tw = pg.evaluate("(() => { const s = Q('#fsel'), g = document.createElement('canvas').getContext('2d'), cs = getComputedStyle(s); g.font = cs.fontSize + ' ' + cs.fontFamily; return g.measureText(s.options[s.selectedIndex].textContent).width; })()")
            assert tw < s["w"] - 16, (tw, s["w"])
        step("1280 px: the whole name is readable", name)
        def long():
            pg.evaluate("E.cur = tabFiles().find(f => /Campione/.test(f.label)).k; renderNav()")
            s = sel(); assert "…" in s["t"] and s["t"].endswith("t30 (2)") and LONG.split(".")[0][:10] in s["t"], s
            assert LONG in s["title"], s
            assert s["w"] <= 0.4 * 1280 + 2, s
        step("a very long name is cut in the middle, full name in the tooltip", long)
        def gone():
            assert pg.evaluate("['np-more','np-merge','morepop'].every(i => !document.getElementById(i))")
            assert not pg.evaluate("!!document.getElementById('np-tile')")
            assert "Altro" not in pg.inner_text("#tools") and "Unisci" not in pg.inner_text("#tools")
        step("no Altro / Unisci; no Ordina button", gone)
        def fold():
            b = "#stbar [data-cmd=file]"
            assert pg.evaluate(f"!!Q('{b} svg')") and pg.evaluate(f"Q('{b}').title") == "Elenco dei file caricati: scegli, nascondi, rinomina, colora"
            ready(pg)
            if pg.is_visible("#flst"):          # the drawer remembers its state: start from the closed one, so the click below opens it
                pg.click(b); attendi(pg, "(() => { const e = Q('#flst'); return !e || e.getBoundingClientRect().width === 0 })()")
            pg.click(b)
            attendi(pg, "(() => { const e = Q('#flst'); return !!e && e.getBoundingClientRect().width > 0 })()")
            assert pg.is_visible("#flst") and pg.is_visible("#sb-x") and pg.evaluate("!!Q('#sb-x svg')") and pg.evaluate("Q('#sb-x').title") == "Chiudi il pannello"
            pg.click("#sb-x")
            attendi(pg, "(() => { const e = Q('#flst'); return !e || e.getBoundingClientRect().width === 0 })()")
            assert not pg.is_visible("#flst")
            # the only previous/next arrows left in the toolbar are the file arrows (SVG icons, named by their aria-label)
            assert pg.evaluate("[...document.querySelectorAll('#tools button')].filter(b => /precedente|successivo/i.test(b.getAttribute('aria-label') || '')).map(b => b.id)") == ["fprev", "fnext"]
        step("side-panel icon for the file list", fold)
        def narrow():
            pg.set_viewport_size({"width": 700, "height": 900}); pg.wait_for_timeout(600)
            h = pg.evaluate("Q('#tools').offsetHeight"); s = sel()
            assert h > 45 and s["w"] <= 700 * 0.6 + 2, (h, s)
        step("narrow window: the bar wraps", narrow)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
bad = [s for s in steps if s[1] != "ok"]
r.errs = [e for e in r.errs if not str(e).startswith("('http4")] if hasattr(r, "errs") else []
print("--- ERRORS", r.errs[:5]); sys.exit(1 if bad else 0)
