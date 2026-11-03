"""Coming back to Dati from another page shows the panels at their final place on the first frame: no slide of the panels or of the side list
(the sidebar layout of the page that was hidden is restored at once, TRANS.jump). Real motion on (the other tests run with reduced motion). Synthetic files."""
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth
tmp = Path(tempfile.mkdtemp(prefix="ritorno_"))
FILES = []
for k, n in enumerate(["R_t000min", "R_t010min"]):
    f = tmp / f"{n}.mzML"; synth.write_full(f, 10 * k, seed=k + 1); FILES.append(str(f))
# left/top of the panels and of the side list on every frame for 600 ms after the switch (an animation keeps moving them over ~200 ms)
SAMPLE = """() => new Promise(done => { const out = [], t0 = performance.now();
  const f = () => { out.push([...document.querySelectorAll('#dpanels>.pnl, #dfiles')].map(e => { const r = e.getBoundingClientRect(); return [e.id || e.className.split(' ')[0], Math.round(r.left * 10) / 10, Math.round(r.top * 10) / 10]; }));
    if (performance.now() - t0 < 600) requestAnimationFrame(f); else done(out); };
  document.querySelector('nav button[data-v=data]').click(); requestAnimationFrame(f); })"""
r = Run(port=8978, wd="/tmp/wd_data_ritorno")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        pg = r.b.new_page(viewport={"width": 1500, "height": 1000}, reduced_motion="no-preference")
        pg.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#drop"); attendi(pg, "window.qqReady === true", 60000)
        stage(pg, FILES); pg.wait_for_selector("#dpanels .pnl canvas", timeout=60000); ready(pg)
        for other in ("draw", "tpmine"):
            pg.click(f"nav button[data-v={other}]"); attendi(pg, f"S.view === '{other}'"); ready(pg); frames(pg, 3)
            before = pg.evaluate("[...document.querySelectorAll('#dpanels>.pnl, #dfiles')].map(e => { const r = e.getBoundingClientRect(); return r.width })")
            seq = pg.evaluate(SAMPLE)
            attendi(pg, "S.view === 'data'"); ready(pg)
            final = pg.evaluate("[...document.querySelectorAll('#dpanels>.pnl, #dfiles')].map(e => { const r = e.getBoundingClientRect(); return [e.id || e.className.split(' ')[0], Math.round(r.left * 10) / 10, Math.round(r.top * 10) / 10] })")
            assert len(final) >= 2, f"no Data panels found: {final}"
            bad = [s for s in seq if s != final]
            assert not bad, f"after {other}: {len(bad)}/{len(seq)} frames differ from the final place {final}; first: {bad[0]}"
            assert not pg.evaluate("document.getAnimations().filter(a => a.effect && a.effect.target && a.effect.target.closest && a.effect.target.closest('#dpanels, #dfiles')).length"), "an animation still runs on the Data panels"
        r.errs = [e for e in r.errs if e[0] != "console.warning"]
        assert not r.errs, r.errs[:3]
    print("ok: the panels do not move when coming back to Dati")
finally:
    r.close()
