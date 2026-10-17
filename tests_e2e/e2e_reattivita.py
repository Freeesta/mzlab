"""Tabui, panel of reactivity of the parent (TP_Mine/lazy/reattivita.js + viewer3d.js, docs/agenti/reattivita.md): administrator mode only.
Without «qqq.admin» there is no TPREATT, no panel and no .wasm «chem» request. With it, a hand-written `reatt/1` (paracetamol, geometry and values invented here, not a calculation) gives the six cards, the
table per atom with as many rows as `atoms`, the threshold (slider and number linked) that filters the rows and the atoms of the viewer, the buttons and the keys that rotate and zoom (the state of the view
changes), the switch of the H, no automatic rotation, the exports (XYZ has N + 2 lines), the states of the calculation, and the 2D drawing (a halo under each heavy atom). The order of the heavy atoms of
OpenChemLib is checked against `heavy_index` for paracetamol and carbamazepine (same order) and for a relabelled graph (the mapping by element and neighbours). Run from the repo root: python3 tests_e2e/e2e_reattivita.py"""
import json, math, os, shutil, subprocess, sys, time, tempfile, re
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port; PORT = free_port()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:500]))

def smiles_graph(smi):
    """heavy atoms in the order of the SMILES and their bonds (a minimal reader for the SMILES of this test: branches, rings, = and #, aromatic lowercase, bracket atoms)"""
    toks = re.findall(r"\[[^\]]+\]|Cl|Br|[BCNOSPFI]|[bcnosp]|\(|\)|=|#|-|\d", smi)
    els, bonds, stack, prev, order, ring = [], [], [], None, 1, {}
    for t in toks:
        if t == "(": stack.append(prev)
        elif t == ")": prev = stack.pop()
        elif t in "=#-": order = {"=": 2, "#": 3, "-": 1}[t]
        elif t.isdigit():
            if t in ring: bonds.append((ring.pop(t), prev, order)); order = 1
            else: ring[t] = prev
        else:
            els.append(re.sub(r"[\[\]]|H\d*|[+-]\d*|@+", "", t).capitalize()); k = len(els) - 1
            if prev is not None: bonds.append((prev, k, order))
            prev, order = k, 1
    return els, bonds

def heavy_only(smi):
    els, bonds = smiles_graph(smi)
    return {"format": "reatt", "version": 1, "smiles": smi,
            "atoms": [{"i": k, "el": e, "heavy_index": k, "x": 0, "y": 0, "z": 0, "q": 0, "f_plus": 0, "f_zero": 0, "f_minus": 0, "dual": 0} for k, e in enumerate(els)],
            "bonds": [{"a": a, "b": b, "order": o} for a, b, o in bonds]}

