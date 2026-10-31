"""Flow: the checkbox list of the imported regulatory lists (flow/lazy/flow-imported.js) in the form of screen.suspects and annot.regulatory. Runs the real flow-registry.js, flow-ui.js, flow-inputs.js and
flow-imported.js in a host page with a stand-in for the Library module (two imported lists and their bytes): ticking adds `imported-ID.mzsus` as an input of type library with its SHA-256 and the b64 content in
S.samplesJson and wires it to `imported_lists` (the package port stays empty); a second step shares the file; unticking removes the wire, and the input once no step uses it; the box is keyboard-reachable.
No site build, no cargo."""
import sys, os, json, hashlib, tempfile, shutil, threading; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

ROOT = Path(__file__).resolve().parents[1]
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:1500]))

tmp = Path(tempfile.mkdtemp(prefix="fi_"))
for f in ("flow-registry.js", "flow-ui.js", "flow-inputs.js", "flow-imported.js"): shutil.copy(ROOT / "flow/lazy" / f, tmp / f)
RAW = bytes(range(256)) + b"MZSU"
(tmp / "libreria-normative.js").write_text("""const RAW = new Uint8Array(%s);
export async function importedLists() { return [{ id: "SVHC", tags: { citation: "imported by you" } }, { id: "CA_PROP65", tags: {} }]; }
export async function importedBytes(id) { return id === "SVHC" ? RAW : null; }
""" % json.dumps(list(RAW)))
HOST = """<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="icon" href="data:,"><title>host</title>
<style>:root{--ink:#1a1a1a}body{background:#fff;color:var(--ink);margin:8px}input{min-width:24px;min-height:24px}</style></head><body><main id="m"></main>
<script src="flow-registry.js"></script><script src="flow-ui.js"></script><script src="flow-inputs.js"></script><script src="flow-imported.js"></script>
<script>
const step = { id: "screen.suspects", in: { table: { type: "table" }, suspects: { type: "library", many: true }, imported_lists: { type: "library", many: true, optional: true } } };
window.S = { plan: { inputs: [], steps: [{ id: "s", step: "screen.suspects@1", in: {} }, { id: "r", step: "screen.suspects@1", in: {} }] }, samplesJson: {} };
window.step = step; window.done = 0;
document.getElementById("m").append(FLOW.importedLists(S, S.plan.steps[0], step, () => { window.done++; }));
</script></body></html>"""
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
        pg.goto(f"http://127.0.0.1:{port}/"); pg.wait_for_selector("input[data-imported]")
        def listed():
            assert pg.locator("input[data-imported]").count() == 2 and "Lists you imported" in pg.inner_text("fieldset") and "imported by you" in pg.inner_text("fieldset")
        step("one checkbox per imported list", listed)
        def tick():
            pg.check("input[data-imported=SVHC]"); pg.wait_for_function("window.done === 1")
            S = pg.evaluate("({inputs: S.plan.inputs, in: S.plan.steps[0].in, text: S.samplesJson['imported-SVHC.mzsus']})")
            want = hashlib.sha256(RAW).hexdigest()
            assert S["inputs"] == [{"name": "imported-SVHC.mzsus", "sha256": want, "type": "library"}], S["inputs"]
            assert S["in"] == {"imported_lists": ["file:imported-SVHC.mzsus"]}, S["in"]
            import base64; assert base64.b64decode(S["text"][4:]) == RAW and S["text"].startswith("b64:")
        step("ticking adds the file as an input with its SHA-256 and wires imported_lists only", tick)
        def unreadable():
            pg.click("input[data-imported=CA_PROP65]"); pg.wait_for_function("!document.querySelector('input[data-imported=CA_PROP65]').checked")
            assert "could not be read" in pg.inner_text("fieldset") and len(pg.evaluate("S.plan.inputs")) == 1
        step("a file that cannot be read leaves the plan as it was", unreadable)
        def shared():
            pg.evaluate("(async () => { await FLOW.importedSet(S, S.plan.steps[1], step, 'SVHC', new Uint8Array([77, 90, 83, 85])); })()")
            pg.uncheck("input[data-imported=SVHC]"); pg.wait_for_function("window.done === 2")
            assert pg.evaluate("S.plan.inputs.length") == 1 and pg.evaluate("S.plan.steps[0].in.imported_lists") is None, "the input stays while another step uses it"
            pg.evaluate("(async () => { await FLOW.importedSet(S, S.plan.steps[1], step, 'SVHC', null); })()")
            assert pg.evaluate("S.plan.inputs.length") == 0 and pg.evaluate("'imported-SVHC.mzsus' in S.samplesJson") is False
        step("unticking unwires; the input goes with the last step that used it", shared)
        def keyboard():
            pg.focus("input[data-imported=SVHC]"); pg.keyboard.press("Space")
            pg.wait_for_function("window.done === 3"); assert pg.locator("input[data-imported=SVHC]").is_checked()
        step("the checkbox works from the keyboard", keyboard)
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs)) if errs else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
bad = [s for s in steps if s[1] != "ok"]
for n, r in steps: print(("ok   " if r == "ok" else "FAIL ") + n + ("" if r == "ok" else " :: " + r))
sys.exit(1 if bad else 0)
