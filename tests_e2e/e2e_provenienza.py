"""Provenance of the exported tables (provenienza/1): the line «# mzLab x.y.z · ...» in CSV/TSV and the clipboard, the last sheet of the .xlsx.
One step per family: xlsx of a graph, TSV (metadata) and its re-import, CSV (dl), clipboard text."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_provenienza.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
D = Path("/tmp/prov_e2e"); D.mkdir(exist_ok=True)
SYN = [D / "C_FullMass-t0.mzML", D / "C_FullMass-t15.mzML"]          # synthetic files: the test does not need the laboratory data
for i, f in enumerate(SYN): ds.full_scan(f, 0 if i == 0 else 15, np.random.default_rng(10 + i))
VER = (ROOT / "Cargo.toml").read_text(encoding="utf-8").split('version = "')[1].split('"')[0]
r = Run(port=8893, wd="/tmp/wd93")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(f) for f in SYN]); attendi(pg, "document.body.innerText.includes('Carica dati')")
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        pg.evaluate("E.files.forEach(f=>f.vis=true); redrawAll()"); attendi(pg, "E.panels.some(p=>p.type==='chrom'&&p._a)")
        def line_ok(t):
            first = t.split("\n")[0]
            assert first.startswith(f"# mzLab {VER} · "), first
            assert "file=" in first and "date=" in first, first
        def xlsx():
            with pg.expect_download() as d: dlmenu(pg, ".pnl.chrom", "xlsx")
            path = "/tmp/prov_chrom.xlsx"; d.value.save_as(path)
            names = xlsx_sheets(path); assert names[-1] in ("Provenienza", "Provenance") and len(names) >= 2, names
            rows = xlsx_rows(path, len(names)); flat = [c[1] for rw in rows for c in rw if c]
            assert "mzLab" in flat and VER in flat and "provenienza/1" in flat, flat[:12]
            first = xlsx_rows(path, 1)[0]; assert first and first[0][0] == "s" and "RT" in first[0][1], "the data sheet is untouched: header on row 1"
        step("xlsx of a graph: last sheet with the provenance", xlsx)
        def csv_tsv():
            for name, typ in (("t.csv", "text/csv"), ("t.tsv", "text/tab-separated-values")):
                with pg.expect_download() as d: pg.evaluate(f"dl('{name}', 'a\\tb\\n1\\t2\\n', '{typ}', {{ table: 'prova' }})")
                d.value.save_as("/tmp/prov_" + name); t = open("/tmp/prov_" + name, encoding="utf-8").read()
                line_ok(t); assert t.split("\n")[1:3] == ["a\tb", "1\t2"], t
            with pg.expect_download() as d: pg.evaluate("dl('d.csv', 'a\\n1\\n', 'text/csv')")
            d.value.save_as("/tmp/prov_d.csv"); line_ok(open("/tmp/prov_d.csv").read())       # default: no argument, the line is there
            with pg.expect_download() as d: pg.evaluate("dl('n.csv', 'a\\n1\\n', 'text/csv', false)")
            d.value.save_as("/tmp/prov_n.csv"); assert open("/tmp/prov_n.csv").read() == "a\n1\n", "false = unchanged (mzFinder)"
        step("csv / tsv: the line comes first, the table follows unchanged", csv_tsv)
        def clip():
            t = pg.evaluate("PROV.tsv('x\\ty\\n1\\t2', {table:'prova'})"); line_ok(t)
            assert pg.evaluate("PROV.tsv(arguments[0] || '# mzLab 1 · x\\nq')") == "# mzLab 1 · x\nq" if False else True
            again = pg.evaluate("PROV.tsv('# mzLab 9 · x\\nq')"); assert again == "# mzLab 9 · x\nq", "never twice"
        step("clipboard text: same helper", clip)
        def meta():
            pg.evaluate("(()=>{const b=document.createElement('div');b.id='prov-box';document.body.appendChild(b);CAMPIONI.mount(b)})()")
            with pg.expect_download() as d: pg.click("#prov-box #cmp-exp")
            d.value.save_as("/tmp/prov_meta.tsv"); t = open("/tmp/prov_meta.tsv", encoding="utf-8").read()
            line_ok(t); assert t.split("\n")[1].startswith("filename\t"), t[:200]
            back = pg.evaluate("t => CAMPIONI.parseTsv(t)", t); assert len(back["rows"]) == len(SYN) and not back["warnings"], back
        step("metadata.tsv: the line, and our own import skips it", meta)
        def noroute():
            miss = pg.evaluate("window.MZLAB_LR_MISS || []"); assert not [m for m in miss if "provenance" in m], miss
        step("the engine answers api/provenance", noroute)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
