"""Tabui, the network of the products (TP_Mine/lazy/rete-vista.js, TPRETE_VIEW). The component is mounted by hand in #tp-right of the built site (administrator mode) with the output of the Rust
engine saved in TP_Mine/tests/rete_fixture.json (5 products of the timolol formula, 6 arcs, two MS2 compared). Checks: mount (nodes, arcs, columns as in the JSON), selection from the keyboard and by
click with `onSelect`, `selectRow` from outside, the reason of an arc under the chart, the table of the arcs (TDATI) and «Copy data», the shapes of the nodes (not only colour), targets of at least
24 px, axe (light and dark), no console error. MZLAB_PIPE_SITE reuses a built site. Prints «MISURA clic_rete_riga»: clicks from a node to the selected row of the table. Run from the repo root: python3 tests_e2e/e2e_rete.py"""
import json, os, re, shutil, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
from lib import axe_path  # noqa: E402

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_rete.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:900]))

FX = json.loads((ROOT / "TP_Mine" / "tests" / "rete_fixture.json").read_text(encoding="utf-8"))
tmp = Path(tempfile.mkdtemp(prefix="tabui_rete_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True, capture_output=True)
shutil.copy2(ROOT / "TP_Mine" / "lazy" / "rete-vista.js", site / "static" / "mzfinder" / "lazy" / "rete-vista.js")      # fresh source over the site that was built
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
for _ in range(50):
    try: __import__("urllib.request").request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,3).map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,3).join(' | '))]))"
MOUNT = """(fx) => { const host = document.querySelector("#tp-right"); host.innerHTML = '<div id="rv-host"></div><table id="rv-rows"><tbody></tbody></table>'; window.__picked = []; window.__clicks = 0;
  const rows = document.querySelector("#rv-rows tbody"); fx.nodes.forEach(n => { const tr = document.createElement("tr"); tr.dataset.row = n.id; tr.innerHTML = "<td>" + n.id + "</td>"; rows.appendChild(tr); });
  document.addEventListener("click", () => window.__clicks++, true);
  window.__V = window.TPRETE_VIEW.mount(document.querySelector("#rv-host"), fx, { onSelect: id => { window.__picked.push(id); document.querySelectorAll("#rv-rows tr").forEach(r => r.classList.toggle("sel", +r.dataset.row === id)); } }); }"""

