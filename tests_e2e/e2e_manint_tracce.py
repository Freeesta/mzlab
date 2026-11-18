"""The card «Manual integrations» lists the windows drawn on an XIC too (engine route api/mantraces, manint::table_text), and its numbers are those of the table
the toolbar dialog shows: area, height, apex, edges, below-the-line and correction of the same window, on an XIC and on the TIC; with the optional columns on (S/N of a noise zone, w1/2,
plates, tailing, quality) the engine's figures are those the dialog computes. Steps 1 and 2 of one integration table: the dialog stays until the card has everything.
Needs the site and the .wasm (python tools/prepara_sito.py)."""
# covers: mzlab/web/integrazioni-tracce, mzlab/web/integrazioni-manuali, mzlab/web/rust-bridge
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
site = ROOT / "site"
if not (site / "index.html").exists() or not (site / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ or site/static/wasm not built: run python tools/prepara_sito.py")); sys.exit(1)
D = HERE / "shots" / "manint_tracce_tmp"; D.mkdir(parents=True, exist_ok=True)      # synthetic full scans (no laboratory data in this repository)
ds.full_scan(D / "B_FullMass-t0.mzML", 0, np.random.default_rng(2026)); ds.full_scan(D / "B_FullMass-t15.mzML", 15, np.random.default_rng(2027))
r = Run(port=8871, wd="/tmp/wd_manint_tracce")
XP = "E.panels.find(p=>p.type==='xic')"
try:
    with sync_playwright() as p:
        pg = r.page(p); errs = r.errs
        pg.wait_for_selector("#drop", timeout=120000)
        pg.set_input_files("#pick", [str(D / "B_FullMass-t0.mzML"), str(D / "B_FullMass-t15.mzML")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); pg.wait_for_function("E.panels.some(p=>p.type==='xic'&&p._a&&p._a.sr&&p._a.sr.length)", timeout=60000)
        pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode=null;p.intf='all'}})()")
        x = pg.evaluate(f"(()=>{{const s={XP}._a.sr[0];let k=0;s.y.forEach((v,i)=>{{if(v>s.y[k])k=i}});return s.x[k]}})()")
        pg.evaluate(f"(()=>{{const p={XP};intTargets(p,{x},null).forEach(s=>autoInt(p,s,{x}))}})()")
        TP = "E.panels.find(p=>p.type==='chrom')"
        pg.evaluate(f"(()=>{{const p={TP};setActive(p);p.imode=null;p.intf='all';const s=p._a.sr[0];let k=0;s.y.forEach((v,i)=>{{if(v>s.y[k])k=i}});intTargets(p,s.x[k],null).forEach(q=>autoInt(p,q,s.x[k]))}})()")
        pg.wait_for_function(f"{TP}.ints.length > 0", timeout=60000)
        pg.wait_for_function("document.querySelector('#manint') && !document.querySelector('#manint').hidden && document.querySelector('#manint-x tbody tr')", timeout=60000)
        pg.wait_for_function("MANTR.count() > 0", timeout=60000)
        def card():
            n = pg.evaluate("E.panels.reduce((a,p)=>a+p.ints.length,0)"); assert n >= 1
            assert pg.locator("#manint-x tbody tr").count() == n, (n, pg.locator("#manint-x tbody tr").count())
            assert f"({n})" in pg.inner_text("#manint-h")
            heads = pg.locator("#manint-x th").all_inner_texts()
            for h in ["Panel", "Channel", "Sample", "Conc.", "Below the line", "Correction"]: assert h in heads, (h, heads)
        step("the card has a row per window drawn on the XIC, with panel, conc., below and correction columns", card)
        def parity():
            dlg = pg.evaluate("allInts().map(r => ({panel: r.panel, file: r.file, a: r.a, b: r.b, area: r.area, height: r.height, rt: r.rt, below: r.below || 0, corr: r.corr || ''}))")
            rows = pg.evaluate("(() => { const [h, ...b] = MANINT.rows(MANTR.csv()); const c = n => h.indexOf(n); return b.map(r => ({panel: r[c('panel')], below: +r[c('below')] || 0, corr: r[c('correction')], sample: r[c('sample')], a: +r[c('start_min')], b: +r[c('end_min')], area: +r[c('area')], height: +r[c('height')], rt: +r[c('apex_rt_min')]})); })()")
            assert len(dlg) == len(rows) and dlg, (dlg, rows)
            for d in dlg:
                r = next(r for r in rows if r["panel"] == d["panel"] and r["sample"] in d["file"] and abs(r["a"] - min(d["a"], d["b"])) < 6e-4)
                for k in ("area", "height"): assert abs(r[k] - d[k]) <= 1e-4 * max(abs(d[k]), 1), (k, d, r)
                assert abs(r["rt"] - d["rt"]) < 6e-4, (d, r)
                assert abs(r["below"] - d["below"]) <= 1e-4 * max(abs(d["below"]), 1) and r["corr"] == d["corr"], (d, r)
            assert len({d["panel"] for d in dlg}) >= 2, dlg      # the XIC and the TIC are both compared
        step("area, height and apex of the card are those of the dialog table", parity)
        def figures():
            # a noise zone for each trace (the first tenth of it), the three optional groups on; then the card asks the engine again
            pg.evaluate("""(() => { for (const p of E.panels) { if (!p.ints || !p.ints.length || ESP.mrm(p)) continue;
              const it = p.ints[0], s = p._a.sr.find(q => q.key === it.key), n = Math.floor(s.x.length / 10); setNoiseZone(p, it, [s.x[0], s.x[n]]); }
              COMANDI.fai('peaks.settings', COMANDI.GLOBALE, () => { INTOPT.sn = INTOPT.par = INTOPT.q = true; }); E.panels.forEach(p => draw(p)); })()""")
            pg.wait_for_function("MANINT.rows(MANTR.csv())[0].includes('gaussian') && MANINT.rows(MANTR.csv())[0].includes('sn_zone')", timeout=60000)
            heads = pg.locator("#manint-x th").all_inner_texts()
            for h in ["S/N (noise zone)", "w½ (min)", "N (plates)", "Tailing (USP, 5%)", "Gaussian similarity", "Asymmetry 10%", "Noise score"]: assert h in heads, (h, heads)
            dlg = pg.evaluate("""(async () => { const rs = allInts().map(r => ({...r, ...intExtra(r)})); const out = [];
              for (const r of rs) { const q = await qualityOf(r); out.push({panel: r.panel, file: r.file, a: Math.min(r.a, r.b), sn: r.sn, w50: r.w50, N: r.N, T: r.T, qg: q && q.gaussian, qa: q && q.asymmetry, qn: q && q.noise}); } return out; })()""")
            rows = pg.evaluate("""(() => { const [h, ...b] = MANINT.rows(MANTR.csv()); const c = n => h.indexOf(n); return b.map(r => ({panel: r[c('panel')], sample: r[c('sample')], a: +r[c('start_min')],
              sn: r[c('sn_zone')], w50: r[c('w_half_min')], N: r[c('plates')], T: r[c('tailing')], qg: r[c('gaussian')], qa: r[c('asymmetry')], qn: r[c('noise_score')]})); })()""")
            def same(x, y, tol, name, d):
                if x is None or x != x: assert y == "", (name, d, y); return
                assert y != "" and abs(float(y) - x) <= tol, (name, x, y, d)
            for d in dlg:
                r = next(r for r in rows if r["panel"] == d["panel"] and r["sample"] in d["file"] and abs(r["a"] - d["a"]) < 6e-4)
                same(d["sn"], r["sn"], 0.051, "S/N", d); same(d["w50"], r["w50"], 6e-4, "w1/2", d); same(d["N"], r["N"], 0.51, "plates", d); same(d["T"], r["T"], 0.0051, "tailing", d)
                same(d["qg"], r["qg"], 6e-4, "gaussian", d); same(d["qa"], r["qa"], 0.0051, "asymmetry", d); same(d["qn"], r["qn"], 0.0051, "noise score", d)
            assert any(r["sn"] != "" for r in rows) and any(r["N"] != "" for r in rows), rows
        step("with the optional columns on, S/N, w1/2, plates, tailing and quality are the dialog's figures", figures)
        def excel():
            with pg.expect_download() as dl: pg.click("#manint [data-xlsx]")
            assert dl.value.suggested_filename.endswith(".xlsx"), dl.value.suggested_filename
        step("the card exports its tables to Excel", excel)
        bad = [e for e in errs if e[0] in ("pageerror", "console.error")]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(bad[:3])) if bad else None)
finally:
    r.close()
for s in steps: print("  ", s)
