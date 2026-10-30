"""The «Regulatory lists» section of the Library window (mzlab/web/libreria-normative.js) on an invented package served next to it: nothing installed offers the
download; after it, one row per list with jurisdiction, status, version and coverage; proposals and drafts start unticked; ticking writes the `lists` and `include`
parameters of the steps; «Check for updates» says the lists are current; the disclaimer is the report's; keyboard and axe-core clean. No site build, no cargo."""
import sys, os, json, hashlib, zlib, tempfile, shutil, threading; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

ROOT = Path(__file__).resolve().parents[1]
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_normative_panel.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:1500]))

tmp = Path(tempfile.mkdtemp(prefix="np_"))
for f in ("libreria-normative.js", "normative-scarica.js"): shutil.copy(ROOT / "mzlab/web" / f, tmp / f)
shutil.copy(ROOT / "flow/lazy/flow-form.js", tmp / "flow-form.js")
(tmp / "lang").mkdir(); shutil.copy(ROOT / "mzlab/web/lang/en/normative.js", tmp / "lang/normative.js")
# an invented package: the panel reads only the catalogue; the raw bytes stand in for the .mzsus
raw = b"MZSU invented regulatory package for the panel test" * 8
z = zlib.compress(raw, 9)
sha = lambda b: hashlib.sha256(b).hexdigest()
def lst(i, status, entries, screenable, groups, inorganic, juris):
    return {"id": i, "tags": {"status": status, "jurisdiction": juris, "instrument": f"Invented {i}", "citation": f"Invented act {i}", "version_date": "2000-01-01"},
            "coverage": {"entries": entries, "screenable": screenable, "groups": groups, "uvcb": 0, "inorganic": inorganic, "unresolved": 0}, "csv_sha256": "a" * 64, "retrieved": "2000-01-02"}
LISTS = [lst("T_LIST_A", "in_force", 70, 61, 6, 3, "EU"), lst("T_LIST_B", "adopted_not_applicable", 5, 5, 0, 0, "EU"), lst("T_LIST_P", "proposal", 4, 4, 0, 0, "US"),
         lst("T_LIST_D", "draft", 2, 2, 0, 0, "US")]
CAT = {"format": "normative/1", "piece": 4194304, "size": len(raw), "size_compressed": len(z), "sha256": sha(raw), "rows": 1, "lists": LISTS, "attributions": [],
       "pieces": [{"file": "000.bin", "size": len(z), "sha256": sha(z)}]}