try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce"); pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.on("console", lambda m: errs.append("console: " + m.text) if m.type == "error" and re.search(r"rete|TPRETE", m.text + str(m.location)) else None)
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector('#nav button[data-v="tpmine"]', state="attached", timeout=120000)
        pg.evaluate("() => { const l = document.getElementById('loading'); if (l && /motore|engine/i.test(l.innerText)) l.remove(); }")     # the view needs no engine
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-right", timeout=60000)
        pg.wait_for_function("typeof window.TDATI === 'object'", timeout=120000)
        pg.evaluate("() => window.TPLAZY.load('rete-vista.js')")
        pg.wait_for_function("typeof window.TPRETE_VIEW === 'object'", timeout=20000)
        pg.evaluate(MOUNT, FX)
        ids = [n["id"] for n in FX["nodes"]]
        parent = FX["parent"]

        def drawn():
            got = pg.evaluate("() => [...document.querySelectorAll('#rv-host g.nd')].map(g => +g.dataset.node)")
            assert sorted(got) == sorted(ids), (got, ids)
            assert pg.locator("#rv-host path.arc").count() == len(FX["arcs"]) == FX["n_arcs"]
            for n in FX["nodes"]:                       # the column of a node is its step: x grows with the step, y with the place
                tr = pg.evaluate(f"() => document.querySelector('#rv-host g.nd[data-node=\"{n['id']}\"]').getAttribute('transform')")
                x, y = [float(v) for v in tr[len("translate("):-1].split()]
                assert x == 60 + n["step"] * 190 and y == 44 + n["pos"] * 66, (n, tr)
            assert pg.locator("#rv-host text.colh").count() == FX["columns"]
        step("nodes, arcs and columns are those of the JSON", drawn)

        def shapes():
            assert pg.locator("#rv-host g.nd rect.glyph").count() == 1, "the parent is the only square"
            kin = {n["id"]: n["kinship"] for n in FX["nodes"]}
            for nid, k in kin.items():
                g = pg.locator(f'#rv-host g.nd[data-node="{nid}"]')
                if nid == parent: continue
                dash = g.locator("circle.glyph[stroke-dasharray]").count(); half = g.locator("path.glyph").count(); full = g.locator("circle.glyph:not(.hollow)").count()
                assert (k >= 4 and full == 1) or (k == 3 and half == 1) or (k == 2 and dash == 1) or (k < 2 and full == 0 and half == 0 and dash == 0), (nid, k, dash, half, full)
            assert "Parent" in pg.get_attribute(f'#rv-host g.nd[data-node="{parent}"]', "aria-label")
            assert pg.locator("#rv-host .rv-leg li").count() >= 5
        step("the kinship is a shape (square, filled, half, dashed, empty) and a word in the label", shapes)

        def widths():
            by = {(a["from"], a["to"]): a for a in FX["arcs"]}
            ws = pg.evaluate("() => [...document.querySelectorAll('#rv-host path.arc')].map(p => [parseFloat(p.getAttribute('stroke-width')), p.hasAttribute('stroke-dasharray')])")
            for a, (w, dashed) in zip(FX["arcs"], ws):
                n = (a["ms2"]["n_equal"] + a["ms2"]["n_shifted"]) if a["ms2"] else 0
                assert dashed == (n == 0), (a, w, dashed)
                assert abs(w - (1.4 + 0.9 * min(n, 5))) < 0.06, (a, w)
        step("the width of an arc follows its shared fragments; no MS2 compared = dashed", widths)

        def targets():
            sizes = pg.evaluate("() => [...document.querySelectorAll('#rv-host g.nd .hit')].map(r => { const b = r.getBoundingClientRect(); return [b.width, b.height]; })")
            assert all(w >= 24 and h >= 24 for w, h in sizes), sizes
            ah = pg.evaluate("() => [...document.querySelectorAll('#rv-host .rv-arcs button')].map(r => r.getBoundingClientRect().height)")
            assert all(h >= 24 for h in ah), ah
        step("targets of at least 24 px", targets)

        def keyboard():
            first = ids[0]
            pg.focus(f'#rv-host g.nd[data-node="{first}"]'); pg.keyboard.press("Enter")
            assert pg.evaluate("window.__picked") == [first], pg.evaluate("window.__picked")
            assert pg.get_attribute(f'#rv-host g.nd[data-node="{first}"]', "aria-pressed") == "true"
            assert pg.locator("#rv-host circle.ring").count() == 1, "the chosen node has a thick outline"
            pg.keyboard.press("ArrowRight")
            nxt = pg.evaluate("document.activeElement && +document.activeElement.dataset.node")
            assert nxt in ids and nxt != first, nxt
            pg.keyboard.press("Space"); assert pg.evaluate("window.__picked")[-1] == nxt
            assert pg.locator(f"#rv-rows tr.sel").get_attribute("data-row") == str(nxt), "the table follows the node"
        step("a node is chosen from the keyboard (Enter, Space, arrows); onSelect gives its row id", keyboard)

        def reason():
            nid = FX["arcs"][0]["from"]
            pg.click(f'#rv-host g.nd[data-node="{nid}"]')
            why = pg.locator("#rv-host [data-why]").inner_text()
            for a in FX["arcs"]:
                if nid in (a["from"], a["to"]): assert a["reason"] in why, (a["reason"], why)
            pg.click('#rv-host [data-arcbtn="0"]')
            assert pg.locator("#rv-host [data-why]").inner_text().count("\n") == 0 and FX["arcs"][0]["reason"] in pg.locator("#rv-host [data-why]").inner_text()
            assert pg.locator("#rv-host path.arc.sel").count() == 1
        step("the reason of the chosen arc is written under the chart", reason)

        def outside():
            n0 = len(pg.evaluate("window.__picked"))
            pg.evaluate(f"window.__V.selectRow({ids[-1]})")
            assert pg.evaluate("window.__V.state().node") == ids[-1] and len(pg.evaluate("window.__picked")) == n0, "selectRow does not call onSelect"
            pg.evaluate("window.__V.selectRow(null)"); assert pg.evaluate("window.__V.state().node") is None
        step("selectRow chooses a node from outside without calling onSelect", outside)

        def data_table():
            pg.focus("#rv-host svg.rv-svg"); pg.keyboard.press("t")
            pg.wait_for_selector("dialog#tdati table", timeout=5000)
            txt = pg.evaluate("() => [...document.querySelectorAll('dialog table, .tdati table, [role=dialog] table')].map(t => t.innerText).join(' ')")
            assert FX["arcs"][0]["rule"] in txt and "Δ ion formula" in txt, txt[:300]
        step("the table of the arcs (TDATI) opens from the chart", data_table)

        def copy():
            pg.keyboard.press("Escape")
            csv = pg.evaluate("window.__V.csv()")
            lines = csv.strip().split("\r\n")
            assert lines[0].startswith("From m/z,To m/z,Rule") and len(lines) == 1 + len(FX["arcs"]), lines[:2]
            ctx.grant_permissions(["clipboard-read", "clipboard-write"], origin=f"http://127.0.0.1:{PORT}")
            pg.click("#rv-host [data-copy]"); pg.wait_for_function("document.querySelector('#rv-host [data-note]').textContent.length > 0", timeout=3000)
            assert "Copied" in pg.locator("#rv-host [data-note]").inner_text() or "not" in pg.locator("#rv-host [data-note]").inner_text()
        step("«Copy data» gives the arcs as CSV", copy)

        def clicks():
            pg.evaluate(f"window.__V.selectRow(null); window.__clicks = 0; window.__picked = []")
            pg.click(f'#rv-host g.nd[data-node="{ids[1]}"]')
            n = pg.evaluate("window.__clicks")
            assert pg.locator("#rv-rows tr.sel").get_attribute("data-row") == str(ids[1])
            print(f"MISURA clic_rete_riga = {n}")
            assert n == 1, n
        step("one click on a node chooses its row", clicks)

        def axe():
            if not AXE: print("SKIP axe: axe-core is not available"); return
            pg.add_script_tag(path=str(AXE))
            for theme in ("light", "dark"):
                pg.evaluate(f"document.documentElement.dataset.theme = '{theme}'")
                v = pg.evaluate(RUN % "document.querySelector('#rv-host')"); assert not v, (theme, v)
            pg.evaluate("delete document.documentElement.dataset.theme")
        step("axe: no serious violation, light and dark", axe)

        def no_miss():
            assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0
        step("MZLAB_LR_MISS stays empty", no_miss)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " :: " + s))
if errs: print("PAGE ERRORS", errs[:5])
sys.exit(1 if any(s != "ok" for _, s in steps) or errs else 0)
