# e2e_raw_riaggiunto: a Thermo .raw given again (the same file: name, size, date) keeps its one sample (shell.js rawToMzml, nativo.js cached): no second row, no second copy or index in
# OPFS, both in the same page and after a reload of the page (the session restored). Synthetic .raw (crates/mzlab-lettori examples/sintetici, invented numbers).
# Needs the site and cargo: python3 tools/prepara_sito.py, then python3 tests_e2e/e2e_raw_riaggiunto.py
import os, sys, tempfile, shutil, subprocess; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
D = Path(tempfile.mkdtemp(prefix="rawri_"))
subprocess.run(["cargo", "run", "-q", "--release", "-p", "mzlab-lettori", "--example", "sintetici", "--", str(D)], cwd=str(ROOT), check=True)
RAW = D / "thermo" / "synt.raw"
# every file of OPFS mzlab/ with its size
WALK = """async () => { const out = {}; const walk = async (h, p) => { for await (const [n, c] of h.entries()) { if (c.kind === 'file') out[p + n] = (await c.getFile()).size; else await walk(c, p + n + '/'); } };
  await walk(await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), ''); return out; }"""
# upload() counted: a loading has ended when the count grows (the picker calls the global upload)
COUNT = "(() => { if (window.__ups === undefined) { const u = window.upload; window.__ups = 0; window.upload = async (...a) => { try { return await u(...a); } finally { window.__ups++; } }; } return window.__ups; })()"
IDLE = "UPBUSY === 0 && OPENNOW.busy() === 0"
def state(pg):
    o = pg.evaluate(f"({WALK})()")
    return {"rows": pg.evaluate("ST.files.map(f => f.name).sort()"),
            "copies": sorted(k for k in o if k.startswith("files/nat~")), "idx": sorted(k for k in o if k.startswith("idx/"))}
def give(pg):
    n = pg.evaluate(COUNT)
    pg.set_input_files("#pick", str(RAW))
    wait_for_js(pg, f"window.__ups > {n} && {IDLE}", 180000)
def again():
    r = Run(port=8867, wd="/tmp/wd_raw_riaggiunto")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            give(pg)
            wait_for_js(pg, f"ST.files.length === 1 && OPENNOW.isOpen(ST.files[0].name) && {IDLE}", 180000)
            first = state(pg)
            assert len(first["copies"]) == 1, first
            give(pg)
            same = state(pg)
            assert same == first, ("same page", first, same)
            pg.reload()
            wait_for_js(pg, f"typeof ST !== 'undefined' && ST.files.length === 1 && OPENNOW.isOpen(ST.files[0].name) && {IDLE}", 180000)
            give(pg)
            after = state(pg)
            assert after == first, ("after a reload", first, after)
            assert pg.evaluate("ST.files.every(f => OPENNOW.isOpen(f.name))"), "the sample is not open"
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("a .raw given again keeps one row, one copy and one index (same page, then after a reload)", again)
shutil.rmtree(D, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
