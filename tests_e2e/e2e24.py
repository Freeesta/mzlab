# e2e24: scheda Disegno without data files. The student's drawing is never erased (2.0), properties with the charge excluded (2.1),
# default ion (2.2), SMILES of a selection, logP, the four exports (name, transparent background, JPEG state) (2.3).
# Runs against the local server; with SITE=<built site dir> (tools/build_site.py) it runs on the browser version instead.
import os, sys, subprocess, time, re, io, zipfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
KQ = "document.getElementById('kframe').contentWindow.ketcher"
SELALL = "(()=>{const k=%s;k.editor.selection({atoms:[...k.editor.struct().atoms.keys()]})})()" % KQ
SITE = os.environ.get("SITE")
errs = []
srv = None
if SITE:
    srv = subprocess.Popen([sys.executable, "-m", "http.server", "8861", "--bind", "127.0.0.1", "-d", SITE], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); time.sleep(1)
    r = None
else:
    r = Run(port=8824, wd="/tmp/wd24")
STATS = "(async()=>{const k=%s;const s=k.editor.struct();const ids=[...s.atoms.keys()];let fr=new Set();const up=new Map(ids.map(i=>[i,i]));const root=i=>{while(up.get(i)!==i)i=up.get(i);return i};s.bonds.forEach(b=>{const a=root(b.begin),c=root(b.end);if(a!==c)up.set(a,c)});ids.forEach(i=>fr.add(root(i)));return {atoms:ids.length,frags:fr.size,smi:await k.getSmiles()}})()" % KQ
PROPS = "[...document.querySelectorAll('#prop-body tr')].slice(1).map(r=>[...r.children].map(c=>c.textContent))"
def setmol(pg, smi):
    pg.evaluate(KQ + ".setMolecule('%s')" % smi); pg.wait_for_timeout(1500)
