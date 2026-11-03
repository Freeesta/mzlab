# e2e_ms2_titoli: MS2 panels read «MS² 229.1» (IT and EN, not truncated at 1280 px), the precursor menu lists numbers only, MS2 files are never ticked together
# (one MS2 trace per panel, no overlay control at level 2) and Full scan and MS2 never share a panel.
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
FILES = ["B_FullMass-t0", "B_FullMass-t15", "B_MS2-t15", "B_MS2-t45"]
r = Run(port=8879, wd="/tmp/wd_ms2tit")
try:
    with sync_playwright() as p:
        for lang in ("it", "en"):
            pg = r.page(p); pg.set_viewport_size({"width": 1280, "height": 1000})
            pg.goto(f"http://127.0.0.1:{r.port}/?lang={lang}"); attendi(pg, "!!document.querySelector('#pick')", 30000)
            pg.set_input_files("#pick", [mz(f) for f in FILES]); frames(pg, 3)
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=4", timeout=30000)
            pg.click("text=Carica dati" if lang == "it" else "text=Load data"); ready(pg)
            open_kind(pg, "ms2"); attendi(pg, "E.panels.filter(p => p.tab === 'ms2').length >= 2", 30000); frames(pg, 6)
            def titles(lang=lang, pg=pg):
                t = pg.evaluate("E.panels.filter(p => p.tab === 'ms2').map(p => p.el.querySelector('.ttl').textContent)")
                assert t == ["MS² 229.1", "MS² 229.1"], t
                cut = pg.evaluate("E.panels.filter(p => p.tab === 'ms2').map(p => { const e = p.el.querySelector('.ttl'); return e.scrollWidth > e.clientWidth + 1; })")
                assert not any(cut), ("title truncated", cut)
                shown = " ".join(pg.evaluate("[...document.querySelectorAll('.pnl .ttl, .pnl select[data-o=prec] option, .pnl .leg, .pnl .leg2')].map(e => e.textContent)"))
                for bad in ("precursore", "precursor ", "prec."): assert bad not in shown, (bad, shown)
            step(f"{lang}: MS2 panel titles read «MS² 229.1», whole", titles)
            def menu(pg=pg):
                for o in pg.evaluate("[...document.querySelectorAll('.pnl select[data-o=prec]')].map(s => [...s.options].map(o => o.textContent))"):
                    assert o[1:] == ["229.1", "305"] and len(o[0]) <= 8, o
            step(f"{lang}: the precursor menu keeps short values", menu)
            def one_ms2(pg=pg):
                pg.evaluate("setVisOf(E.files.filter(f => f.kind === 'ms2'), true)"); frames(pg, 6)
                v = pg.evaluate("E.files.filter(f => f.kind === 'ms2' && f.vis).length"); assert v == 1, ("two MS2 files ticked", v)
                n = pg.evaluate("E.panels.filter(p => p.tab === 'ms2' && p.type === 'chrom').map(p => (p._a && p._a.sr || []).length)"); assert n == [1], n
            step(f"{lang}: ticking the MS2 files together keeps one, one trace per panel", one_ms2)
            def no_mix(pg=pg):
                pg.evaluate("setVisOf(E.files.filter(f => f.kind === 'full'), true)"); frames(pg, 6)
                bad = pg.evaluate("E.panels.filter(p => p._a && p._a.sr).filter(p => p._a.sr.some(s => E.files[s.k] && E.files[s.k].kind !== (p.tab || E.tab))).map(p => p.title)")
                assert not bad, bad
                tabs = pg.evaluate("[...new Set(E.panels.map(p => p.tab))].sort()"); assert tabs == ["full", "ms2"], tabs
            step(f"{lang}: Full scan and MS2 files are on separate panels", no_mix)
            def no_overlay(pg=pg):
                has = pg.evaluate("E.panels.filter(p => p.tab === 'ms2' && p.type === 'spec').every(p => !p.el.querySelector('input[data-o=all]'))"); assert has, "overlay control on an MS2 spectrum"
                pg.evaluate("""() => { const q = E.panels.find(p => p.tab === 'full' && p.type === 'spec'); q.all = true; q.level = 2; ctl(q); }""")
                st = pg.evaluate("(() => { const q = E.panels.find(p => p.tab === 'full' && p.type === 'spec'); const l = q.el.querySelector('.ovb'); return [q.all, !l || l.classList.contains('dis')]; })()")
                assert st == [False, True], st
            step(f"{lang}: no overlay at level MS2", no_overlay)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
