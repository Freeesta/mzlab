"""«Choose samples» dialog of a .wiff with several samples (wiff-choose.js, docs/agenti/wiff-sample-choice.md PR 4; the range copy of a partial choice, wiff-range-copy.md): shown only when the file has several samples, every non-empty sample ticked for a small
set and an empty one (acquired nothing) not, a SIM-only sample listed with its cycles, Select all / Select none, Open selected (Enter too) opens only the ticked ones, X and Escape open nothing,
phone width, the setting «never» and «always»; the ticked samples are copied ahead while the dialog is open (start, untick, X, reuse by Open selected; mzlab/web/rust-prefetch.js). Synthetic pairs of tools/wiff_grande.py and the `tipi` pair of tools/wiff_tipi.py (MS, SIM-only and empty sample). Needs the site and the .wasm: python tools/build_site.py, and the .wasm built here when cargo and wasm-bindgen exist."""
# covers: mzlab/web/archivio, crates/mzlab-core/src/readers/ranges, mzlab/web/wiff-choose, mzlab/web/settings, mzlab/web/shell, mzlab/web/rust-nativo, mzlab/web/rust-worker, mzlab/web/rust-prefetch, mzlab/web/rust-index-ahead, crates/mzlab-core/src/readers/selection
import os, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, Run

def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_wiff_choose.py")][-1]      # the line of a bare assert is the only clue of a red CI run
        print("  ", (name, "FAIL line %d: " % ln + str(e)[:1500]))

wasm = ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js"
if not (ROOT / "site" / "index.html").exists() or not wasm.exists():
    print("  ", ("site", "FAIL site/ or its .wasm missing: python tools/build_site.py, then build mzlab-wasm")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="wiffchoose_e2e_"))
for name, n in (("multi", 3), ("solo", 1), ("pre", 4)):
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(D / name), "--name", name, "--samples", str(n), "--scans", "40", "--points", "50"], check=True, capture_output=True)
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_tipi.py"), str(D / "tipi")], check=True, capture_output=True)
def pair(name): return [str(D / name / f"{name}.wiff"), str(D / name / f"{name}.wiff.scan")]
run = Run(); PORT = run.port
DLG = "dialog#wiffchoose[open]"
def rows(pg): return pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=use]')).length")
def dlg(pg): pg.wait_for_selector(DLG, timeout=60000)
def ticked(pg): return pg.evaluate("Array.from(document.querySelectorAll('#wiffchoose tbody input:checked')).length")
def neutral(pg):
    t = pg.evaluate("document.querySelector('#fl-up [data-cvhead]')?.textContent || ''"); body = pg.evaluate("document.querySelector('#fl-up')?.innerText || ''")
    return t == "Choose samples" and "Opening" not in body and not __import__("re").search(r"\d:\d\d", body)
