"""Download of the regulatory-list package on the built site: `mzlab-cli normative pack` writes the committed lists into site/static/normative/ and the test serves
them. First use downloads the pieces into the Cache Storage `mzlab-libs` and hands the raw `.mzsus` over; a reload with the network cut is served from the cache;
a corrupted piece is refused (SHA-256) and installs nothing; «check for updates» only compares catalogues. Nothing leaves the machine.
Needs the site: python tools/build_site.py (and cargo, to build mzlab-cli once); delete site/ afterwards."""
import socket, hashlib, json, os, shutil, subprocess, sys, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port
from lib import copia as lib_copia

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:500]))

if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
cli = ROOT / "target" / "release" / "mzlab-cli"
if not cli.exists():
    if not shutil.which("cargo"):
        print("  ", ("cli", "FAIL no target/release/mzlab-cli and no cargo to build it")); sys.exit(0)
    subprocess.run(["cargo", "build", "--locked", "--release", "-p", "mzlab-cli"], cwd=ROOT, check=True)
OUT = ROOT / "site" / "static" / "normative"
shutil.rmtree(OUT, ignore_errors=True)
subprocess.run([str(cli), "normative", "pack", str(ROOT / "normative"), str(OUT)], check=True, capture_output=True)
lib_copia(ROOT / "mzlab" / "web" / "normative-scarica.js", ROOT / "site" / "static" / "normative-scarica.js")
RAW = (OUT / "normative.mzsus").read_bytes()
CAT = json.loads((OUT / "catalogo.json").read_text(encoding="utf-8"))
(OUT / "normative.mzsus").unlink()          # the raw file is not served: the pieces are the package

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):                          # wait on the state: the port accepts connections
    try:
        socket.create_connection(("127.0.0.1", PORT), timeout=0.2).close()
        break
    except OSError:
        pass
URL = f"http://127.0.0.1:{PORT}/"
BASE = "/static/normative/"
hits, corrupt = {}, [False]
def handler(route):
    k = route.request.url.split("/static/normative/")[1]; hits[k] = hits.get(k, 0) + 1
    if corrupt[0] and k.endswith(".bin"):
        r = route.fetch(); b = bytearray(r.body()); b[5] ^= 0xFF; return route.fulfill(response=r, body=bytes(b))
    return route.continue_()

# the module is imported by a page of the site (its own source, served as text: no engine needed); `window.N` holds the helpers
JS = """async (base) => { const m = await import('/static/normative-scarica.js'); window.N = {
  ensure: async () => { try { const r = await m.ensure({ base }); return { ok: true, size: r.raw.length, head: new TextDecoder().decode(r.raw.subarray(0, 4)),
      sha: [...new Uint8Array(await crypto.subtle.digest('SHA-256', r.raw))].map(x => x.toString(16).padStart(2, '0')).join(''), cached: r.cached, lists: r.catalog.lists.length }; }
    catch (e) { return { ok: false, key: e.key, params: e.params }; } },
  check: async () => (await m.check({ base })).state,
  cached: async () => (await (await caches.open('mzlab-libs')).keys()).map(r => new URL(r.url).pathname).sort(),
  wipe: async () => caches.delete('mzlab-libs') }; }"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); ctx = b.new_context(); pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.route("**/static/normative/**", handler)
        def boot():
            pg.goto(URL + "static/normative-scarica.js"); pg.evaluate(JS, BASE)
        boot()
        step("nothing is requested before the click", lambda: (_ for _ in ()).throw(AssertionError(hits)) if hits else None)
        def state_none():
            assert pg.evaluate("N.check()") == "none"
            assert list(hits) == ["catalogo.json"], hits
        step("check for updates compares catalogues only and sees nothing installed", state_none)
        def first():
            r = pg.evaluate("N.ensure()"); assert r["ok"], r
            assert r["head"] == "MZSU" and r["size"] == len(RAW) and r["sha"] == hashlib.sha256(RAW).hexdigest() and r["cached"] is False and r["lists"] == len(CAT["lists"]), r
            assert all(hits.get(p["file"]) == 1 for p in CAT["pieces"]), hits
            assert pg.evaluate("N.cached()") == sorted([BASE + "catalogo.json"] + [BASE + p["file"] for p in CAT["pieces"]])
        step("first use downloads the pieces, checks every SHA-256 and gives the raw .mzsus", first)
        def current():
            assert pg.evaluate("N.check()") == "current"
        step("check for updates says current", current)
        def offline():
            n = dict(hits); ctx.set_offline(True)
            try:
                boot_ok = pg.evaluate("N.ensure()"); assert boot_ok["ok"] and boot_ok["cached"] is True and boot_ok["sha"] == hashlib.sha256(RAW).hexdigest(), boot_ok
                assert pg.evaluate("N.check()") == "offline"
            finally: ctx.set_offline(False)
            assert all(hits.get(p["file"]) == n.get(p["file"]) for p in CAT["pieces"]), "a piece was requested again"
        step("with the network cut the cached package is used", offline)
        def reload_offline():
            boot(); ctx.set_offline(True)          # a fresh page (no state in memory) with the network cut after it loaded
            try:
                r = pg.evaluate("N.ensure()"); assert r["ok"] and r["cached"] is True, r
            finally: ctx.set_offline(False)
        step("a reload offline still opens the package", reload_offline)
        def corrupted():
            pg.evaluate("N.wipe()"); hits.clear(); corrupt[0] = True
            r = pg.evaluate("N.ensure()"); corrupt[0] = False
            assert r["ok"] is False and r["key"] == "normative.dl.err.hash", r
            assert hits[CAT["pieces"][0]["file"]] == 2, hits           # fetched once more, still wrong
            assert pg.evaluate("N.cached()") == [], "a corrupt piece was kept"
        step("a corrupted piece is refused after a second try and nothing is installed", corrupted)
        def recovers():
            r = pg.evaluate("N.ensure()"); assert r["ok"] and r["cached"] is False, r
        step("the next try installs the package", recovers)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(OUT, ignore_errors=True)
