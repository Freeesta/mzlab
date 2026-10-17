"""The loading of a SCIEX .wiff + .wiff.scan of realistic size in the browser (synthetic, tools/wiff_grande.py; no laboratory data), on the built site with the Rust engine (?motore=rust&lr=1):
the bar goes forward only and ends; each sample has its row with the phase, the scans done and the time, and the page never stays still for more than 5 s; at the end each sample keeps
«Loaded ✓ (n scans, t s)» and the totals by kind; a truncated .wiff.scan says how many scans are missing; «Cancel» stops the files still running and they do not enter the list; a damaged pair leaves an error with «Details» and «Copy diagnosis», whose text carries the
trace of the loading and no file name. Needs the site and the .wasm (python tools/build_site.py + wasm, see e2e_wiff.py). Run from the repo root: python3 tests_e2e/e2e_wiff_carico.py"""
import json, os, re, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port, hold

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py + wasm")); sys.exit(0)
D = Path(tempfile.mkdtemp(prefix="wiff_carico_"))
for name, samples, scans, points in (("lotto", 4, 700, 1500), ("ferma", 4, 900, 1500), ("molti", 48, 40, 300), ("corto", 1, 200, 300)):
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(D), "--name", name, "--samples", str(samples), "--scans", str(scans), "--points", str(points)], check=True, capture_output=True)
(D / "rotto.wiff").write_bytes(b"\x00not a compound file\x01" * 4000)      # a .wiff that is not an OLE container: the pair cannot be opened
(D / "rotto.wiff.scan").write_bytes((D / "lotto.wiff.scan").read_bytes())
cut = D / "corto.wiff.scan"; cut.write_bytes(cut.read_bytes()[: cut.stat().st_size // 2])      # a copy of the pair that stopped halfway: the .wiff lists scans the .wiff.scan does not have
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
for _ in range(50):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)      # the server is starting
# what the student sees of the loading: the rows of the table, the list of the ended ones, the bar of the card; the bar of PROGRESSO; the rows of CONVROWS
LOOK = """JSON.stringify({see: [...document.querySelectorAll('#flist [data-cvrow], #up .loadbar')].map(e => e.innerText).join('|'), pc: PROGRESSO.state().pc, busy: UPBUSY,
  rows: CONVROWS.rows.map(r => [r.name, r.state]), files: ST.files.map(f => f.name)})"""

def follow(pg, until, limit=300):
    """polls the page every 250 ms until `until(state)`; gives the states and the longest time the visible text stayed the same"""
    seen, last, since, still, t0 = [], None, time.time(), 0.0, time.time()
    while time.time() - t0 < limit:
        s = json.loads(pg.evaluate(LOOK)); seen.append(s)
        if s["see"] != last: still = max(still, time.time() - since); last, since = s["see"], time.time()
        if until(s): break
        hold(pg, 250)                                   # the polling measures time itself: how long the text stays the same
    return seen, still

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(locale="en-US", viewport={"width": 1500, "height": 1800}, permissions=["clipboard-read", "clipboard-write"]); c.set_default_timeout(120000)
        pg = c.new_page(); pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); pg.wait_for_selector("#drop"); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)

        pg.set_input_files("#pick", [str(D / "lotto.wiff"), str(D / "lotto.wiff.scan")])
        states, still = follow(pg, lambda s: s["busy"] == 0 and len(s["files"]) == 4)
        def bar():
            pcs = [s["pc"] for s in states if s["busy"]]
            assert pcs and all(a <= b for a, b in zip(pcs, pcs[1:])), pcs
            assert states[-1]["busy"] == 0 and sorted(states[-1]["files"]) == [f"lotto_s{i}.mzML" for i in range(1, 5)], states[-1]
        step("the bar only goes forward and the loading ends with the 4 samples in the list", bar)
        def rows():
            phases = {st for s in states for _, st in s["rows"]}
            assert {"nscan"} <= phases and all(st == "done" for _, st in states[-1]["rows"]), (phases, states[-1]["rows"])
            assert any(" of 700" in s["see"] for s in states), "the row counts the scans of its sample"
        step("each sample has its row with its phases and the scans done", rows)
        def alive(): assert still <= 5, f"the loading was still for {still:.1f} s"
        step("the visible text never stays the same for more than 5 s", alive)
        def loaded():
            items = pg.locator("#flist .cvsum li").all_inner_texts()
            assert len(items) == 4 and all("Loaded ✓ (700 scans," in t and t.rstrip().endswith("s)") for t in items), items
            assert pg.locator("#flist tr[aria-busy=true]").count() == 0
            tot = pg.inner_text("#flist .cvsum [data-cvtot]")
            assert re.fullmatch(r"4 files: 4 full scan · \d+:\d\d", tot.strip()), tot
            pg.locator("#flist .cvsum [data-cvact=close]").click()
            assert pg.locator("#flist .cvsum").count() == 0 and pg.evaluate("CONVROWS.rows.length") == 0
        step("each sample keeps «Loaded ✓ (n scans, t s)» until «Close»", loaded)

        def many():                                     # one .wiff with 48 samples: each builder opens only its own sample (wiff::lettore::apri_solo), none waits on the others
            pg.set_input_files("#pick", [str(D / "molti.wiff"), str(D / "molti.wiff.scan")])
            seen, still = follow(pg, lambda s: s["busy"] == 0 and sum(n.startswith("molti_s") for n in s["files"]) == 48)
            assert sum(n.startswith("molti_s") for n in seen[-1]["files"]) == 48, seen[-1]
            assert still <= 5, f"the loading was still for {still:.1f} s"
            items = pg.locator("#flist .cvsum li").all_inner_texts()
            assert len(items) == 48 and all("Loaded ✓ (40 scans," in t for t in items), items[:3]
            pg.locator("#flist .cvsum [data-cvact=close]").click()
        step("a container of 48 samples loads them all, never still for more than 5 s", many)
        def truncated():                                # a truncated .wiff.scan: the sample loads with the scans it has, and its row and the totals say how many are missing
            pg.set_input_files("#pick", [str(D / "corto.wiff"), str(D / "corto.wiff.scan")])
            pg.wait_for_function("UPBUSY === 0 && CONVROWS.rows.length > 0", timeout=120000)
            li = pg.locator("#flist .cvsum li[data-cvmiss]")
            assert li.count() == 1, pg.inner_text("#flist .cvsum")
            n, t = int(li.get_attribute("data-cvmiss")), li.inner_text()
            assert 0 < n < 200 and f"{n} scans missing: the .wiff.scan ends before them" in t and "Loaded ✓" in t, t
            assert "1 with scans missing" in pg.inner_text("#flist .cvsum [data-cvtot]")
            pg.locator("#flist .cvsum [data-cvact=close]").click()
        step("a truncated .wiff.scan loads what it has and says how many scans are missing", truncated)

        def cancel():
            pg.set_input_files("#pick", [str(D / "ferma.wiff"), str(D / "ferma.wiff.scan")])
            pg.wait_for_function("CONVROWS.rows.some(r => r.state === 'nscan')", timeout=120000)
            pg.locator("#flist [data-cvact=cancel]").click(); t0 = time.time()
            pg.wait_for_function("UPBUSY === 0", timeout=30000)
            assert time.time() - t0 < 15, f"Cancel took {time.time() - t0:.1f} s"
            end = dict(pg.evaluate("CONVROWS.rows.map(r => [r.name, r.state])"))
            assert any(v == "cancel" for v in end.values()) and all(v in ("cancel", "done") for v in end.values()), end
            files = pg.evaluate("ST.files.map(f => f.name)")
            assert not any(n in files for n, v in end.items() if v == "cancel"), (end, files)
            assert "Cancelled" in pg.inner_text("#flist .cvsum")
        step("«Cancel» stops the samples running: they end as Cancelled and do not enter the list", cancel)

        def broken():
            pg.locator("#flist .cvsum [data-cvact=close]").click()
            pg.set_input_files("#pick", [str(D / "rotto.wiff"), str(D / "rotto.wiff.scan")])
            pg.wait_for_function("UPBUSY === 0", timeout=120000)
            box = "#flist .cvsum"
            assert pg.locator(f"{box} [data-cvend=fail] .diag [data-a=diagdet]").count() == 1, pg.inner_text("#flist")[-800:]
            pg.locator(f"{box} [data-a=diagdet]").first.click()
            pg.locator(f"{box} [data-a=diagcopy]").first.click()
            pg.wait_for_function("async () => (await navigator.clipboard.readText()).includes('phase:')")      # the button may be drawn again (the list of the files changes): the clipboard is the proof
            txt = pg.evaluate("navigator.clipboard.readText()")
            assert "phase:" in txt and "rotto" not in txt and str(D) not in txt, txt
            assert "loading:" in txt and ".wiff.scan" in txt and "phases:" in txt, txt
        step("a damaged pair leaves the error with «Details» and «Copy diagnosis» (trace of the loading, no file name)", broken)
        def overlay():                                  # «Load data» while the samples are still loading: the overlay says the phase and the time, and has its «Cancel»
            pg.locator("#flist .cvsum [data-cvact=close]").click()
            pg.set_input_files("#pick", [str(D / "ferma.wiff"), str(D / "ferma.wiff.scan")])
            pg.wait_for_function("UPBUSY > 0"); pg.locator("#opbtn").click()
            pg.wait_for_function("document.querySelector('#ldact button') && /\\d:\\d\\d/.test(document.querySelector('#ldmsg').textContent)")
            t1 = pg.inner_text("#ldmsg")
            pg.wait_for_function("t => document.querySelector('#ldmsg').textContent !== t", arg=t1, timeout=3000)       # the elapsed time moves every second
            assert "Indexing 0%" not in t1, t1
            pg.locator("#ldact button").click()
            pg.wait_for_function("UPBUSY === 0", timeout=30000)
            assert pg.locator("#ldact").is_hidden()
        step("the overlay of «Load data» shows the phase and the time, and «Cancel» stops the loading", overlay)
        assert not errs, errs
        b.close()
finally:
    srv.terminate(); shutil.rmtree(D, ignore_errors=True)