def settle(pg): pg.wait_for_function("!document.querySelector('dialog#wiffchoose')", timeout=30000)

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1300, "height": 900}, device_scale_factor=2, pick_dialog=True); c.set_default_timeout(120000); pg = c.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.bar.hint','1');localStorage.setItem('mzlab.panels.side_by_side.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
        def key(): return pg.evaluate("NOMI.KEY('pick')")
        def several():
            pg.set_input_files("#pick", pair("multi")); dlg(pg)
            assert ticked(pg) == 3, ticked(pg)
            assert pg.get_attribute("#wiffchoose", "aria-labelledby") == "wc-title" and pg.evaluate("document.querySelector('#wiffchoose').matches(':modal')")
            assert pg.get_attribute("#wiffchoose .wc-sel", "role") == "status" and pg.evaluate("document.querySelectorAll('#wiffchoose tbody input[aria-label]').length") == 3
            assert pg.inner_text("#wiffchoose .wc-sel") == "3/3" and pg.get_attribute("#wiffchoose .wc-sel", "aria-label") == "3 of 3 samples ticked", pg.inner_text("#wiffchoose .wc-sel")
            # the simple layout: no number box, no Cancel, no "No." column; X at the top right; Open selected centred at the bottom
            assert not pg.query_selector("#wiffchoose .wc-intro") and "Ticked samples start loading" not in pg.inner_text("#wiffchoose")      # no line about the loading
            assert not pg.query_selector("#wc-ids") and not pg.query_selector("#wiffchoose button:text('Cancel')") and not pg.query_selector("#wiffchoose button:text('Tick numbers')")
            assert pg.evaluate("Array.from(document.querySelectorAll('#wiffchoose thead th')).map(h => h.textContent).join('|')") == "Open|Sample|Type|Scans|Size"
            assert pg.evaluate("""() => { const d = document.querySelector('#wiffchoose').getBoundingClientRect(), x = document.querySelector('#wiffchoose .wc-x').getBoundingClientRect(), o = document.querySelector('#wiffchoose button.go').getBoundingClientRect();
              return x.right > d.right - 40 && x.top < d.top + 60 && Math.abs((o.left + o.right) / 2 - (d.left + d.right) / 2) < 12 }""")
            assert pg.evaluate("Array.from(document.querySelectorAll('#wiffchoose button, #wiffchoose input')).every(e => e.getBoundingClientRect().height >= 24 && e.getBoundingClientRect().width >= 24)")
            assert pg.evaluate("document.querySelector('#wiffchoose button.go').getBoundingClientRect().height >= 44")      # a large, easy target
            # the Files list while the dialog waits: a neutral «Choose samples», no clock and no «Opening»
            try: pg.wait_for_function("(() => { const t = document.querySelector('#fl-up [data-cvhead]')?.textContent || '', b = document.querySelector('#fl-up')?.innerText || ''; return t === 'Choose samples' && !b.includes('Opening') && !/\\d:\\d\\d/.test(b) })()", timeout=10000)
            except Exception: pass      # the assert below says what the list shows
            assert neutral(pg), pg.evaluate("document.querySelector('#fl-up')?.innerText")
            shot = os.environ.get("MZLAB_SHOTS")
            if shot: Path(shot).mkdir(parents=True, exist_ok=True); pg.screenshot(path=str(Path(shot) / "wiff-choose-dialog.png"))
        step("several samples: the dialog opens with every sample ticked and is accessible", several)
        def none_then_all():
            pg.click("#wiffchoose button:text('Select none')"); assert ticked(pg) == 0 and pg.is_disabled("#wiffchoose button.go") and pg.inner_text("#wiffchoose .wc-sel") == "0/3" and pg.get_attribute("#wiffchoose .wc-sel", "aria-label") == "0 of 3 samples ticked"
            pg.click("#wiffchoose button:text('Select all')"); assert ticked(pg) == 3 and not pg.is_disabled("#wiffchoose button.go")
        step("Select none disables Open, Select all ticks every sample again", none_then_all)
        def phone():
            pg.set_viewport_size({"width": 390, "height": 780})
            assert pg.evaluate("() => { const d = document.querySelector('#wiffchoose'); return d.scrollWidth <= d.clientWidth + 1 && d.getBoundingClientRect().right <= innerWidth && d.getBoundingClientRect().left >= 0 }")
            big = pg.evaluate("() => { const o = document.querySelector('#wiffchoose button.go').getBoundingClientRect(), d = document.querySelector('#wiffchoose').getBoundingClientRect(); return [o.height, o.left - d.left, d.right - o.right] }")      # Open selected stays large and inside the dialog
            assert big[0] >= 44 and big[1] >= 0 and big[2] >= 0, big
            assert pg.evaluate("Array.from(document.querySelectorAll('#wiffchoose button, #wiffchoose input')).every(e => e.getBoundingClientRect().height >= 24 && e.getBoundingClientRect().width >= 24)")
            shot = os.environ.get("MZLAB_SHOTS")
            if shot: Path(shot).mkdir(parents=True, exist_ok=True); pg.screenshot(path=str(Path(shot) / "wiff-choose-phone.png"))
            pg.set_viewport_size({"width": 1300, "height": 900})
        step("phone width: the dialog fits the screen and its controls stay tappable", phone)
        def cancel():
            pg.keyboard.press("Escape"); settle(pg); assert rows(pg) == 0, rows(pg)
        step("Escape cancels: nothing is opened", cancel)
        def close_x():
            pg.set_input_files("#pick", pair("multi")); dlg(pg)
            pg.click("#wiffchoose .wc-x"); settle(pg); assert rows(pg) == 0, rows(pg)
            # a deliberate cancel is no news: no «was not opened» notice, no row of the file, no store left of it
            pg.wait_for_function("!document.querySelector('#fl-up [data-cvhead]') && !(document.getElementById('flist')?.innerText || '').includes('multi.wiff')", timeout=30000)
            assert "was not opened" not in pg.evaluate("document.body.innerText")
            pg.wait_for_function("""async () => { const out = []; const walk = async (d) => { for await (const [n, h] of d.entries()) { if (h.kind === 'directory') await walk(h); else if (n.startsWith('nat~') && n.includes('~multi.wiff')) out.push(n); } }; await walk(await navigator.storage.getDirectory()); return out.length === 0; }""", timeout=30000)
        step("the X at the top right closes the dialog and opens nothing: no notice, no row, no store", close_x)
        def open_some():
            pg.set_input_files("#pick", pair("multi")); dlg(pg)
            pg.click("#wiffchoose tbody tr:nth-child(2) input"); assert ticked(pg) == 2
            pg.click("#wiffchoose button.go"); settle(pg)
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
            assert rows(pg) == 2, rows(pg)
        step("Open selected opens only the ticked samples", open_some)
        def range_copy():
            # the .wiff.scan in OPFS has the size of the original but holds only the chosen samples' bytes (the skipped sample's section is zeros): the engine's copy plan, putSparse
            got = pg.evaluate("""async () => { const out = []; const walk = async (d, path) => { for await (const [n, h] of d.entries()) { if (h.kind === 'directory') await walk(h, path + n + '/'); else if (n.startsWith('nat~') && n.endsWith('.wiff.scan')) { const f = await h.getFile(); const b = new Uint8Array(await f.arrayBuffer()); let z = 0, nz = 0; for (const v of b) v === 0 ? z++ : nz++; out.push({ n, size: b.length, nz }); } } }; await walk(await navigator.storage.getDirectory(), ''); return out; }""")
            src = (D / "multi" / "multi.wiff.scan").read_bytes()
            mine = [g for g in got if "multi" in g["n"]]
            assert mine, got
            last = mine[-1]
            assert last["size"] == len(src), (last["size"], len(src))
            assert 0 < last["nz"] < sum(1 for v in src if v != 0), (last["nz"], sum(1 for v in src if v != 0))
        step("a partial choice copies only the chosen ranges of the .wiff.scan into OPFS (same size, the skipped section is zeros)", range_copy)
        def solo():
            n = rows(pg); pg.set_input_files("#pick", pair("solo"))
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length > {n}", timeout=120000)
            assert not pg.query_selector("dialog#wiffchoose")
        step("one sample per file: no dialog", solo)
        def never():
            pg.evaluate(f"localStorage.setItem({key()!r}, 'never')"); n = rows(pg)
            pg.set_input_files("#pick", pair("multi"))
            try: pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n} + 3", timeout=30000)
            except Exception: raise AssertionError(("rows", n, rows(pg), pg.evaluate("Array.from(document.querySelectorAll('#flist [data-k=use]')).map(e => (e.closest('tr,li,div')||e).textContent.slice(0, 60))"), pg.evaluate("(document.querySelector('#notes,.notes,#msgs')||{}).textContent || ''")))
            assert not pg.query_selector("dialog#wiffchoose")
        step("setting «never»: every sample opens without asking", never)
        def always():
            pg.evaluate(f"localStorage.setItem({key()!r}, 'always')")
            pg.set_input_files("#pick", pair("solo")); dlg(pg); assert ticked(pg) == 1
            pg.click("#wiffchoose .wc-x"); settle(pg)
        step("setting «always»: a file with one sample is asked too", always)
        # last: it needs the pair opened by the first choice and must not change the row counts the settings steps rely on
        pg.evaluate("localStorage.removeItem(NOMI.KEY('pick'))")
        def second_choice():
            # sample 1 and 3 were opened above: choosing sample 2 of the same pair must make its own copy, never reuse the first one (its sample-2 ranges are zeros)
            import base64
            js = """async (src64) => { const src = Uint8Array.from(atob(src64), c => c.charCodeAt(0)); const out = []; const walk = async (d) => { for await (const [n, h] of d.entries()) { if (h.kind === 'directory') await walk(h); else if (n.startsWith('nat~') && n.endsWith('.wiff.scan')) { const b = new Uint8Array(await (await h.getFile()).arrayBuffer()); let nz = 0, bad = 0, digest = 0; for (let i = 0; i < b.length; i++) if (b[i] !== 0) { nz++; if (b[i] !== src[i]) bad++; digest = (digest * 31 + b[i] + i) >>> 0; } out.push({ n, size: b.length, nz, bad, digest }); } } }; await walk(await navigator.storage.getDirectory()); return out; }"""
            src = base64.b64encode((D / "multi" / "multi.wiff.scan").read_bytes()).decode()
            before = {g["n"]: g for g in pg.evaluate(js, src) if "multi" in g["n"]}
            assert before, before
            n = rows(pg); pg.set_input_files("#pick", pair("multi")); dlg(pg)
            pg.click("#wiffchoose button:text('Select none')"); pg.click("#wiffchoose tbody tr:nth-child(2) input")
            assert ticked(pg) == 1
            pg.click("#wiffchoose button.go"); settle(pg)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length > {n}", timeout=120000)
            after = {g["n"]: g for g in pg.evaluate(js, src) if "multi" in g["n"]}
            new = [g for k, g in after.items() if k not in before]
            assert new, (sorted(before), sorted(after))
            assert all(g["bad"] == 0 and g["nz"] > 0 for g in new), new
            assert all(g["digest"] != b["digest"] for g in new for b in before.values()), (new, before)
        step("another sample of the same file gets its own partial copy (sample 2 after samples 1 and 3)", second_choice)
        def kinds_and_empty():
            # MS sample, SIM-only sample (its cycles are its scans, type SIM), a sample that acquired nothing: only that one is unticked and says «empty»
            n = rows(pg); pg.set_input_files("#pick", pair("tipi")); dlg(pg)
            cells = pg.evaluate("Array.from(document.querySelectorAll('#wiffchoose tbody tr')).map(r => ({ on: r.querySelector('input').checked, t: Array.from(r.querySelectorAll('td')).slice(1).map(c => c.textContent) }))")
            assert [c["on"] for c in cells] == [True, True, False], cells
            assert cells[0]["t"][1:3] == ["Full scan", "5"] and cells[1]["t"][1:3] == ["SIM", "7"], cells
            assert cells[2]["t"][2] == "empty" and cells[2]["t"][1] == "", cells
            assert pg.inner_text("#wiffchoose .wc-sel") == "2/3" and pg.get_attribute("#wiffchoose .wc-sel", "aria-label") == "2 of 3 samples ticked" and pg.evaluate("document.querySelector('#wiffchoose tbody tr.wc-empty') !== null")
            shot = os.environ.get("MZLAB_SHOTS")
            if shot:
                Path(shot).mkdir(parents=True, exist_ok=True); pg.screenshot(path=str(Path(shot) / "wiff-choose-kinds.png"))
                pg.set_viewport_size({"width": 390, "height": 780}); pg.screenshot(path=str(Path(shot) / "wiff-choose-kinds-phone.png")); pg.set_viewport_size({"width": 1300, "height": 900})
            pg.keyboard.press("Enter"); settle(pg)     # focus starts on Open selected, and Enter opens
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n} + 2", timeout=120000)
            assert rows(pg) == n + 2, (n, rows(pg))
        step("a SIM sample lists its cycles and type SIM, an empty one is unticked and says empty; Enter opens the ticked ones", kinds_and_empty)
        def enter_from_a_row():
            pg.set_input_files("#pick", pair("multi")); dlg(pg)
            pg.focus("#wiffchoose tbody tr:nth-child(1) input"); pg.keyboard.press("Enter"); settle(pg)
            assert not pg.query_selector("dialog#wiffchoose")
        step("Enter pressed on a checkbox opens too", enter_from_a_row)
        # the copy ahead (rust-prefetch.js): the ticked samples of the `pre` pair (4 samples) are copied into OPFS while the dialog is open
        STORES = """async () => { const out = []; const walk = async (d) => { for await (const [n, h] of d.entries()) { if (h.kind === 'directory') await walk(h); else if (n.startsWith('nat~') && n.includes('~pre.wiff')) { const f = await h.getFile(); out.push([n, f.lastModified, f.size]); } } }; await walk(await navigator.storage.getDirectory()); return out.sort(); }"""
        def scans(): return [x for x in pg.evaluate(STORES) if x[0].endswith(".wiff.scan")]
        # the loading is silent: no text in the dialog says anything about it; a row being copied only carries aria-busy, so «done» is a store per sample and no busy row
        BUSY = "document.querySelectorAll('#wiffchoose tbody tr[aria-busy=true]').length"
        def silent(): return pg.evaluate("Array.from(document.querySelectorAll('#wiffchoose tbody tr')).every(r => !/wait|load|%/i.test(r.textContent)) && !document.querySelector('#wiffchoose .wc-st, #wiffchoose .wc-intro')")
        def wait_loaded(n): pg.wait_for_function(f"{BUSY} === 0 && ({STORES})().then(a => a.filter(x => x[0].endsWith('.wiff.scan')).length === {n})", timeout=60000)
        # the index ahead (rust-index-ahead.js): the index of a ticked sample is built while the dialog is open, under a name of the pair and the sample (not of the final selection), and Open only moves it
        IDXS = """async () => { const out = []; const walk = async (d) => { for await (const [n, h] of d.entries()) { if (h.kind === 'directory') await walk(h); else if (n.endsWith('.mzidx') && (n.startsWith('nat~') || n.endsWith('.mzML.mzidx'))) { const f = await h.getFile(); const hash = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', await f.arrayBuffer()))).map(v => v.toString(16).padStart(2, '0')).join(''); out.push([n, f.size, hash]); } } }; await walk(await navigator.storage.getDirectory()); return out.sort(); }"""
        def aheads(): return [x for x in pg.evaluate(IDXS) if x[0].startswith("nat~") and x[0].endswith("~pre.mzidx")]
        def wait_aheads(n): pg.wait_for_function(f"({IDXS})().then(a => a.filter(x => x[0].startsWith('nat~') && x[0].endsWith('~pre.mzidx')).length === {n})", timeout=60000)
        def finals(): return [x for x in pg.evaluate(IDXS) if not x[0].startswith("nat~")]
        def stats(): return pg.evaluate("window.MZLAB_RUST.rpc({ op: 'natstats' }).then(r => JSON.parse(r.info))")
        def prefetch_start():
            pg.set_input_files("#pick", pair("pre")); dlg(pg)
            assert ticked(pg) == 4, ticked(pg)
            assert pg.is_enabled("#wiffchoose button.go")      # the dialog is never blocked by the loading
            wait_loaded(4)                                     # before pressing Open: one store of the .wiff.scan per ticked sample, no row busy
            assert len(scans()) == 4 and pg.evaluate(BUSY) == 0 and silent()
            assert len(aheads()) == 4, aheads()                # loaded is true: the index of each ticked sample is there too, before Open
        step("ticked samples start loading when the dialog opens: a store and an index per sample before Open, and the dialog says nothing about it", prefetch_start)
        def prefetch_untick():
            pg.click("#wiffchoose tbody tr:nth-child(2) input")
            wait_loaded(3); wait_aheads(3)
            assert pg.evaluate("document.querySelectorAll('#wiffchoose tbody tr[aria-busy]').length") == 0 and silent()
            pg.click("#wiffchoose tbody tr:nth-child(2) input")      # ticked again: loaded again
            wait_loaded(4); wait_aheads(4)
        step("unticking a sample discards its copy and its index; ticking it again loads it again", prefetch_untick)
        def prefetch_x():
            pg.click("#wiffchoose .wc-x"); settle(pg)
            pg.wait_for_function(f"({STORES})().then(a => a.length === 0)", timeout=60000)
            wait_aheads(0); assert not [x for x in pg.evaluate(IDXS) if x[0].startswith("nat~")], pg.evaluate(IDXS)      # no index of the pair is left either
            assert rows(pg) == n_rows_before_pre, rows(pg)
        n_rows_before_pre = rows(pg)
        step("the X cancels everything and frees the copies: no store of the pair is left", prefetch_x)
        def prefetch_reuse():
            n = rows(pg); pg.set_input_files("#pick", pair("pre")); dlg(pg)
            wait_loaded(4); before = scans(); ahead = aheads(); s0 = stats()
            assert len(ahead) == 4 and s0["built"] >= 4, (ahead, s0)
            t0 = pg.evaluate("performance.now()")
            pg.click("#wiffchoose button.go"); settle(pg)
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n} + 4", timeout=120000)
            took = pg.evaluate("performance.now()") - t0
            assert scans() == before, (before, scans())      # the opening reused the copies: same stores, same bytes, never written again
            # ... and the indexes: moved to the names of the samples (same bytes, same sizes), none built at the opening, none left under the neutral names
            s1 = stats(); done = [x for x in finals() if x[0].startswith("pre_")]      # the indexes of the samples of the pair, under the names of the list
            assert s1["adopted"] - s0["adopted"] == 4 and s1["atOpen"] == s0["atOpen"] and s1["built"] == s0["built"], (s0, s1)
            assert len(done) == 4 and sorted(x[2] for x in done) == sorted(x[2] for x in ahead) and sorted(x[1] for x in done) == sorted(x[1] for x in ahead), ([x[1:] for x in ahead], done)
            assert not aheads(), aheads()
            print("   Open selected -> 4 rows listed in", round(took), "ms (indexes adopted, not built)")
        step("Open selected reuses what is loaded: the stores and the indexes are not written or built again (moved, same bytes) and the rows open", prefetch_reuse)
        NZ = """async () => { const out = []; const walk = async (d) => { for await (const [n, h] of d.entries()) { if (h.kind === 'directory') await walk(h); else if (n.startsWith('nat~') && n.includes('~pre.wiff.scan')) { const b = new Uint8Array(await (await h.getFile()).arrayBuffer()); let nz = 0; for (const v of b) if (v !== 0) nz++; out.push([n, nz]); } } }; await walk(await navigator.storage.getDirectory()); return out; }"""
        def prefetch_bytes():
            # every sample ticked: the copies made ahead hold the sections once and the head and tail of the .wiff.scan once per sample, nothing else
            got = pg.evaluate(NZ); src = (D / "pre" / "pre.wiff.scan").read_bytes(); whole = sum(1 for v in src if v != 0)
            total = sum(g[1] for g in got); assert len(got) == 4, got
            own = sorted(g[1] for g in got); extra = total - whole      # what is copied twice: the head and tail shared by the samples
            assert 0 <= extra <= (len(got) - 1) * (own[0] if own else 0), (total, whole, own)
            assert total < 2 * whole, (total, whole)
        step("every sample ticked: the copies hold the file once plus the head and tail once per extra sample, never twice the file", prefetch_bytes)
        def prefetch_cached():
            # the same pair dropped again, its samples already open with their index: nothing is copied ahead (no row busy, no store written; a copy that started marks its row busy at once)
            n = rows(pg); before = scans(); pg.set_input_files("#pick", pair("pre")); dlg(pg)
            assert pg.evaluate(BUSY) == 0 and scans() == before, (before, scans())
            pg.click("#wiffchoose .wc-x"); settle(pg); assert rows(pg) == n, rows(pg)
        step("a pair already loaded is not copied ahead when it is dropped again", prefetch_cached)
        b.close()
finally:
    srv = run.srv; srv.terminate(); import shutil; shutil.rmtree(D, ignore_errors=True)