(tmp / "normative").mkdir(); (tmp / "normative/000.bin").write_bytes(z); (tmp / "normative/catalogo.json").write_text(json.dumps(CAT))
DISC = (ROOT / "crates/mzlab-core/src/flow/corsa_reg.rs").read_text(encoding="utf-8").split('pub const DISCLAIMER: &str = "')[1].split('";')[0]
HOST = """<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="icon" href="data:,"><title>host</title>
<style>:root{--ink:#1a1a1a;--line:#999}body{background:#fff;color:var(--ink);margin:8px}button,input{min-width:24px;min-height:24px}</style></head>
<body><main><fieldset id="lib-reg" aria-label="Regulatory lists"></fieldset></main>
<script>window.I18N = { d: {}, add(l, o) { Object.assign(this.d, o); }, t(k, p) { let s = this.d[k] ?? k; for (const [a, b] of Object.entries(p || {})) s = s.split("{" + a + "}").join(String(b)); return s; } };</script>
<script src="lang/normative.js"></script>
<script type="module">import * as m from "./libreria-normative.js"; window.NP = m; m.mount(document.getElementById("lib-reg")).then(() => { window.NP_READY = 1; });</script></body></html>"""
(tmp / "index.html").write_text(HOST)
port = free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).with_name("servi.py")), str(port), str(tmp)], cwd=str(ROOT), stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)
try:
    import urllib.request
    for _ in range(100):
        try: urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1).read(); break
        except Exception: threading.Event().wait(0.1)
    errs = []
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_page()
        pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("console", lambda m: errs.append(m.text) if m.type == "error" else None)
        pg.goto(f"http://127.0.0.1:{port}/"); pg.wait_for_function("window.NP_READY === 1")
        def empty():
            assert pg.is_visible("#lib-reg-get") and pg.locator("input[data-reg]").count() == 0
            assert "not downloaded yet" in pg.inner_text("#lib-reg")
        step("nothing installed: the download button and no rows", empty)
        def download():
            pg.click("#lib-reg-get"); pg.wait_for_function("document.querySelectorAll('#lib-reg input[data-reg]').length === 4")
            assert not pg.is_visible("#lib-reg-get")
        step("«Download the lists» installs the package and shows one row per list", download)
        def rows():
            t = pg.inner_text("#lib-reg table")
            assert "Invented T_LIST_A" in t and "EU" in t and "in force" in t and "2000-01-01" in t, t
            assert "70 entries: 61 screened, 6 groups, 3 inorganic" in t, t
            assert "adopted, not yet applicable" in t and "proposal" in t and "draft" in t, t
            assert pg.get_attribute("#lib-reg td[title='Invented act T_LIST_A']", "title") == "Invented act T_LIST_A"
        step("each row has label, jurisdiction, status, version and coverage", rows)
        def defaults():
            on = pg.eval_on_selector_all("#lib-reg input[data-reg]", "e => e.filter(x => x.checked).map(x => x.dataset.reg)")
            assert on == ["T_LIST_A", "T_LIST_B"], on
        step("lists in force and adopted start ticked, proposals and drafts not", defaults)
        def tick():
            pg.focus("input[data-reg=T_LIST_P]"); pg.keyboard.press("Space")
            sel = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.normative.sel'))")
            assert sel == {"lists": "T_LIST_A,T_LIST_B,T_LIST_P", "include": "proposal"}, sel
            assert pg.evaluate("NP.selection().lists") == "T_LIST_A,T_LIST_B,T_LIST_P"
            pg.uncheck("input[data-reg=T_LIST_A]"); pg.uncheck("input[data-reg=T_LIST_B]"); pg.uncheck("input[data-reg=T_LIST_P]")
            assert pg.evaluate("localStorage.getItem('mzlab.normative.sel')") is None and "Tick at least one list." in pg.inner_text("#lib-reg")
            pg.check("input[data-reg=T_LIST_A]")
        step("ticking (keyboard too) writes the lists and include parameters; none ticked keeps nothing", tick)
        def kept():
            pg.reload(); pg.wait_for_function("window.NP_READY === 1")
            on = pg.eval_on_selector_all("#lib-reg input[data-reg]", "e => e.filter(x => x.checked).map(x => x.dataset.reg)")
            assert on == ["T_LIST_A"], on
        step("the selection survives a reload", kept)
        def check():
            pg.click("#lib-reg-check"); pg.wait_for_function("document.getElementById('lib-reg').innerText.includes('up to date')")
        step("«Check for updates» compares the catalogues", check)
        def disclaimer():
            assert pg.inner_text("#lib-reg-disclaimer") == DISC
        step("the footer is the report's disclaimer", disclaimer)
        def flow_button():
            # flow-form.js with the few FLOW helpers it calls: the button copies the stored selection into the step's own parameters
            pg.evaluate("""() => new Promise(ok => { const got = {};
              globalThis.FLOW = { el: (t, a = {}, ...k) => { const n = document.createElement(t); for (const [x, v] of Object.entries(a)) if (x === 'text') n.textContent = v; else if (x.startsWith('on')) n[x] = v; else if (v != null) n.setAttribute(x, v); n.append(...k.filter(Boolean)); return n; },
                btn: (i, name, fn, text) => FLOW.el('button', { type: 'button', 'aria-label': name, onclick: fn, id: 'np-use' }, text || ''), effective: () => ({}), presetList: () => [], declared: () => [], defaults: () => ({}),
                setParam: (inst, step, k, v) => { got[k] = v; } };
              const s = document.createElement('script'); s.src = 'flow-form.js'; s.onload = () => { document.body.append(FLOW.form({ id: 's', step: 'screen.suspects@1' }, {}, () => { window.NP_GOT = got; })); ok(); }; document.head.append(s); })""")
            pg.click("#np-use")
            assert pg.evaluate("window.NP_GOT") == {"lists": "T_LIST_A", "include": None}, pg.evaluate("window.NP_GOT")
            assert pg.evaluate("FLOW.form({ id: 'x', step: 'features.align@1' }, {}, () => {}).querySelector('#np-use')") is None
        step("the Flow step's «Use Library lists» copies the selection into lists and include", flow_button)
        AXE = axe_path()
        def axe():
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run('#lib-reg', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length]))")
            assert not v, v
        if AXE: step("axe-core: no violation", axe)
        else: print("SKIP axe-core not available")
        b.close()
    assert not errs, errs
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
