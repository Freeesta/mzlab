# e2e_ms23: one file with MS2 and MS3 scans in turn (synthetic, tools/dati_sintetici_msn.ms2_ms3) opens as two items, «· MS2» at level 2 and «· MS3» at
# level 3, in one group named «MS2/MS3 (Product Ion)»; switching the precursor to 194.1 draws the MS3 chromatogram titled «MS³» and its MS3 spectrum,
# back to 237.1 the MS2 ones; scan lists and precursor tables are per level; IT and EN.
import os, sys, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici_msn as ds, numpy as np
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
tmp = Path(tempfile.mkdtemp(prefix="ms23_"))
F = tmp / "S_MSN-t0.mzML"; ds.ms2_ms3(F, np.random.default_rng(3))
SPEC = "(() => { const s = E.panels.find(p => p.tab === 'ms2' && p.type === 'spec'); return s && s._a && s._a.data && s._a.data[0] ? [s.k, s.level, s.el.querySelector('.ttl').textContent, [...s._a.data[0].d.mz].map(Number)] : null; })()"
CHROM = "(() => { const c = E.panels.find(p => p.tab === 'ms2' && p.type === 'chrom'); return [c.el.querySelector('.ttl').textContent, c._a.sr.map(s => [s.k, (s.y || []).filter(v => v > 0).length])]; })()"
r = Run(port=8985, wd="/tmp/wd_ms23")
try:
    with sync_playwright() as p:
        for lang in ("it", "en"):
            pg = r.page(p); pg.set_viewport_size({"width": 1280, "height": 1000})
            pg.goto(f"http://127.0.0.1:{r.port}/?lang={lang}"); wait_for_js(pg, "!!document.querySelector('#pick')", 30000)
            pg.set_input_files("#pick", [str(F)])
            wait_for_js(pg, "E.files.filter(f => f.kind === 'ms2' && !f.gone).length === 2", 60000)
            wait_for_js(pg, "E.panels.some(p => p.tab === 'ms2' && p.type === 'chrom' && p._a && p._a.sr)", 60000); frames(pg, 6)
            def items(pg=pg):
                f = pg.evaluate("E.files.map(f => [f.file.split('#')[1], f.msn_level, f.lv, f.label])")
                assert f == [["MS2", 2, 2, "S_MSN-t0 · MS2"], ["MS3", 3, 3, "S_MSN-t0 · MS3"]], f
            step(f"{lang}: two items, MS2 at level 2 and MS3 at level 3", items)
            def group(pg=pg):
                g = pg.evaluate("[...document.querySelectorAll('#flst .fgh > span:not(.chv)')].map(e => e.textContent)")
                assert g == ["MS2/MS3 (Product Ion)"], g
            step(f"{lang}: the group names both levels", group)
            def ms3(lang=lang, pg=pg):
                pg.evaluate("ms2Switch(194.1)")
                wait_for_js(pg, f"(() => {{ const s = {SPEC}; return s && s[1] === 3; }})()", 30000); frames(pg, 6)
                t, sr = pg.evaluate(CHROM); k3 = pg.evaluate("E.files.find(f => f.lv === 3).k")
                assert t == "TIC · MS³ · " + {"it": "precursore", "en": "precursor"}[lang] + " 194.1", t
                assert sr == [[k3, 200]], sr
                k, lv, st, mz = pg.evaluate(SPEC)
                assert (k, lv) == (k3, 3) and st == {"it": "MS³ di 194.1", "en": "MS³ of 194.1"}[lang], (k, lv, st)
                assert any(abs(m - 122.1) < 0.3 for m in mz) and not any(abs(m - 219.1) < 0.3 for m in mz), mz
            step(f"{lang}: precursor 194.1 draws the MS3 chromatogram and spectrum", ms3)
            def ms2(pg=pg):
                pg.evaluate("ms2Switch(237.1)")
                wait_for_js(pg, f"(() => {{ const s = {SPEC}; return s && s[1] === 2; }})()", 30000); frames(pg, 6)
                k, lv, st, mz = pg.evaluate(SPEC)
                assert st.startswith("MS²") and any(abs(m - 219.1) < 0.3 for m in mz) and not any(abs(m - 122.1) < 0.3 for m in mz), (st, mz)
            step(f"{lang}: back to 237.1 the MS2 spectrum", ms2)
            def routes(pg=pg):
                k2, k3 = pg.evaluate("[E.files.find(f => f.lv === 2).k, E.files.find(f => f.lv === 3).k]")
                for k, lv, q in ((k2, 2, 237.1), (k3, 3, 194.1)):
                    sl = pg.evaluate(f"J('api/scanlist?k={k}&level={lv}').then(d => (d.sid || []).length)"); assert sl == 200, (lv, sl)
                    tr = pg.evaluate(f"getMs2Tr({k}, {lv}).then(t => t.map(x => x.q1))"); assert tr and all(abs(x - q) < 0.06 for x in tr), (lv, tr)
            step(f"{lang}: scan list and precursor table of each level", routes)
            pg.close()
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
