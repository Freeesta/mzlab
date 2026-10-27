"""Channel selector of the chromatogram panel (canali.js): the tree, several channels = several traces, the same channel across files, the default
channel of each type of experiment, the XIC inside a channel, the keyboard, the size of the targets and no request left unanswered by the engine."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_canali.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "canali_tmp"; D.mkdir(parents=True, exist_ok=True)
ds.make(D / "s")
S = D / "s"
ds.full_scan(D / "DAD-t0.mzML", 0, np.random.default_rng(3), dad=True)
def load(pg, files):
    stage(pg, files); pg.click("text=Carica dati"); ready(pg); attendi(pg, f"E.files.length >= {len(files)}")
def chans(pg, k): return pg.evaluate(f"CANALI.canali({k}).then(j => j.canali.map(c => c.fam + ':' + c.omo))")
def tipo(pg, k): return pg.evaluate(f"CANALI.canali({k}).then(j => j.tipo.proposto)")
def idx(pg, label): return pg.evaluate(f"E.files.find(f => (f.label || f.name || '').includes({label!r})).k")
r = Run(port=8873, wd="/tmp/wd_canali")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [S / "canali" / "K_MIRATO-t0.mzML", S / "B_FullMass-t0.mzML", S / "B_FullMass-t5.mzML", S / "canali" / "K_BLOCCHI-t0.mzML", S / "B_MRM-t0.mzML", S / "C_IDA-t0.mzML", D / "DAD-t0.mzML"])
        def types():
            exp = {"K_MIRATO-t0": "lc-ms2-mirato", "K_BLOCCHI-t0": "infusione-msn", "B_MRM-t0": "lc-mrm", "B_FullMass-t0": "lc-full"}
            for lab, t in exp.items():
                k = idx(pg, lab)
                assert tipo(pg, k) == t, (lab, tipo(pg, k))
        step("the engine proposes the type of experiment", types)
        def defaults():
            want = {"K_MIRATO-t0": ("ms", 2), "K_BLOCCHI-t0": ("ms", 3), "B_MRM-t0": ("srm", 2), "B_FullMass-t0": ("tic", 0)}
            for lab, (fam, liv) in want.items():
                c = pg.evaluate(f"CANALI.iniziale(E.files.find(f => (f.label || f.name || '').includes({lab!r}))).then(c => c && [c.fam, c.liv])")
                assert c == [fam, liv], (lab, c)
        step("default channel by type (table in esperimenti.js)", defaults)
        def ida():
            k = idx(pg, "C_IDA-t0"); v = chans(pg, k)
            assert any(c.startswith("ms:") for c in v) and any(c.startswith("tic") for c in v), v
        step("EMS/EPI file: channels of scans", ida)
        def dad():
            k = idx(pg, "DAD-t0"); assert tipo(pg, k) in ("lc-full", "misto", "solo-uv"), tipo(pg, k)
        step("DAD file opens", dad)
        pg.evaluate("setTab('full')"); ready(pg)
        def tree():
            pg.click("[data-o=canali]"); attendi(pg, "document.querySelector('.can-pop [role=treeitem]')")
            n = pg.evaluate("document.querySelectorAll('.can-pop [role=treeitem]').length"); assert n >= 5, n
            assert pg.evaluate("document.querySelector('.can-pop [role=tree]').getAttribute('aria-label')")
            lv = pg.evaluate("[...document.querySelectorAll('.can-pop [role=treeitem]')].map(e => e.getAttribute('aria-level'))"); assert "2" in lv, lv
            t = pg.evaluate("[...document.querySelectorAll('.can-pop [role=treeitem]')].some(e => /@/.test(e.title) || /^ms:/.test(e.title))"); assert t
        step("tree with levels, aria and the exact id in the title", tree)
        def two():
            before = pg.evaluate("E.panels.find(p => p.type==='chrom').canali")
            pg.click(".can-pop [role=treeitem][data-omo^='ms|1']"); ready(pg); frames(pg, 6)
            c = pg.evaluate("E.panels.find(p => p.type==='chrom').canali"); assert c and len(c) == 2, (before, c)
            ns = pg.evaluate("E.panels.find(p => p.type==='chrom')._a.sr.length"); assert ns >= 4, ns
        step("two channels over several files = more traces", two)
        def table():
            pg.evaluate("TDATI_P.update(E.panels.find(p => p.type==='chrom'))")
            names = pg.evaluate("E.panels.find(p => p.type==='chrom')._a.sr.map(s => s.name)")
            assert any("MS" in n and "B_FullMass-t0" in n for n in names), names
        step("one trace per file and channel (named for the data table)", table)
        def keys():
            pg.focus(".can-pop [role=treeitem][aria-selected=true]")
            pg.keyboard.press("ArrowDown"); pg.keyboard.press(" "); ready(pg)
            c = pg.evaluate("E.panels.find(p => p.type==='chrom').canali"); assert c and len(c) >= 2, c
            pg.keyboard.press("Escape"); assert pg.evaluate("document.querySelector('.can-pop').hidden")
        step("keyboard: arrows, space, Escape", keys)
        def size():
            pg.click("[data-o=canali]"); attendi(pg, "document.querySelector('.can-pop [role=treeitem]')")
            h = pg.evaluate("Math.min(...[...document.querySelectorAll('.can-pop .can-row')].filter(e => e.offsetParent).map(e => e.getBoundingClientRect().height))"); assert h >= 24, h
            b = pg.evaluate("document.querySelector('[data-o=canali]').getBoundingClientRect().height"); assert b >= 24, b
            pg.keyboard.press("Escape")
        step("targets of at least 24 px", size)
        def miss():
            pg.evaluate("(() => { const p = E.panels.find(p => p.type==='chrom'); p.canali = [CANALI.albero(E.files.filter(f=>f.kind==='full')).find(n => n.omo.startsWith('ms|2')).omo]; draw(p); })()"); ready(pg); frames(pg, 6)
            t = pg.evaluate("E.panels.find(p => p.type==='chrom').leg.innerText"); assert "—" in t, t
        step("a file without the channel: «—» in the legend", miss)
        def xic():
            d = pg.evaluate("""(async () => { const f = E.files.find(f => (f.label || f.name || '').includes('K_MIRATO-t0')); const c = (await CANALI.canali(f.k)).canali.find(c => c.liv === 2);
              const a = await getXic(f.k, 129.96, 0.5, 2, c.id); const b = await getXic(f.k, 129.96, 0.5, 2); return [a.rt.length, b.rt.length]; })()""")
            assert d[0] > 0 and d[0] <= d[1], d
        step("XIC inside a channel (api/xic canale=)", xic)
        def restore():
            rec = pg.evaluate("panelRec(E.panels.find(p => p.type==='chrom')).canali"); assert rec, rec
        step("the panel saves the omologo, not the index", restore)
        def miss_routes():
            assert pg.evaluate("(window.MZLAB_LR_MISS||[]).length") == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("no request left unanswered", miss_routes)
        r.b.close(); pg = r.page(p)
        load(pg, [S / "B_MRM-t0.mzML", S / "B_MRM-t5.mzML"])
        def mrm():
            pg.evaluate("setTab('mrm')"); ready(pg); attendi(pg, "document.querySelector('[data-o=canali]')")
            pg.click("[data-o=canali]"); attendi(pg, "document.querySelector('.can-pop [role=treeitem]')")
            rows = pg.evaluate("document.querySelectorAll('.can-pop [role=treeitem]').length"); assert rows == 2, rows
            pg.click(".can-pop [role=treeitem] >> nth=1"); ready(pg); frames(pg, 6)
            tr = pg.evaluate("E.panels.find(p => p.type==='mrm').tr"); assert isinstance(tr, list) and len(tr) == 1, tr
        step("MRM panel: srm channels give the transitions", mrm)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
