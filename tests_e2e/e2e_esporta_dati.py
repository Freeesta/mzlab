"""«Export all my data» (settings, group «Dati e privacy»): fills some settings, plants two secrets, presses the button FROM THE KEYBOARD, reads the downloaded ZIP
(Python zipfile) and checks the manifest lists the entries, the secrets are neither inside nor un-listed in «excluded», and axe finds nothing in the panel.
Build first: python tools/prepara_sito.py"""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import io, json, zipfile
from lib import *

AXE = axe_path()
SECRET_A, SECRET_B = "ghp_SECRETTOKEN0123456789", "sk-SECRETAPIKEY9876543210"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).replace("\n", " ")[:600]))
AXE_RUN = """() => axe.run(document.querySelector('#uipset'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
r = Run(port=8871, wd="/tmp/wd_esporta")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate("""([a, b]) => { localStorage.setItem('mzlab.libro.token', a); localStorage.setItem('mzlab.llm.apikey', b); localStorage.setItem('mzlab.tunit', 's'); }""", [SECRET_A, SECRET_B])
        pg.click("#np-set"); pg.wait_for_selector("#uipset", state="visible")
        pg.check("input[name='uip-th'][value='dark']", force=True); pg.wait_for_timeout(300)
        pg.select_option("#uip-pal", index=1); pg.wait_for_timeout(300)
        z = {}
        def press():
            pg.focus("#uip-g-priv > summary"); pg.keyboard.press("Enter")
            pg.wait_for_selector("#uip-export", state="visible")
            if AXE:
                pg.add_script_tag(path=AXE)
                v = pg.evaluate(AXE_RUN); assert not v, v
            box = pg.evaluate("(() => { const b = document.querySelector('#uip-export').getBoundingClientRect(); return [b.width, b.height]; })()")
            assert box[0] >= 24 and box[1] >= 24, box
            pg.focus("#uip-export")
            with pg.expect_download(timeout=30000) as dl:
                pg.keyboard.press("Enter")
            d = dl.value; assert d.suggested_filename.startswith("mzlab-dati-") and d.suggested_filename.endswith(".zip"), d.suggested_filename
            z["bytes"] = open(d.path(), "rb").read()
            pg.wait_for_function("document.querySelector('#uip-export-st').textContent.length > 0 && !document.querySelector('#uip-export').disabled")
            z["status"] = pg.inner_text("#uip-export-st")
        step("button from the keyboard downloads mzlab-dati-<date>.zip, axe clean, target >= 24 px", press)
        def content():
            zf = zipfile.ZipFile(io.BytesIO(z["bytes"])); assert zf.testzip() is None
            names = zf.namelist(); assert names[0] == "manifesto.json" and "localStorage.json" in names, names
            man = json.loads(zf.read("manifesto.json")); assert man["format"] == "mzlab-dati/1", man
            listed = {f["name"] for f in man["files"]}; assert listed == set(names) - {"manifesto.json"}, (listed, names)
            ls = json.loads(zf.read("localStorage.json"))
            assert "mzlab.tunit" in ls and "mzlab.prefs" in ls, list(ls)
            assert json.loads(ls["mzlab.prefs"])["theme"] == "dark", ls["mzlab.prefs"]
            excl = {e["item"]: e["reason"] for e in man["excluded"]}
            assert excl.get("mzlab.libro.token") == "secret" and excl.get("mzlab.llm.apikey") == "secret", excl
            for n in names:
                data = zf.read(n); assert SECRET_A.encode() not in data and SECRET_B.encode() not in data, n
            assert str(len(names)) in z["status"], z["status"]
        step("manifest lists the entries, settings are inside, no secret anywhere, secrets listed as excluded", content)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