# paracetamol CC(=O)Nc1ccc(O)cc1: heavy atoms 0 CH3, 1 C=O, 2 O, 3 N, 4 c(N), 5 c, 6 c, 7 c(OH), 8 O, 9 c, 10 c; then nine H
def paracetamol():
    ring = {4: 0, 5: 60, 6: 120, 7: 180, 9: 240, 10: 300}
    xyz = {k: (1.4 * math.cos(math.radians(a)), 1.4 * math.sin(math.radians(a)), 0.0) for k, a in ring.items()}
    xyz.update({3: (2.85, 0, 0), 1: (3.6, 1.2, 0), 2: (3.1, 2.3, 0), 0: (5.0, 1.1, 0.3), 8: (-2.75, 0, 0)})
    hs = [(5, 2.48, 60), (6, 2.48, 120), (9, 2.48, 240), (10, 2.48, 300)]
    for n, (host, rr, a) in enumerate(hs): xyz[11 + n] = (rr * math.cos(math.radians(a)), rr * math.sin(math.radians(a)), 0.0)
    xyz.update({15: (3.3, -0.9, 0), 16: (-3.2, 0.85, 0.2), 17: (5.4, 2.0, 0.3), 18: (5.4, 0.6, 1.2), 19: (5.4, 0.5, -0.7)})
    els = ["C", "C", "O", "N", "C", "C", "C", "C", "O", "C", "C"] + ["H"] * 9
    fm = [0.01, 0.02, 0.06, 0.08, 0.03, 0.10, 0.04, 0.02, 0.12, 0.10, 0.04] + [0.005] * 9
    atoms = [{"i": k, "el": els[k], "heavy_index": k if k < 11 else None, "x": xyz[k][0], "y": xyz[k][1], "z": xyz[k][2], "q": round(-0.1 + 0.01 * k, 3),
              "f_plus": round(fm[k] * 0.5, 4), "f_zero": round(fm[k] * 0.8, 4), "f_minus": fm[k], "dual": round(fm[k] * 0.3, 4)} for k in range(20)]
    _, hv = smiles_graph("CC(=O)Nc1ccc(O)cc1")
    bonds = [{"a": a, "b": b, "order": o} for a, b, o in hv] + [{"a": 5, "b": 11, "order": 1}, {"a": 6, "b": 12, "order": 1}, {"a": 9, "b": 13, "order": 1}, {"a": 10, "b": 14, "order": 1},
             {"a": 3, "b": 15, "order": 1}, {"a": 8, "b": 16, "order": 1}, {"a": 0, "b": 17, "order": 1}, {"a": 0, "b": 18, "order": 1}, {"a": 0, "b": 19, "order": 1}]
    return {"format": "reatt", "version": 1, "engine": "mzlab-chem/0-test", "method": "GFN2-xTB", "solvent": "water", "smiles": "CC(=O)Nc1ccc(O)cc1", "atoms": atoms, "bonds": bonds,
            "hbonds": [{"donor_h": 16, "acceptor": 2}], "global": {"homo_ev": -8.1, "lumo_ev": -1.2, "gap_ev": 6.9, "ip_ev": 8.1, "ea_ev": 1.2, "mu_ev": -4.65, "eta_ev": 3.45, "omega_ev": 3.13},
            "conformers": {"n": 3, "chosen": 0, "f_minus_spread": 0.004}, "scc": {"converged": [True, True, True], "iterations": [9, 11, 11]},
            "notes": ["Test data written by hand."], "cite": ["Bannwarth et al. GFN2-xTB doi:10.1021/acs.jctc.8b01176", {"text": "Parr, Szentpaly, Liu: electrophilicity index", "doi": "10.1021/ja983494x"}]}