try:
    with sync_playwright() as p:
        if SITE:
            b = p.chromium.launch(); pg = b.new_page(viewport={"width": 1500, "height": 2200}, accept_downloads=True)
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('qqq.tutorial','1')}catch(e){}")
            pg.goto("http://127.0.0.1:8861/"); pg.wait_for_timeout(2500)
        else:
            pg = r.page(p)
        pg.click("#nav button[data-v=draw]"); pg.wait_for_function("window.TPDraw && TPDraw.ready()", timeout=90000)
        # ---------------------------------------------------------------- 2.0 the drawing is never erased
        setmol(pg, "CC(=O)Nc1ccc(O)cc1.Nc1ccc(O)cc1")
        base = pg.evaluate(STATS); print("base", base)
        def quick_adds():
            assert base["frags"] == 2, base
            pg.fill("#ex-smi", "CCO"); pg.click("#ex-load"); pg.wait_for_timeout(1500)
            s1 = pg.evaluate(STATS); print("after add", s1)
            assert s1["atoms"] == base["atoms"] + 3 and s1["frags"] == 3, (base, s1)
            assert all(x in s1["smi"] for x in ("CCO", "CC(=O)NC1=CC=C(O)C=C1")) or s1["smi"].count(".") == 2, s1
            # undo removes only the addition
            pg.evaluate("(()=>{const d=document.getElementById('kframe').contentDocument;const b=[...d.querySelectorAll('[data-testid=undo]')].find(e=>e.getBoundingClientRect().width>0);b.click()})()"); pg.wait_for_timeout(800)
            s2 = pg.evaluate(STATS); assert s2["atoms"] == base["atoms"] and s2["frags"] == 2, ("undo", s2)
            pg.fill("#ex-smi", "not a smiles((("); pg.click("#ex-load"); pg.wait_for_timeout(1200)
            assert pg.evaluate(STATS)["atoms"] == base["atoms"], "invalid SMILES changed the canvas"
            assert pg.is_visible("#smi-warn")
        step("2.0 Disegna veloce adds next to the structures (Ctrl+Z removes only it); invalid SMILES changes nothing", quick_adds)
        def no_control_erases():
            n0 = pg.evaluate(STATS)["atoms"]
            def same(why): n = pg.evaluate(STATS)["atoms"]; assert n == n0, (why, n0, n)
            pg.uncheck("#lb-f"); pg.check("#lb-f"); pg.uncheck("#lb-m"); pg.check("#lb-m"); same("labels")
            for v in ("1", "2", "0"): pg.select_option("#lb-dec", v); pg.wait_for_timeout(200)
            same("decimals")
            pg.evaluate("(async()=>{const k=%s;const ids=[...k.editor.struct().atoms.keys()];k.editor.selection({atoms:ids.slice(0,4)})})()" % KQ); pg.wait_for_timeout(1500)
            same("selection"); pg.evaluate(KQ + ".editor.selection(null)"); pg.wait_for_timeout(600)
            for fmt in ("png", "svg", "jpg", "ket"):
                with pg.expect_download() as d: pg.click("#ex-" + fmt)
            same("exports")
            pg.click("#ex-link"); pg.wait_for_timeout(600); pg.click("#bigx"); pg.wait_for_timeout(300); same("examples link")
            pg.click("#nav button[data-v=explore]") if pg.locator("#nav button[data-v=explore]").count() else pg.click("#nav button[data-v=data]")
            pg.wait_for_timeout(500); pg.click("#nav button[data-v=draw]"); pg.wait_for_timeout(1500); same("tab switch")
            pg.set_viewport_size({"width": 900, "height": 1400}); pg.wait_for_timeout(600); pg.set_viewport_size({"width": 1500, "height": 2200}); pg.wait_for_timeout(600); same("resize")
            pg.evaluate("NB.ket && nbSave()"); pg.wait_for_timeout(1500)
            pg.reload(); pg.wait_for_timeout(2500)
            pg.click("#nav button[data-v=draw]"); pg.wait_for_function("window.TPDraw && TPDraw.ready()", timeout=90000); pg.wait_for_timeout(2500)
            same("page reload (restored from the notebook)")
        step("2.0 no other control erases the drawing (labels, ion, selection, 4 exports, examples, tab switch, resize, reload)", no_control_erases)
        # ---------------------------------------------------------------- 2.1 charge excluded
        def logp_of(smi):
            setmol(pg, smi); pg.evaluate(SELALL); pg.wait_for_timeout(1500)      # the table is filled only for a selected molecule
            rows = pg.evaluate(PROPS); return rows
        def charge():
            acid = logp_of("CC(=O)O"); ac = logp_of("CC(=O)[O-]")
            assert acid and ac and acid[0][1] == ac[0][1] != "–", (acid, ac)
            assert pg.is_visible("#prop-neut") and pg.is_checked("#prop-neut"), "checkbox missing or not on"
            assert "forma neutra" in pg.inner_text("#prop-body"), pg.inner_text("#prop-body")
            assert ac[0][0].replace(" ", "") in ("C2H4O2",) or "C" in ac[0][0], ac
            pg.uncheck("#prop-neut"); pg.wait_for_timeout(1500); r2 = pg.evaluate(PROPS); assert r2[0][1] == "–", r2
            pg.check("#prop-neut"); pg.wait_for_timeout(1500)
            q = logp_of("C[N+](C)(C)C"); assert q and q[0][1] == "–", q
            assert "carica permanente: non neutralizzabile" in pg.inner_text("#prop-body")
            na = logp_of("CC(=O)[O-].[Na+]"); assert len(na) == 1 and na[0][1] == acid[0][1], ("counter-ion kept?", na)
            nh = logp_of("C[NH3+]"); mn = logp_of("CN"); assert nh[0][1] == mn[0][1] != "–", (nh, mn)
            neu = logp_of("CC(=O)Nc1ccc(O)cc1"); assert not pg.is_visible("#prop-neut"), "checkbox shown without charge"
            assert "forma neutra" not in pg.inner_text("#prop-body")
        step("2.1 charged structures: neutral form (acetate = acetic acid, ammonium, counter-ion dropped, quaternary = message)", charge)
        def helptext():
            assert pg.locator("#prop-card .hq").count() == 0
            t = r.teoria(pg); assert "0.5" in t and "logD" in t, t[:300]      # the explanation lives in the guide chapter of the Teoria
            assert "L'errore tipico" not in pg.inner_text("#prop-body")
        step("2.1 the explanation is in the ? of the box, not inside it", helptext)
        # ---------------------------------------------------------------- 2.2 default ion
        def ion():
            assert pg.locator("#lb-ion").count() == 0, "the Ione menu is gone"
            pg.evaluate("(()=>{NB.labIon='[M+K]+';document.dispatchEvent(new Event('nbloaded'))})()"); pg.wait_for_timeout(300)     # an old notebook with labIon opens without errors, the value is ignored
        step("2.2 no Ione menu; an old notebook with labIon opens fine", ion)
        def smiles_clear():
            pg.fill("#ex-smi", "CC(=O)Nc1ccc(O)cc1"); pg.click("#ex-load"); pg.wait_for_timeout(2500)
            assert pg.input_value("#ex-smi") == "", "a valid SMILES is drawn and the field is emptied"
            assert pg.evaluate("document.querySelector('#smi-warn').hidden")
            pg.fill("#ex-smi", "not a smiles((("); pg.click("#ex-load"); pg.wait_for_timeout(2000)
            assert pg.input_value("#ex-smi") == "not a smiles(((" and not pg.evaluate("document.querySelector('#smi-warn').hidden"), "a wrong SMILES stays in the field with the warning"
            pg.fill("#ex-smi", "")
        step("6.1 the SMILES field empties after a good drawing, keeps a wrong one", smiles_clear)
        # ---------------------------------------------------------------- 2.3 selection SMILES, logP, exports
        def selection():
            setmol(pg, "CC(=O)Nc1ccc(O)cc1.Nc1ccc(O)cc1")
            box = pg.evaluate("(()=>{const f=document.getElementById('kframe').getBoundingClientRect();const k=%s;const r=k.editor.render;const o0=r.page2obj({clientX:0,clientY:0}),o1=r.page2obj({clientX:100,clientY:100}),sc=100/(o1.x-o0.x);const out=[];k.editor.struct().atoms.forEach((a,i)=>{out.push([i,(a.pp.x-o0.x)*sc,(a.pp.y-o0.y)*sc])});return {x:f.x,y:f.y,at:out}})()" % KQ)
            at = box["at"]; xs = [a[1] for a in at]; ys = [a[2] for a in at]
            # a) rectangle selection around the first structure (left part)
            xmid = (min(xs) + max(xs)) / 2
            def rect(x0, y0, x1, y1):
                pg.mouse.move(box["x"] + x0, box["y"] + y0); pg.mouse.down(); pg.mouse.move(box["x"] + x1, box["y"] + y1, steps=8); pg.mouse.up(); pg.wait_for_timeout(1800)
            rect(min(xs) - 20, min(ys) - 20, max(x for x in xs if x < xmid + 1) + 20, max(ys) + 20)
            assert pg.is_visible("#sel-card") and pg.inner_text("#sel-body").strip(), "no SMILES for a rectangle selection"
            assert pg.locator("#sel-body [data-cp]").count() >= 1
            print("selection SMILES:", pg.inner_text("#sel-body")[:80].replace("\n", " "))
            assert pg.evaluate(PROPS), "no logP rows with a selection"
            # b) select all programmatically (all the structures) and change of selection updates
            pg.evaluate("(()=>{const k=%s;k.editor.selection({atoms:[...k.editor.struct().atoms.keys()]})})()" % KQ); pg.wait_for_timeout(1800)
            n_all = pg.locator("#sel-body code").count(); assert n_all == 2, n_all
            pg.evaluate("(()=>{const k=%s;k.editor.selection({atoms:[...k.editor.struct().atoms.keys()].slice(0,2)})})()" % KQ); pg.wait_for_timeout(1800)
            assert pg.locator("#sel-body code").count() >= 1
            pg.evaluate(KQ + ".editor.selection(null)"); pg.wait_for_timeout(1000); assert not pg.is_visible("#sel-card")
            # c) click on one atom
            a0 = at[0]; pg.mouse.click(box["x"] + a0[1], box["y"] + a0[2]); pg.wait_for_timeout(1800)
            assert pg.is_visible("#sel-card"), "click on an atom: no SMILES"
        step("2.3 SMILES of the selection (rectangle, select all, partial, single click) and logP rows", selection)
        def logp_without_selection():
            pg.evaluate(KQ + ".editor.selection(null)"); pg.wait_for_timeout(1000)
            assert pg.evaluate(PROPS) == [] and "Seleziona una molecola" in pg.inner_text("#prop-body")
        step("2.3 logP box without selection: only the hint", logp_without_selection)
        def exports():
            setmol(pg, "CC(=O)Nc1ccc(O)cc1")
            def grab(fmt):
                with pg.expect_download() as d: pg.click("#ex-" + fmt)
                return d.value
            pat = re.compile(r"^disegno_\d{4}-\d\d-\d\d_\d{4}(_trasparente)?(-\d)?\.(png|jpg|svg|ket)$")
            ds = {f: grab(f) for f in ("png", "jpg", "svg", "ket")}
            names = {f: d.suggested_filename for f, d in ds.items()}; print(names)
            assert all(pat.match(n) for n in names.values()), names
            # same base name for the same moment; the four downloads may straddle a change of minute (07:23 -> 07:24), which is not an error
            hm = [re.search(r"\d{4}-\d\d-\d\d_(\d\d)(\d\d)", n) for n in names.values()]
            mins = sorted(int(m.group(1)) * 60 + int(m.group(2)) for m in hm)
            rest = {re.sub(r"(\d{4}-\d\d-\d\d)_\d{4}", r"\1", n).rsplit(".", 1)[0] for n in names.values()}
            assert mins[-1] - mins[0] <= 1 and len(rest) == 1, ("one base name per moment", names)
            png = open(ds["png"].path(), "rb").read(); assert png[:8] == b"\x89PNG\r\n\x1a\n" and b"tEXt" in png[:600] and b"mzLab" in png[:900], "PNG metadata"
            svg = open(ds["svg"].path(), encoding="utf-8").read(); assert "<title>" in svg
            assert open(ds["jpg"].path(), "rb").read(2) == b"\xff\xd8"
            assert open(ds["ket"].path(), encoding="utf-8").read().lstrip().startswith("{")
            again = grab("png").suggested_filename; assert again == names["png"].replace(".png", "-2.png"), again
            # transparent
            pg.check("#ex-nobg"); pg.wait_for_timeout(300)
            assert pg.is_disabled("#ex-jpg") and "trasparenza" in (pg.get_attribute("#ex-jpg", "title") or "")
            assert pg.evaluate("getComputedStyle(document.getElementById('ex-jpg')).opacity") < "0.6"
            assert pg.inner_text("label:has(#ex-nobg)").strip() == "Sfondo trasparente"
            t = grab("png"); tn = t.suggested_filename; assert tn.endswith("_trasparente.png") or "_trasparente-" in tn, tn
            from PIL import Image
            im = Image.open(t.path()).convert("RGBA"); assert im.getpixel((2, 2))[3] == 0 and im.getpixel((im.width - 3, im.height - 3))[3] == 0, "PNG corners not transparent"
            ts = grab("svg"); txt = open(ts.path(), encoding="utf-8").read()
            assert not re.search(r'<rect[^>]*fill="(rgb\(100%, ?100%, ?100%\)|#fff(fff)?|white)"', txt), "SVG still has the white background rectangle"
            pg.uncheck("#ex-nobg"); pg.wait_for_timeout(300); assert not pg.is_disabled("#ex-jpg")
            j = grab("jpg"); assert "trasparente" not in j.suggested_filename
            im = Image.open(j.path()).convert("RGB"); assert im.getpixel((2, 2)) == (255, 255, 255), im.getpixel((2, 2))
            # name field: odd characters
            pg.fill("#ex-name", "Paracetamolo è TP 1/2:"); pg.press("#ex-name", "Tab"); pg.wait_for_timeout(200)
            assert pg.input_value("#ex-name") == "paracetamolo_e_tp_1_2", pg.input_value("#ex-name")
            assert grab("png").suggested_filename == "paracetamolo_e_tp_1_2.png"
            pg.fill("#ex-name", ""); pg.press("#ex-name", "Tab"); assert re.match(r"^disegno_\d{4}", pg.input_value("#ex-name"))
        step("2.3 exports: names (4 formats, _trasparente, -2, odd characters), PNG metadata, transparent PNG/SVG, JPEG grey when transparent", exports)
        def export_error():
            pg.evaluate("window.__g=TPDraw.image; ")                               # the error message path: force a failure
            pg.evaluate("(()=>{const k=%s;k.generateImage=async()=>{throw new Error('prova')}})()" % KQ)
            pg.click("#ex-svg"); pg.wait_for_timeout(1500); assert pg.is_visible("#ex-warn") and "Esportazione non riuscita" in pg.inner_text("#ex-warn")
        step("2.3 a failing export shows a visible message", export_error)
finally:
    if r: r.close(); r.report()
    if srv: srv.terminate()
    print("page errors:", errs)
print("\nSTEPS"); [print(" ", s) for s in steps]
