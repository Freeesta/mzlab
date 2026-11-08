# e2e_chiusura_rapida: a page closed right after a file opened, before the engine stored the notebook, keeps its last notebook in localStorage (nb-pendiente.js);
# at the next visit the engine boot (lr.js) reopens the files of that copy, not only those of the older one the engine holds (the sample table showed every row ticked
# while only some files opened, and adding them again did nothing). The lost write is made certain by dropping the POSTs of api/notebook after the first file is stored.
# Example files only. Needs the site and the .wasm: python3 tools/build_site.py, then python3 tests_e2e/e2e_chiusura_rapida.py
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
A, B = "FullScan_t0", "FullScan_t5"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
OPEN = "E.files.filter(f => !f.gone).map(f => f.file.split('#')[0]).sort()"
def reopen_all():
    r = Run(port=8855, wd="/tmp/wd_chiusura_rapida")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            stage(pg, [str(EX / f"{A}.mzML")])
            open_first(pg); ready(pg); saved(pg)
            # from here the engine never gets the notebook: the state of a tab closed before its write landed
            pg.evaluate("""() => { const f = window.fetch; window.fetch = (u, o) => (String(u).includes('api/notebook') && o && o.method === 'POST') ? Promise.reject(new Error('lost')) : f(u, o); }""")
            pg.set_input_files("#pick", [str(EX / f"{B}.mzML")])
            wait_for_js(pg, f"E.files.filter(f => !f.gone).length === 2 && UPBUSY === 0 && OPENNOW.busy() === 0", 120000)
            stored = pg.evaluate("fetch('api/notebook').then(r => r.json()).then(n => (n.session || []).map(x => x.file.split('#')[0]))")
            assert sorted(set(stored)) == [f"{A}.mzML"], f"the engine must still hold the older notebook: {stored}"
            pg.reload()
            pg.wait_for_function("typeof E !== 'undefined' && E.files.filter(f => !f.gone).length >= 1", timeout=120000)
            wait_for_js(pg, "UPBUSY === 0 && OPENNOW.busy() === 0", 120000)
            got = pg.evaluate(OPEN)
            assert got == [f"{A}.mzML", f"{B}.mzML"], f"after the reload only {got} opened (the rows: {pg.evaluate('ST.files.map(r => r.name)')})"
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("the files of the notebook kept in localStorage open again after the reload", reopen_all)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