REATT = paracetamol(); N = len(REATT["atoms"])
tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
subprocess.run(cmd, check=True)
from lib_sito import add_wasm
try: add_wasm(tmp / "site")          # the panel does not need the engine of the data: where the .wasm cannot be built the page starts without it
except Exception as e: print("SKIP .wasm of the engine not available:", str(e)[:80])
site = tmp / "site"
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
_t0 = time.time()
while time.time() - _t0 < 30:
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: pass
errs = []; reqs = []
MOUNT = "r => { document.querySelector('#tpr-host')?.remove(); const h = document.createElement('div'); h.id = 'tpr-host'; document.querySelector('.tp').appendChild(h); window.__dl = []; window.dl = (n, t, ty) => window.__dl.push([n, t, ty]); return window.TPREATT.mount(h, r); }"
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(admin):
            c = b.new_context(viewport={"width": 1400, "height": 1600}); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.on("request", lambda r: reqs.append(r.url))
            pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 and "logo.svg" not in r.url and "/vendor/" not in r.url and "wasm-feat/manifest" not in r.url else None)
            if admin: pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000); lib.attendi(pg, "typeof window.QTOOLS !== 'undefined'", 60000)
            return pg
        pg = fresh(False)
        step("without qqq.admin there is no TPREATT, no panel and no .wasm chem", lambda: (_ for _ in ()).throw(AssertionError("TPREATT or panel present")) if pg.evaluate("typeof window.TPREATT !== 'undefined' || !!document.querySelector('.tpr')")
             or any("chem" in u and u.endswith(".wasm") for u in reqs) else None)
        pg.context.close()

        pg = fresh(True)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000); pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        pg.wait_for_function("window.QTOOLS && window.QTOOLS.list().some(t => t.id === 'tpmine' && !t.ph)", timeout=60000)
        def lazy_not_at_start():
            assert pg.evaluate("typeof window.TPREATT === 'undefined' && typeof window.TPV3D === 'undefined' && typeof window.TPLAZY.load === 'function'"), "loaded at the start"
            assert not any(u.endswith(("reattivita.js", "viewer3d.js")) for u in reqs), [u for u in reqs if "reattivita" in u or "viewer3d" in u]
            idx = pg.evaluate("fetch(window.QTOOLS.ctx.base + 'mzfinder/indice.json').then(r => r.json())")
            assert {"reattivita.js", "viewer3d.js"} <= set(idx["lazy"]) and not any(n in idx["js"] for n in idx["lazy"]), idx
        step("the two scripts are not loaded at the start of Tabui (TP_Mine/lazy, not in js)", lazy_not_at_start)
        pg.fill("#tp-smi", "CC(=O)Nc1ccc(O)cc1")
        pg.wait_for_function("typeof window.TPREATT !== 'undefined' && typeof window.TPV3D !== 'undefined'", timeout=60000)
        step("the first structure loads them once, from mzfinder/lazy/", lambda: (_ for _ in ()).throw(AssertionError(reqs)) if sum(u.endswith("lazy/reattivita.js") for u in reqs) != 1 else None)
        def cards():
            assert pg.evaluate(MOUNT, REATT)
            tabs = pg.evaluate("[...document.querySelectorAll('#tpr-host [role=tab]')].map(b => b.textContent)")
            assert tabs == ["f(−)", "f(0)", "f(+)", "Info", "Notes", "JSON"], tabs
            assert pg.evaluate("document.querySelector('#tpr-host [role=tab][aria-selected=true]').textContent") == "f(−)", "f(−) is not the default card"
        step("six cards, f(−) first", cards)
        def table():
            assert pg.evaluate("document.querySelectorAll('#tpr-host .tpr-tab tbody tr').length") == N
            assert pg.evaluate("document.querySelectorAll('#tpr-host .tpr-tab tbody tr.over').length") == 5, "atoms at or above 0.05"
            pg.click('#tpr-host button[data-sort="f_minus"]')
            first = pg.evaluate("document.querySelector('#tpr-host .tpr-tab tbody tr td:nth-child(6)').textContent")
            pg.click('#tpr-host button[data-sort="f_minus"]'); last = pg.evaluate("document.querySelector('#tpr-host .tpr-tab tbody tr td:nth-child(6)').textContent")
            assert float(first) <= float(last) and float(last) == 0.12, (first, last)
            assert pg.evaluate("[...document.querySelectorAll('#tpr-host .tpr-tab tbody tr.over')].every(r => r.lastElementChild.textContent.includes('▲'))"), "no sign on the rows above the threshold"
        step("table per atom: one row per atom, rows above the threshold marked, sortable", table)
        def thr():
            pg.fill("#tpr-num", "0.09"); pg.wait_for_function("document.querySelectorAll('#tpr-host .tpr-tab tbody tr.over').length === 3")
            o = pg.evaluate("document.querySelector('#tpr-host canvas').dataset.over"); assert o == "3", "atoms over in the viewer: " + o
            assert float(pg.input_value("#tpr-rng")) == 0.09, "slider and number are not linked"
            pg.fill("#tpr-rng", "0.05") if False else pg.evaluate("() => { const r = document.querySelector('#tpr-rng'); r.value = 0.02; r.dispatchEvent(new Event('input')); }")
            pg.wait_for_function("document.querySelectorAll('#tpr-host .tpr-tab tbody tr.over').length === 10"); assert pg.input_value("#tpr-num") == "0.02"
            pg.fill("#tpr-num", "0.05"); pg.wait_for_function("document.querySelector('#tpr-host canvas').dataset.over === '5'")
        step("the threshold (slider and number) filters the table and the atoms of the viewer", thr)
        def view():
            s = lambda: pg.evaluate("({...document.querySelector('#tpr-host canvas').dataset})")
            a = s(); pg.click('#tpr-host [data-v3="right"]'); b2 = s(); assert a["yaw"] != b2["yaw"], "rotate right"
            pg.click('#tpr-host [data-v3="up"]'); assert s()["pitch"] != b2["pitch"], "rotate up"
            z = s()["zoom"]; pg.click('#tpr-host [data-v3="in"]'); assert float(s()["zoom"]) > float(z), "zoom in"
            pg.click('#tpr-host [data-v3="out"]'); pg.click('#tpr-host [data-v3="out"]'); assert float(s()["zoom"]) < float(z), "zoom out"
            pg.click('#tpr-host [data-v3="reset"]'); assert s()["yaw"] == a["yaw"] and s()["zoom"] == a["zoom"], "reset"
            pg.focus("#tpr-host canvas"); pg.keyboard.press("ArrowLeft"); assert s()["yaw"] != a["yaw"], "key left"
            pg.keyboard.press("+"); assert float(s()["zoom"]) > 1, "key plus"; pg.keyboard.press("0"); assert s()["yaw"] == a["yaw"]
            pg.click('#tpr-host input[data-v3="h"]'); assert s()["showH"] == "false" and int(s()["over"]) == 5, s()
            pg.click('#tpr-host input[data-v3="h"]'); assert s()["showH"] == "true"
            y0 = s()["yaw"]; pg.evaluate("new Promise(r => setTimeout(r, 600))"); assert s()["yaw"] == y0, "the viewer rotates by itself"
            assert pg.evaluate("document.querySelector('#tpr-host canvas').getAttribute('aria-label').includes('card f(−)')")
            sizes = pg.evaluate("[...document.querySelectorAll('#tpr-host .v3-bar button')].map(b => { const r = b.getBoundingClientRect(); return Math.min(r.width, r.height); })")
            assert min(sizes) >= 24, sizes
        step("buttons and keys rotate and zoom, H switch, no automatic rotation", view)
        def cardsswitch():
            pg.click("#tpr-t1"); pg.wait_for_selector("#tpr-host canvas"); assert pg.evaluate("document.querySelector('#tpr-host canvas').getAttribute('aria-label').includes('card f(0)')")
            pg.click("#tpr-t2"); assert pg.evaluate("document.querySelector('#tpr-host canvas').getAttribute('aria-label').includes('card f(+)')")
            pg.click("#tpr-t3"); t = pg.inner_text("#tpr-host #tpr-p"); assert "GFN2-xTB" in t and "HOMO" in t and "6.900 eV" in t and "yes / yes / yes" in t and "3, chosen 0" in t, t
            pg.click("#tpr-t4"); t = pg.inner_text("#tpr-host #tpr-p"); assert "not evidence" in t and "deactivated rings" in t and "TiO2" in t, t
            pg.click("#tpr-t5"); assert json.loads(pg.inner_text("#tpr-host #tpr-p pre"))["format"] == "reatt"
            pg.focus("#tpr-t5"); pg.keyboard.press("ArrowRight"); assert pg.evaluate("document.querySelector('#tpr-host [role=tab][aria-selected=true]').textContent") == "f(−)", "arrow keys between the tabs"
        step("cards Info, Notes, JSON and f(0), f(+)", cardsswitch)
        def nulls():
            eht = json.loads(json.dumps(REATT)); eht["method"] = "EHT"
            for at in eht["atoms"]: at["q"] = None; at["f_zero"] = None
            for k in ("ip_ev", "ea_ev", "mu_ev", "eta_ev", "omega_ev"): eht["global"][k] = None
            eht["conformers"]["f_minus_spread"] = None
            assert pg.evaluate(MOUNT, eht)
            q = pg.evaluate("[...document.querySelectorAll('#tpr-host .tpr-tab tbody tr')].map(r => [r.children[2].textContent, r.children[4].textContent])")
            assert all(c == ["—", "—"] for c in q), q[:3]
            pg.click("#tpr-t3"); t = pg.inner_text("#tpr-host #tpr-p")
            assert "EHT" in t and "HOMO" in t and "Ionisation" not in t and "Electrophilicity" not in t and "spread" not in t, t
            assert pg.evaluate(MOUNT, REATT)
        step("values the engine does not give (null: q and f(0) of EHT, ip/ea/ω) are «—» or left out", nulls)
        def exports():
            pg.click('#tpr-host button[data-ex="XYZ"]'); pg.click('#tpr-host button[data-ex="CSV"]'); pg.click('#tpr-host button[data-ex="SMILES"]'); pg.click('#tpr-host button[data-ex="JSON"]')
            d = pg.evaluate("window.__dl"); names = [x[0] for x in d]; assert len(d) == 4 and names[0].endswith(".xyz") and names[1].endswith(".csv"), names
            assert len(d[0][1].strip().split("\n")) == N + 2 and d[0][1].split("\n")[0] == str(N), "XYZ has N + 2 lines"
            assert len(d[1][1].strip().split("\n")) == N + 1 and d[2][1].strip() == REATT["smiles"] and json.loads(d[3][1])["atoms"][0]["el"] == "C"
            t = pg.inner_text("#tpr-host .tpr-cite"); pg.click("#tpr-host .tpr-cite summary"); t = pg.inner_text("#tpr-host .tpr-cite")
            assert "doi:10.1021/ja983494x" in t and "mzlab-chem/0-test" in t and "10.1021/acs.jctc.8b01176" in t, t
        step("exports XYZ (N + 2 lines), CSV, SMILES, JSON and «How to cite»", exports)
        def twod():
            pg.wait_for_selector("#tpr-host svg.tpr-2d", timeout=60000)
            assert pg.evaluate("document.querySelectorAll('#tpr-host svg.tpr-2d circle[fill=\"var(--v3-halo)\"]').length") == 11, "a halo under each heavy atom"
            assert pg.evaluate("document.querySelectorAll('#tpr-host svg.tpr-2d circle[stroke-width=\"3.2\"]').length") == 5, "outline on the atoms above the threshold"
        step("2D drawing with a halo under each heavy atom", twod)
        def order():
            res = pg.evaluate("""async (cases) => {
                const OCL = await import(new URL('vendor/openchemlib.js', window.QTOOLS.ctx.base).href), out = [];
                for (const r of cases) { const m = OCL.Molecule.fromSmiles(r.smiles), map = window.TPREATT.mapHeavy(m, r); out.push(map); }
                return out; }""", [REATT, heavy_only("NC(=O)N1c2ccccc2C=Cc2ccccc12")])
            assert res[0] == list(range(11)), res[0]
            assert res[1] == list(range(len(res[1]))) and len(res[1]) == 18, res[1]
            # a relabelled graph: heavy_index permuted, the mapping by element and neighbours must give a valid correspondence
            rl = heavy_only("CC(=O)Nc1ccc(O)cc1"); perm = [0, 1, 2, 3, 4, 10, 9, 7, 8, 6, 5]      # swaps the two ortho/meta pairs: same graph, other order
            for a in rl["atoms"]: a["heavy_index"] = perm[a["heavy_index"]]
            mp = pg.evaluate("async (r) => { const OCL = await import(new URL('vendor/openchemlib.js', window.QTOOLS.ctx.base).href); const m = OCL.Molecule.fromSmiles(r.smiles); return window.TPREATT.mapHeavy(m, r); }", rl)
            assert mp is not None and sorted(mp) == list(range(11)), mp
        step("OpenChemLib order of the heavy atoms = heavy_index (paracetamol, carbamazepine); a relabelled graph is mapped", order)
        def states():
            for st, txt in [({"state": "waiting"}, "waiting for TPs"), ({"state": "computing"}, "computing…"), ({"state": "failed", "error_key": "error.chem.element"}, "not covered by the engine")]:
                pg.evaluate("s => { document.querySelector('#tpr-host')?.remove(); const h = document.createElement('div'); h.id = 'tpr-host'; document.querySelector('.tp').appendChild(h); window.TPREATT.state(h, s); }", st)
                assert txt in pg.inner_text("#tpr-host .tpr"), (st, pg.inner_text("#tpr-host .tpr"))
        step("states: waiting for TPs, computing…, failed with the message of its error_key", states)
        def axe():
            AXE = lib.axe_path()
            if not AXE: print("SKIP axe-core not available (pip install axe-playwright-python)"); return
            pg.evaluate(MOUNT, REATT); pg.wait_for_selector("#tpr-host svg.tpr-2d", timeout=60000); pg.add_script_tag(path=AXE)
            run = "axe.run('#tpr-host', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,3).map(n => n.target.join(' '))]))"
            v = pg.evaluate(run); assert not v, v
        step("axe: the panel has no serious or critical violation", axe)
        def shots():
            d = os.environ.get("REATT_SHOTS")
            if not d: return
            for th in ("light", "dark"):
                pg.evaluate("t => document.documentElement.setAttribute('data-theme', t)", th);
                pg.locator("#tpr-host .tpr").screenshot(path=str(Path(d) / f"reatt_{th}.png"))
        step("screenshots (REATT_SHOTS)", shots)
        def hosted():
            assert pg.evaluate("!!document.querySelector('#tp-struct + #tp-reatt')"), "no host in the Parent compound card"
            pg.fill("#tp-smi", "CC(=O)Nc1ccc(O)cc1"); pg.wait_for_function("document.querySelector('#tp-reatt .tpr')?.dataset.state === 'waiting'", timeout=30000)
            assert "waiting for TPs" in pg.inner_text("#tp-reatt .tpr")
        step("the card Parent compound hosts the panel (state waiting for TPs once the SMILES is there)", hosted)
        step("window.MZLAB_LR_MISS is empty", lambda: (_ for _ in ()).throw(AssertionError(pg.evaluate("window.MZLAB_LR_MISS"))) if pg.evaluate("(window.MZLAB_LR_MISS || []).length") else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s_ in steps: print(s_)
print("page errors:", errs)
sys.exit(1 if any(s_[1] != "ok" and not s_[1].startswith("SKIP") for s_ in steps) or errs else 0)
