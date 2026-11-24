"""«Name files by» of the download of the mzML on the built site (Rust engine): a synthetic SCIEX .wiff of tools/wiff_riferimento.py (`nomi.wiff`: five samples, two named alike, one
without a name) is loaded; the choice is there with its visible label inside its accessible name, «File name» is the default and the saved file keeps its list name; with «Sample name» the names and the
notes are those the engine's route `mzml_names` gives, which are those of `mzlab-cli file convert --name sample --dry-run` for the same file (names and `note:` lines), and the file saved is
named so. Needs the site and the .wasm: python tools/build_site.py."""
# covers: mzlab/web/conversione.js, mzlab/web/nativo.js, mzlab/web/lang/en/conv.js
import json, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port, axe_path
from cli_cmd import cli_command

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ or its .wasm not built: python tools/build_site.py")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="nameby_e2e_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
import socket
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(300000):      # wait for the server to listen (connection attempts, not a fixed pause)
    try: socket.create_connection(("127.0.0.1", PORT), timeout=0.2).close(); break
    except OSError: pass
INIT = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.bar.hint','1');localStorage.setItem('mzlab.panels.side_by_side.hint','1');localStorage.setItem('mzlab.pick','never');}catch(_){}"

def cli_names():
    """What the command line writes for the same file with `--name sample`: the sample number and file name of each item, in the order of the samples, and its `note:` lines."""
    out = D / "cli_out"
    r = subprocess.run([*cli_command(), "file", "convert", str(D / "nomi.wiff"), "--out", str(out), "--name", "sample", "--dry-run", "--json"], capture_output=True, text=True, timeout=600)
    assert r.returncode == 0, (r.returncode, r.stderr[-400:])
    items = json.loads(r.stdout)["items"]
    return [[i["sample_id"], Path(i["output"]).name] for i in items], [l for l in r.stderr.splitlines() if l.startswith("note:")]

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1800}, accept_downloads=True); c.set_default_timeout(120000); pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e)))); pg.add_init_script(INIT)
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
        pg.set_input_files("#pick", [str(D / "nomi.wiff"), str(D / "nomi.wiff.scan")])
        pg.wait_for_function("document.querySelectorAll('#flist [data-dl]').length >= 5", timeout=120000)
        for _ in range(12):      # the table shows when asked; a click while the file is still opening does nothing
            pg.click("#samptab")
            try: pg.wait_for_selector("#wtable:not([hidden])", timeout=5000); break
            except Exception: pass
        else: pg.wait_for_selector("#wtable:not([hidden])", timeout=5000)
        pg.locator("#advconv summary").click()
        def the_choice():
            sel = pg.get_by_role("combobox", name="Name files by")      # the visible label text is inside the accessible name
            assert sel.count() == 1, sel.count()
            assert pg.eval_on_selector("#cvnameby", "e => e.value") == "file"
            assert pg.eval_on_selector_all("#cvnameby option", "o => o.map(x => x.textContent)") == ["File name", "Sample name"]
            assert pg.locator("#cvnotes").is_hidden(), "no notes with the default"
            box = sel.bounding_box(); assert box["width"] >= 24 and box["height"] >= 24, box
        step("the choice is there: «Name files by» with File name and Sample name, File name selected, no notes", the_choice)
        listed = pg.evaluate("CONV.listed()")
        def default_unchanged():
            with pg.expect_download(timeout=120000) as d: pg.evaluate("n => CONV.download(n)", listed[0])
            assert d.value.suggested_filename == listed[0], (d.value.suggested_filename, listed[0])
        step("File name: the file is saved under its name in the list, as before", default_unchanged)
        def by_sample():
            pg.select_option("#cvnameby", "sample")
            pg.wait_for_function("document.getElementById('cvnotes') && !document.getElementById('cvnotes').hidden", timeout=60000)
            got = pg.evaluate("CONV.listed().map(n => [CONV.meta.get(n).id, CONV.nameOf(n)])")      # sample number and name, in the order of the list
            want, notes = cli_names()
            assert got == want, (got, want)      # file by file, in the order of the samples: a swap of two names fails
            shown = pg.eval_on_selector_all("#cvnotes li", "l => l.map(x => x.textContent)")
            assert shown == notes, (shown, notes)
            assert len(shown) >= 1 and any("already taken" in t for t in shown), shown      # the two samples named alike: the later one yields, and it is said
        step("Sample name: the names and the notes are those of `mzlab-cli file convert --name sample` for the same file", by_sample)
        def accessible_with_notes():
            # the notes are list items of a plain list inside a polite live region (a `role=status` on the list itself would break its semantics)
            assert pg.eval_on_selector("#cvnotes", "e => e.tagName === 'UL' && !e.hasAttribute('role') && e.parentElement.getAttribute('role') === 'status'")
            AXE = axe_path(); assert AXE, "axe-core not available (pip install axe-playwright-python)"
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("""() => axe.run({include: [['#cvnotes'], ['#cvnameby']]}, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21aa','wcag22aa','best-practice']}})
              .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact) || v.id === 'list' || v.id === 'listitem').map(v => [v.id, v.impact, v.nodes.slice(0, 3).map(n => n.target.join(' '))]))""")
            assert not v, v
        step("Sample name: the notes are visible and the bar passes axe (list and listitem included)", accessible_with_notes)
        def saved_by_sample():
            with pg.expect_download(timeout=120000) as d: pg.evaluate("n => CONV.download(n)", listed[0])
            want = pg.evaluate("n => CONV.nameOf(n)", listed[0])
            assert d.value.suggested_filename == want, (d.value.suggested_filename, want)
        step("Sample name: the file is saved under that name", saved_by_sample)
        def back_to_file():
            pg.select_option("#cvnameby", "file")
            pg.wait_for_function("document.getElementById('cvnotes').hidden", timeout=30000)
            assert pg.evaluate("CONV.listed().every(n => CONV.nameOf(n) === n)")
        step("File name again: the names are the list's and the notes go", back_to_file)
        assert not errs, errs
        b.close()
finally:
    srv.terminate()
