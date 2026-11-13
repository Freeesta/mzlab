# e2e_ms3: a file of MS3 scans only (synthetic, tools/synthetic_data_msn.ms3) opens in the product-ion tab at level 3: the precursor chromatogram is drawn
# (never empty for lack of MS2 scans), titled «TIC · MS³», its spectrum is «MS³ of 194.1» with the MS3 product ions, the level menu offers MS3,
# the scan list holds the MS3 scans and the precursor table lists 194.1; IT and EN. The MS3 spectrum behaves as an MS2 one: the axis fits its own peaks and
# the precursor, the scan line gives the CE, the exports are named and described MS3; with a second MS3 file (precursors 210.1 and 230.1) the spectrum of
# a precursor the shown file lacks falls back to the file that has it, and «all precursors» splits one spectrum per precursor.
import os, sys, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools")); import synthetic_data_msn as ds, numpy as np
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
tmp = Path(tempfile.mkdtemp(prefix="ms3_"))
F = tmp / "S_MS3-t0.mzML"; ds.ms3(F, np.random.default_rng(3))
G = tmp / "S_MS3b-t0.mzML"; rng = np.random.default_rng(4)
ds._write(G, [ds._spec(i, rt, np.array([100.1, 160.1]) + 10 * (i % 2), np.array([3e5, 6e5]), level=3, prec=210.1 + 20 * (i % 2), ce=30.0)
              for i, rt in enumerate(np.linspace(0.02, 10.0, 200))], [])
SPEC = "E.panels.find(p => p.tab === 'ms2' && p.type === 'spec')"
r = Run(port=8983, wd="/tmp/wd_ms3")
try:
    with sync_playwright() as p:
        for lang in ("it", "en"):
            pg = r.page(p); pg.set_viewport_size({"width": 1280, "height": 1000})
            pg.goto(f"http://127.0.0.1:{r.port}/?lang={lang}"); wait_for_js(pg, "!!document.querySelector('#pick')", 30000)
            pg.set_input_files("#pick", [str(F)])
            wait_for_js(pg, "E.files.some(f => f.kind === 'ms2' && !f.gone)", 60000)
            wait_for_js(pg, "E.panels.some(p => p.tab === 'ms2' && p.type === 'chrom' && p._a && p._a.sr)", 60000); frames(pg, 6)
            def level(pg=pg):
                f = pg.evaluate("(() => { const f = E.files.find(f => f.kind === 'ms2'); return [f.msn_level, f.lv]; })()"); assert f == [3, 3], f
            step(f"{lang}: the file is MSn of level 3", level)
            def chrom(pg=pg):
                n = pg.evaluate("(() => { const c = E.panels.find(p => p.tab === 'ms2' && p.type === 'chrom'); return c._a.sr.map(s => (s.y || []).filter(v => v > 0).length); })()")
                assert n and n[0] > 100, ("empty MS3 chromatogram", n)
            step(f"{lang}: the precursor chromatogram is drawn from the MS3 scans", chrom)
            def titles(lang=lang, pg=pg):
                wait_for_js(pg, "E.panels.some(p => p.tab === 'ms2' && p.type === 'spec' && p.el)", 30000); frames(pg, 4)
                t = pg.evaluate("E.panels.filter(p => p.tab === 'ms2').map(p => p.el.querySelector('.ttl').textContent)")
                want = {"it": ["TIC · MS³", "MS³ di 194.1"], "en": ["TIC · MS³", "MS³ of 194.1"]}[lang]
                assert t == want, t
            step(f"{lang}: titles name MS³", titles)
            def spectrum(pg=pg):
                wait_for_js(pg, "(() => { const s = E.panels.find(p => p.tab === 'ms2' && p.type === 'spec'); return s && s.level === 3 && s._a && s._a.data && s._a.data.length; })()", 30000)
                mz = pg.evaluate("(() => { const s = E.panels.find(p => p.tab === 'ms2' && p.type === 'spec'); return [...s._a.data[0].d.mz].map(Number); })()")
                assert any(abs(m - 150.1) < 0.3 for m in mz), ("no MS3 product ion in the spectrum", mz[:10])
                o = pg.evaluate("[...document.querySelectorAll('.pnl select[data-o=level] option')].map(o => [o.value, o.textContent])")
                assert ["3", "MS3"] in o and ["2", "MS2"] not in o, o
            step(f"{lang}: the spectrum is the MS3 one and the level menu offers MS3", spectrum)
            def routes(pg=pg):
                k = pg.evaluate("E.files.find(f => f.kind === 'ms2').k")
                sl = pg.evaluate(f"J('api/scanlist?k={k}&level=3').then(d => (d.sid || []).length)"); assert sl == 400, sl
                tr = pg.evaluate(f"getMs2Tr({k}, 3).then(t => t.map(x => x.q1))"); assert tr and all(abs(q - 194.1) < 0.06 for q in tr), tr
            step(f"{lang}: scan list and precursor table at level 3", routes)
            def like_ms2(lang=lang, pg=pg):
                ax = pg.evaluate(f"(() => {{ const s = {SPEC}; return [s._a.x0, s._a.x1]; }})()")
                assert 110 < ax[0] < 122 and 199 <= ax[1] < 215, ("the axis does not fit the MS3 peaks and the precursor", ax)
                leg = pg.evaluate(f"{SPEC}.leg.textContent"); assert "CE 35" in leg, leg
                name = pg.evaluate(f"plotName({SPEC})"); assert name.startswith({"it": "spettroMS3", "en": "spectrumMS3"}[lang]) and "prec194.1" in name, name
                typ = pg.evaluate(f"plotSheets({SPEC}).find(x => x.rows && x.rows.some(r => String(r[1]).includes('MS³')))")
                assert typ, "the metadata sheet does not say MS³"
            step(f"{lang}: the MS3 spectrum fits its peaks, gives its CE and is exported as MS3", like_ms2)
            pg.close()
        pg = r.page(p); pg.goto(f"http://127.0.0.1:{r.port}/?lang=en"); wait_for_js(pg, "!!document.querySelector('#pick')", 30000)
        pg.set_input_files("#pick", [str(F), str(G)])
        wait_for_js(pg, "E.files.filter(f => f.kind === 'ms2' && !f.gone && f.lv === 3).length === 2", 60000)
        wait_for_js(pg, f"(() => {{ const s = {SPEC}; return s && s._a && s._a.data && s._a.data.length; }})()", 60000)
        kb = pg.evaluate("E.files.find(f => f.file === 'S_MS3b-t0.mzML').k")
        def fallback():
            got = pg.evaluate(f"""(async () => {{ const s = {SPEC}; s.k = {kb}; s.link = true; s.prec = 194.1; await draw(s);
                return s._a ? s._a.data.map(x => [x.f.file, x.d.mz.length]) : s._say; }})()""")
            assert isinstance(got, list) and got and all(g[0] == "S_MS3-t0.mzML" and g[1] > 0 for g in got), got
        step("a precursor the shown MS3 file lacks falls back to the MS3 file that has it", fallback)
        def split():
            got = pg.evaluate(f"""(async () => {{ const s = {SPEC}; s.k = {kb}; s.link = true; s.prec = null; await draw(s);
                return s._a ? s._a.data.map(x => [x.f.file, +x.prec]) : s._say; }})()""")
            assert isinstance(got, list) and sorted(round(g[1], 1) for g in got) == [210.1, 230.1], got
        step("«all precursors» of an MS3 file with two precursors draws one spectrum per precursor", split)
        pg.close()
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
