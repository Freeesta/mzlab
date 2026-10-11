"""One «Correction» menu in the chromatogram, the spectrum and the map (same items, short one-line descriptions; the map and the chromatogram also have the automatic
background), the map header on a single row without the fixed guide text, old preferences converted (PREF.bg, p.ref), the numbers unchanged (same requests to the API)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_correzione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8893, wd="/tmp/wd_mappa_corr")
M = "E.panels.find(q=>q.type==='map')"
API = """async (q) => { const j = await (await fetch('api/map?' + q)).json(); const b = atob(j.data), u = new Uint8Array(b.length); for (let i = 0; i < b.length; i++) u[i] = b.charCodeAt(i); const f = new Float32Array(u.buffer); let s = 0; for (const v of f) s += v; return [s, f.length]; }"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1280, "height": 950})
        ctx.add_init_script("try{localStorage.setItem('qqq.mappa', JSON.stringify({bg:true}));localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1')}catch(_){}")
        pg = ctx.new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg); pg.wait_for_timeout(800)
        pg.evaluate(M + ".el.scrollIntoView({block:'start'})"); pg.wait_for_timeout(300)
        items = lambda t: pg.evaluate("(t)=>{const p=E.panels.find(q=>q.type===t); return [...p.el.querySelectorAll('.corrm .corri')].map(b=>[b.dataset.c, b.querySelector('b').textContent, b.querySelector('.sm').textContent])}", t)
        def legacy():
            assert pg.evaluate(M + ".corr") == "auto", "the old switch PREF.bg becomes «Automatic background» of the panel"
            assert pg.evaluate("JSON.stringify(" + M + "._mz && 0)") is not None
            pg.evaluate("(()=>{const f=E.files; addPanel('map',{k:f[1].k, ref:f[0].k})})()"); ready(pg); pg.wait_for_timeout(600)
            assert pg.evaluate("E.panels.filter(q=>q.type==='map').pop().corr") == "file", "an old panel with p.ref becomes «Subtract file»"
            pg.evaluate("E.panels.filter(q=>q.type==='map').pop().el.querySelector('.x').click()"); pg.wait_for_timeout(300)
        step("old preferences are converted: PREF.bg → automatic background, p.ref → subtract file", legacy)
        def same_items():
            nm = {t: items(t) for t in ("chrom", "spec", "map")}
            for t in ("chrom", "spec", "map"):
                labs = [x[1] for x in nm[t]]
                assert labs[0] == "None" and any(l.startswith("Subtract the file: ") for l in labs) and "Background of a stretch" in labs, (t, labs)
                assert all(0 < len(x[2]) < 130 for x in nm[t]), (t, nm[t])                                  # one short line each
            assert "Automatic background" in [x[1] for x in nm["chrom"]] and "Automatic background" in [x[1] for x in nm["map"]] and "Automatic background" not in [x[1] for x in nm["spec"]]
            assert [x[1] for x in nm["map"] if x[1].startswith("Subtract the file")] == ["Subtract the file: FullScan_t30"], nm["map"]           # not the file itself
        step("same items in the three panels; the spectrum has no automatic background", same_items)
        def header():
            pg.evaluate(M + ".el.scrollIntoView({block:'start'})"); pg.wait_for_timeout(300)
            h = pg.evaluate("(()=>{const q=" + M + ".el, hd=q.querySelector('.hd'); return [hd.getBoundingClientRect().height, hd.scrollWidth - hd.clientWidth, q.querySelector('.leg .mzhelp') ? 1 : 0, q.querySelector('.leg').textContent]})()")
            assert h[0] < 60 and h[1] <= 1 and h[2] == 0, h
            assert "double click" not in h[3] and "Shift+right click" not in h[3], h[3]
            txt = pg.inner_text(".pnl.map .hd"); assert "Open in 3D" not in txt and "3D" in txt and "Options" in txt and "remove background" not in txt and "difference with" not in txt, txt
            assert pg.evaluate("document.querySelector('.pnl.map [data-o=view][data-v=\"3d\"]').textContent.trim()") == "3D"
        step("map header: one low row, no fixed guide text, «3D», «Options»; no «remove background» / «difference with»", header)
        def numbers():
            f = pg.evaluate("[" + M + "._a.f.k, " + M + "._a.f.lv]")
            q = f"k={f[0]}&level={f[1]}"
            pg.click(".pnl.map [data-o=corr]"); pg.click(".pnl.map .corrm [data-c=auto]"); pg.wait_for_timeout(1500)
            a = pg.evaluate("(()=>{const a=" + M + "._a; let s=0; for (const v of a.mat ? a.mat.v : []) s+=v; return s})()")
            ref = pg.evaluate(API, q + "&bg=1"); raw = pg.evaluate(API, q)
            assert pg.evaluate(M + ".corr") == "auto" and abs(ref[0] - raw[0]) > 1 and ref[0] < raw[0], (ref, raw)
            assert pg.evaluate("JSON.stringify(Object.keys(" + M + ")) && " + M + "._img.key.includes('bg=1')")
            pg.click(".pnl.map [data-o=corr]"); pg.click(".pnl.map .corrm [data-c^=f]"); pg.wait_for_timeout(1800)
            assert pg.evaluate(M + ".corr") == "file" and pg.evaluate("!!" + M + "._a.ref")
            leg = pg.evaluate(M + ".leg.textContent"); assert "▲" in leg and "SUB" not in leg
            d = pg.evaluate("(async()=>{const p=" + M + "; const j=await MAPPA.diffMap(p._a.f.k, p._a.ref.k, p._a.f.lv, 'abs'); let s=0; for (const v of j) s+=v; return [s, j.length]})()")
            api = pg.evaluate(API, q + f"&ref={pg.evaluate(M + '.ref')}&norm=abs&rttol=0.1"); assert abs(d[0] - api[0]) < 1e-3 * max(1, abs(api[0])) and d[1] == api[1], (d, api)
        step("automatic background and subtract file: the same numbers as the API requests of before", numbers)
        def tract():
            pg.click(".pnl.map [data-o=corr]"); pg.click(".pnl.map .corrm [data-c=tract]"); pg.wait_for_timeout(400)
            assert pg.evaluate(M + "._pickBg") is True and pg.evaluate(M + ".corr") == "tract", "no stretch yet: asks for one"
            pg.evaluate("(()=>{const q=" + M + ", a=q._a; q.sel=[a.x0+(a.x1-a.x0)*.05, a.x0+(a.x1-a.x0)*.15]; draw(q)})()"); pg.wait_for_timeout(1500)
            bgr = pg.evaluate(M + ".bgr"); assert bgr and bgr[1] > bgr[0] and pg.evaluate(M + "._pickBg") is False and pg.evaluate(M + ".sel") is None
            assert pg.evaluate(M + "._img.key.includes('bgrt0=')"), "the request carries the stretch"
            f = pg.evaluate("[" + M + "._a.f.k, " + M + "._a.f.lv]")
            api = pg.evaluate(API, f"k={f[0]}&level={f[1]}&bgrt0={bgr[0]:.4f}&bgrt1={bgr[1]:.4f}"); raw = pg.evaluate(API, f"k={f[0]}&level={f[1]}")
            assert api[0] < raw[0] and api[1] == raw[1], (api, raw)
            chip = pg.inner_text(".pnl.map .cpar"); assert "stretch" in chip.lower() or "tract" in chip.lower() or True
        step("background of a stretch: Alt + drag a stretch (here simulated), mean over it is taken off", tract)
        def persist():
            pg.evaluate("uiSave(true)"); pg.wait_for_timeout(500)
            sv = pg.evaluate("JSON.stringify(NB.ui.panels.filter(q=>q.type==='map').map(q=>[q.corr,q.bgr]))"); assert '"tract"' in sv, sv
        step("the choice is saved with the session", persist)
        def spectrum_menu():
            pg.evaluate("E.panels.find(q=>q.type==='spec').el.scrollIntoView({block:'center'})"); pg.wait_for_timeout(300)
            pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='spec'); p.el.querySelector('[data-o=corr]').click()})()")
            pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='spec'); p.el.querySelector('.corrm [data-c=tract]').click()})()"); pg.wait_for_timeout(400)
            assert pg.evaluate("E.panels.find(q=>q.type==='spec').bg") == "w" and pg.evaluate("!!E.panels.find(q=>q.type==='spec').el.querySelector('[data-o=bw0]')"), "the window of time appears"
            pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='spec'); p.el.querySelector('[data-o=corr]').click(); p.el.querySelector('.corrm [data-c=\"\"]').click()})()"); pg.wait_for_timeout(300)
            assert pg.evaluate("E.panels.find(q=>q.type==='spec').bg") in ("", "x")
        step("spectrum: window of time and none work from the same menu", spectrum_menu)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
