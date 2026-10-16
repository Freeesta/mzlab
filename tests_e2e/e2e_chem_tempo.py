"""Time of the `.wasm` `chem` (crates/mzlab-chem-wasm) in Chromium: `reactivity(smiles, "")` (ETKDG geometry, GFN2-xTB of the neutral molecule, cation and anion in
ALPB water, f(−) on the lowest conformers) for carbamazepine (30 atoms) ≤ 5 s and valsartan (61 atoms) ≤ 15 s, the method GFN2-xTB with the three SCC converged,
`version()` = mzlab-chem/1 and a bad SMILES rejected with its key. Uses site/static/wasm-feat/chem when built there, else builds the module (cargo, wasm-bindgen,
tools/wasm_opt.sh); SKIP without cargo or wasm-bindgen. Run from the repo root: python3 tests_e2e/e2e_chem_tempo.py"""
import fcntl, json, os, shutil, socket, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)

MOLS = [("carbamazepine", "NC(=O)N1c2ccccc2C=Cc2ccccc21", 30, 5.0), ("valsartan", "CCCCC(=O)N(Cc1ccc(cc1)-c1ccccc1-c1nnn[nH]1)C(C(C)C)C(O)=O", 61, 15.0)]
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

tmp = Path(tempfile.mkdtemp(prefix="chem_tempo_"))
out = tmp / "chem"
built = ROOT / "site" / "static" / "wasm-feat" / "chem"
if (built / "mzlab_chem_wasm_bg.wasm").exists():
    shutil.copytree(built, out)
elif not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-chem-wasm"); sys.exit(0)
else:
    with open(os.path.join(tempfile.gettempdir(), "mzlab-build-feat.lock"), "w") as lk:   # shares cargo's target/ with lib.build_feat
        fcntl.flock(lk, fcntl.LOCK_EX)
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-chem-wasm"], cwd=ROOT, check=True, capture_output=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(out), "target/wasm32-unknown-unknown/release/mzlab_chem_wasm.wasm"], cwd=ROOT, check=True)
    subprocess.run(["tools/wasm_opt.sh", str(out / "mzlab_chem_wasm_bg.wasm"), "740000", "chem"], cwd=ROOT, env=dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
(tmp / "index.html").write_text("<!doctype html><meta charset=utf-8><title>chem</title>")

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
end = time.time() + 30
while True:
    try:
        socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError:
        if time.time() > end: raise
RUN = """async ([smiles]) => {
  const m = await import('/chem/mzlab_chem_wasm.js'); await m.default();
  const t = performance.now(), r = JSON.parse(m.reactivity(smiles, ''));
  return { s: (performance.now() - t) / 1000, version: m.version(), atoms: r.atoms.length, method: r.method, conv: r.scc.converged };
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/index.html")
        for name, smi, n, limit in MOLS:
            def timed(name=name, smi=smi, n=n, limit=limit):
                r = pg.evaluate(RUN, [smi])
                print(f"  {name}: {r['s']:.2f} s in Chromium ({r['atoms']} atoms)")
                assert r["version"] == "mzlab-chem/1" and r["atoms"] == n and r["method"] == "GFN2-xTB" and r["conv"] == [True, True, True], r
                assert r["s"] <= limit, f"{r['s']:.2f} s > {limit} s"
            step(f"{name} ({n} atoms) in ≤ {limit:.0f} s", timed)
        def bad():
            k = pg.evaluate("""async () => { const m = await import('/chem/mzlab_chem_wasm.js'); await m.default();
              try { m.reactivity('C1CC', ''); return ''; } catch (e) { return JSON.parse(e).error_key; } }""")
            assert k == "error.chem.smiles", k
        step("a bad SMILES is a JSON error with its key", bad)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
