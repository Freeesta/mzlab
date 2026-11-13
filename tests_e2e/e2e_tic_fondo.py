"""The TIC without the chemical background (api/tic_background, docs/agenti/tic-fondo.md) on the built site: the module `ticfondo`
(crates/mzlab-ticfondo-wasm) is fetched on the first request, never at start-up; the answer finds the six background lines of the synthetic run
(tools/tic_fondo_sintetico.py) and equals `mzlab-cli background` byte for byte, alone and with a blank sample (mode=blank).
Needs the built site (tools/prepara_sito.py) and cargo. Run from the repo root."""
import json, os, subprocess, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

D = Path(tempfile.mkdtemp(prefix="ticfondo_"))
for extra in ([], ["--blank"]):
    subprocess.run([sys.executable, str(ROOT / "tools" / "tic_fondo_sintetico.py"), str(D), "--scans", "600", *extra], check=True, capture_output=True)
RUN, BLANK = D / "tic_fondo-s1.mzML", D / "tic_fondo_bianco-s1.mzML"
subprocess.run(["cargo", "build", "-q", "--locked", "--release", "-p", "mzlab-cli"], cwd=ROOT, check=True)
CLI = ROOT / "target" / "release" / "mzlab-cli"
cli = lambda *a: subprocess.run([str(CLI), "background", str(RUN), *a], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()

r = Run(port=8987, wd="/tmp/wd_tic_fondo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        asked = []
        pg.on("request", lambda q: asked.append(q.url))
        pg.wait_for_selector("#drop", timeout=120000)
        mod = lambda: [u for u in asked if "/wasm-feat/ticfondo/" in u and u.endswith("_bg.wasm")]
        pg.set_input_files("#pick", [str(RUN), str(BLANK)])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
        pg.click("#opbtn")
        pg.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000)
        assert not mod(), mod()
        print("OK  the module is not fetched at start-up")

        get = """async u => { const x = await fetch(u); return [x.status, await x.text()]; }"""
        k, b = pg.evaluate("[0, 1].map(i => E.files.find(f => String(f.file).startsWith(i ? 'tic_fondo_bianco' : 'tic_fondo-')).k)")
        st, txt = pg.evaluate(get, f"api/tic_background?k={k}&level=1")
        assert st == 200, (st, txt[:300])
        ions = sorted(round(i["mz"]) for i in json.loads(txt)["ions"])
        assert ions == [141, 149, 158, 198, 214, 236], ions
        assert txt == cli(), "the site and mzlab-cli differ"
        print("OK  six background lines, the same JSON as mzlab-cli")

        st, txt = pg.evaluate(get, f"api/tic_background?k={k}&level=1&mode=blank&blank={b}&kind=bpc")
        assert st == 200, (st, txt[:300])
        assert txt == cli("--blank", str(BLANK), "--set", "mode=blank", "--set", "kind=bpc"), "blank: the site and mzlab-cli differ"
        assert len(mod()) == 1, mod()
        print("OK  with a blank sample, the same JSON as mzlab-cli; the module fetched once")

        st, txt = pg.evaluate(get, f"api/tic_background?k={k}&level=1&mode=nope")
        assert st == 400 and json.loads(txt)["error_key"] == "err.request.badValue", (st, txt[:300])
        print("OK  a bad value is a keyed error")
        assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
finally:
    r.close()
