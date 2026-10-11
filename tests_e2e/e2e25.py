# e2e25: settings gear (3 controls), chart colours (4 palettes, colours by time on the Full Scan tab), "Nuova sessione" with warning and cleanup.
# Synthetic data (tests_e2e/synth.py): no real lab files needed.
import os, sys, re, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e25.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
def lum(h):
    c = [int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)]; c = [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c]; return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
def hexof(c):                                               # "rgb(r, g, b)" or "#rrggbb" -> "#rrggbb"
    c = c.strip()
    if c.startswith("#"): return c.lower()
    m = re.findall(r"\d+", c)[:3]; return "#%02x%02x%02x" % tuple(int(v) for v in m)
r = Run(port=8825, wd="/tmp/wd25")
COLORS = "E.files.filter(f=>f.kind==='full'&&f.type==='sample'&&f.time!=null).sort((a,b)=>a.time-b.time).map(f=>[f.time,f.color,f.rank])"
def load(pg, times, name):
    shutil.rmtree("/tmp/s25_" + name, ignore_errors=True)
    files = synth.make_series("/tmp/s25_" + name, times)
    pg.set_input_files("#pick", files); pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [60, 0, 30, 5, 15], "5")                        # loaded in a mixed order on purpose
        def gear():
            pg.click("#np-set"); pg.wait_for_timeout(200)
            t = pg.inner_text("#uipset"); print(repr(t))
            assert pg.locator("#uipset .row").count() == 7, pg.locator("#uipset .row").count()      # language, theme, font, high contrast, colours and the merge of the centroids (the text size is the browser's: Ctrl/Cmd + and −); the libraries and the high-resolution rows only appear with MS2 / high-resolution files open
            assert "Dimensione testo" not in t and "Tema" in t and "Colori dei grafici" in t
            for bad in ("Dimensione del testo dell", "Vale per menu", "tutorial", "Installare", "Numera", "suggerimenti"):
                assert bad.lower() not in t.lower(), bad
            assert pg.locator("#uipset input[type=checkbox]").count() == 3 and pg.locator("#uip-merge").count() == 1 and pg.locator("#uip-tog").count() == 1      # the merge of the centroids and «Mostra tutti gli esperimenti insieme» (off by default)
            assert [o.strip() for o in pg.locator("#uip-pal option").all_inner_texts()] == ["Per tempo (predefinito)", "Accessibili", "Alto contrasto", "Arcobaleno", "Bianco e nero"]
            assert pg.input_value("#uip-pal") == "time"
            pg.select_option("#uip-th", "dark"); pg.wait_for_timeout(400)
            pg.select_option("#uip-th", "auto"); pg.wait_for_timeout(300)
            assert pg.evaluate("UIP.tips") is None and pg.evaluate("UIP.num") is None and pg.evaluate("typeof TUT") == "undefined" and pg.evaluate("typeof tipOf") == "undefined"
            assert pg.evaluate("document.querySelectorAll('.pnum:not([hidden])').length") >= 1, "panel numbers stay on"
            assert pg.evaluate("document.querySelectorAll('[title]').length") > 40, "tooltips stay on"
        step("gear: exactly 3 controls; tips and numbers are fixed on; no tutorial", gear)
        def time_scale():
            c = pg.evaluate(COLORS); print(c)
            assert [x[0] for x in c] == [0, 5, 15, 30, 60], c
            hs = [hexof(x[1]) for x in c]; assert len(set(hs)) == 5, hs
            ls = [lum(h) for h in hs]; assert all(ls[i] < ls[i + 1] for i in range(4)), ("luminance must rise with time", ls)
            assert ls[0] < 0.1 and lum("#ffffff") / (ls[-1] + 0.05) * 1.05 >= 1.9, "t0 not near white; last colour readable"
            assert [x[2] for x in c] == [0, 1, 2, 3, 4]
        step("5 files: one colour per file ordered by time (t0 darkest), whatever the loading order", time_scale)
        def neutral():
            f = pg.evaluate("E.files.filter(f=>f.type==='blank'||f.type==='standard').map(f=>[f.type,f.color,f.rank])")
            d = {t: hexof(c) for t, c, _ in f}; print(d)
            assert d["blank"] == "#8a8a8a" and d["standard"] == "#4d4d4d" and all(x[2] is None for x in f)
            dash = pg.evaluate("E.files.map(f=>[f.type,JSON.stringify(fdash(f))])"); print(dash)
            assert any(t == "blank" and v == "[4,3]" for t, v in dash) and any(t == "standard" and v == "[10,4]" for t, v in dash)
        step("blank and standard stay out of the scale (neutral greys, dashed)", neutral)
        def list_colours():
            sw = pg.evaluate("[...document.querySelectorAll('#flst .fl i')].map(i=>getComputedStyle(i).backgroundColor)")
            cols = {hexof(x[1]) for x in pg.evaluate(COLORS)}; assert cols <= {hexof(s) for s in sw}, (cols, sw)
        step("the file list shows the same colours", list_colours)
        def manual():
            pg.evaluate("(()=>{const f=E.files.find(x=>x.time===15);f.color='#123456';f.colorSet=true;paintFiles()})()")
            c = {x[0]: hexof(x[1]) for x in pg.evaluate(COLORS)}; assert c[15] == "#123456", c
            ls = [lum(c[t]) for t in (0, 5, 30, 60)]; assert all(ls[i] < ls[i + 1] for i in range(3)), "the others keep their order"
            pg.evaluate("resetColors()"); c = {x[0]: hexof(x[1]) for x in pg.evaluate(COLORS)}; assert c[15] != "#123456"
            pg.evaluate("E.files.find(x=>x.time===5).colorSet")
        step("a colour chosen by hand is not overwritten; 'ripristina colori' brings the scale back", manual)
        def menu_entry():
            pg.click("#flst .fl .nm >> nth=0", button="right"); pg.wait_for_timeout(300)
            assert "Ripristina i colori automatici (per tempo)" in pg.inner_text("body")
            pg.keyboard.press("Escape"); pg.mouse.click(5, 5)
        step("the file menu has 'Ripristina i colori automatici (per tempo)'", menu_entry)
        def palettes():
            seen = {}
            for k in ("time", "cb", "hc", "rainbow"):
                pg.click("#np-set") if pg.locator("#uipset").count() == 0 else None
                pg.select_option("#uip-pal", k); pg.wait_for_timeout(500)
                seen[k] = [hexof(x[1]) for x in pg.evaluate(COLORS)]
                assert pg.evaluate("UIP.pal") == k and pg.evaluate("document.documentElement.dataset.pal") == k
            print(seen); assert len({tuple(v) for v in seen.values()}) == 4, "each palette changes the colours"
            # contrast of the high-contrast palette: every colour >= 4.5:1 on white
            assert all(1.05 / (lum(h) + 0.05) >= 4.5 for h in seen["hc"]), seen["hc"]
            # colour blind: also a line style per file; time order kept in luminance
            pg.select_option("#uip-pal", "cb"); pg.wait_for_timeout(300)
            d = pg.evaluate("E.files.filter(f=>f.rank!=null).sort((a,b)=>a.rank-b.rank).map(f=>JSON.stringify(fdash(f)))"); print(d)
            assert len(set(d[:4])) == 4, d
            ls = [lum(h) for h in seen["cb"]]; assert all(ls[i] < ls[i + 1] for i in range(4)), "cividis rises in lightness"
            pg.select_option("#uip-pal", "hc"); pg.wait_for_timeout(300)
            assert pg.evaluate("LWX()") == 1 and pg.evaluate("getComputedStyle(document.documentElement).getPropertyValue('--muted').trim()") == "#000"
            pg.select_option("#uip-pal", "time"); pg.wait_for_timeout(300); assert pg.evaluate("LWX()") == 0
        step("4 palettes: colours change at once, hc >= 4.5:1 and +1 px, cb has line styles, order by time kept", palettes)
        def persists():
            pg.select_option("#uip-pal", "cb"); pg.wait_for_timeout(300)
            assert pg.evaluate("JSON.parse(localStorage.getItem('qqq.prefs'))") == {"theme": "auto", "pal": "cb", "merge": True}
            pg.reload(); pg.wait_for_timeout(3500)
            assert pg.evaluate("UIP.pal") == "cb"
            c = [hexof(x[1]) for x in pg.evaluate(COLORS)]; assert c and c[0] == "#00204d", c       # cividis start: colours recomputed with the saved palette
            pg.select_option("#uip-pal", "time") if pg.locator("#uipset").count() else (pg.click("#np-set"), pg.select_option("#uip-pal", "time"))
        step("the palette survives a reload (localStorage qqq.prefs)", persists)
        def traces_use_palette():
            pg.evaluate("E.files.forEach(f=>f.vis=true); redrawAll()")
            pg.wait_for_function("(()=>{const p=E.panels.find(p=>p.type==='chrom'); return !!(p && p._a && p._a.sr && p._a.sr.length >= 5)})()", timeout=10000)
            pg.evaluate("(()=>{E.tab='full'})()")
            fl = pg.evaluate("E.panels.filter(p=>p.type==='chrom').length"); assert fl >= 1
            col = pg.evaluate("(()=>{const p=E.panels.find(p=>p.type==='chrom');return p._a && p._a.sr ? p._a.sr.map(s=>s.color) : null})()")
            print("trace colours", col); assert col and len(col) >= 5
            files = {hexof(x[1]) for x in pg.evaluate(COLORS)}; assert files <= {hexof(c) for c in col}, (files, col)
        step("the chromatogram traces use the file colours", traces_use_palette)
        def n_variants():
            for n, times in ((3, [0, 15, 60]), (7, [0, 5, 10, 15, 30, 45, 60])):
                pg.click("#newrun"); pg.wait_for_timeout(500); pg.click("#askok"); pg.wait_for_timeout(2500)
                load(pg, times, str(n))
                c = pg.evaluate(COLORS); hs = [hexof(x[1]) for x in c]; ls = [lum(h) for h in hs]
                assert len(c) == n and len(set(hs)) == n and all(ls[i] < ls[i + 1] for i in range(n - 1)), (n, hs)
                pg.screenshot(path=SH + f"250_scale_{n}.png", clip={"x": 0, "y": 60, "width": 400, "height": 500})
        step("n = 3 and n = 7: distinct, ordered colours (screenshots 250_scale_*.png)", n_variants)
        def new_session():
            pg.evaluate("localStorage.setItem('qqq.extra','1')"); pg.click("#np-set") if pg.locator("#uipset").count() == 0 else None
            pg.select_option("#uip-pal", "rainbow"); pg.wait_for_timeout(300)
            n0 = pg.evaluate("E.files.length"); assert n0 > 0
            pg.click("#newrun"); pg.wait_for_timeout(500)
            txt = pg.inner_text("#asktxt"); print(txt)
            assert "il lavoro salvato" in txt.lower() and "da zero" in txt and "i file sul tuo computer non vengono toccati" in txt
            pg.click("#askno"); pg.wait_for_timeout(500)                               # cancel: nothing is touched
            assert pg.evaluate("E.files.length") == n0 and pg.evaluate("localStorage.getItem('qqq.extra')") == "1" and pg.evaluate("UIP.pal") == "rainbow"
            pg.click("#newrun"); pg.wait_for_timeout(500); pg.click("#askok"); pg.wait_for_timeout(3000)
            print("after:", pg.evaluate("[E.files.length, localStorage.getItem('qqq.extra'), localStorage.getItem('qqq.prefs')]"))
            assert pg.evaluate("E.files.length") == 0 and pg.evaluate("localStorage.getItem('qqq.extra')") is None and pg.evaluate("localStorage.getItem('qqq.prefs')") is None
            assert pg.evaluate("UIP.pal") == "time" and pg.evaluate("document.documentElement.dataset.pal") == "time" and pg.evaluate("UIP.font") is None
            assert pg.evaluate("(NB.session||null)") is None and pg.evaluate("Object.keys(NB).filter(k=>k!=='ui'&&k!=='session').length") == 0
            st = pg.evaluate("fetch('api/notebook').then(r=>r.json())"); assert not st.get("session"), st
        step("Nuova sessione: warning text, cancel changes nothing, confirm clears files, notebook, preferences", new_session)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
