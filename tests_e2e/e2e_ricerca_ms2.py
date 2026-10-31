"""«What is it?»: the voice of the menu exists only for high-resolution MS2 spectra; identity and analogs together, level and evidence, stereoisomer groups,
the decision of the chemist (2b only there), the lines of the mirror (pairs of the golden file of Attributio), the thresholds; the master/detail window (layout at
1280 px, sorting, Options, download, 2D structure, hover, keyboard, axe)."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
from pathlib import Path
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ricerca_ms2.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
GOLD = json.loads((Path(__file__).resolve().parent.parent / "tests" / "golden" / "librerie" / "parita_attributio_ms2.json").read_text())
D = synth("ricms2")
r = Run(port=8981, wd="/tmp/wd_ricms2")
TRIO = "(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null),s2=E.panels.find(p=>p.dda!=null);return {s1,s2}})()"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1500})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); attendi(pg, "E.files.some(f=>f.kind==='ms2')")
        info = pg.evaluate("""(async()=>{const k=E.files.find(f=>f.kind==='ms2').k, d=await J('api/dda?k='+k), cnt={};d.tgt.forEach(t=>{if(t)cnt[t]=(cnt[t]||0)+1});
          const t=+Object.keys(cnt).sort((a,b)=>cnt[b]-cnt[a])[0], i=d.tgt.indexOf(t), s=await J('api/scan?k='+k+'&sid='+d.sid[i]);
          return {k, t, n:cnt[t], sid:d.sid[i], mz:s.mz, y:s.y}})()""")
        pk = [(m, y) for m, y in zip(info["mz"], info["y"]) if m < info["t"] - 1.5 and y > 0]
        def msp(name, prec, peaks, ik):
            return f"Name: {name}\nPrecursorMZ: {prec}\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C10H10\nInChIKey: {ik}\nLicense: CC BY 4.0\nAuthors: Autore di prova\nAccession: ACC001\nSMILES: CC(C)(C)NCC(O)COc1nsnc1N1CCOCC1\nNum Peaks: {len(peaks)}\n" + "\n".join(f"{m:.5f} {y:.1f}" for m, y in peaks) + "\n\n"
        text = (msp("Voce di prova", info["t"], pk, "AAAAAAAAAAAAAA-AAAAAAAAAA-N") + msp("Gemella di prova", info["t"], pk, "AAAAAAAAAAAAAA-BBBBBBBBBB-N")
                + msp("Analogo di prova", info["t"] - 15.9949, [(m if i < len(pk) // 2 else m - 15.9949, y) for i, (m, y) in enumerate(pk)], "CCCCCCCCCCCCCC-AAAAAAAAAA-N"))
        pg.evaluate("(async t=>{await LIB.call('add',{file:new File([t],'prova.msp')})})(%s)" % json.dumps(text))
        def open_menu():
            pg.evaluate("(async()=>{const t=" + TRIO + ";await DDA.selectMs2(t.s2," + str(info["sid"]) + ")})()"); attendi(pg, "(()=>{const t=" + TRIO + ";return !!(t.s2&&t.s2._a)})()"); frames(pg, 3)
            pos = pg.evaluate("(()=>{const t=" + TRIO + ";t.s2.el.scrollIntoView({block:'center'});const r=t.s2.cv.getBoundingClientRect();return {x:r.left+r.width*0.5,y:r.top+r.height*0.3}})()")
            pg.mouse.click(pos["x"], pos["y"], button="right"); attendi(pg, "!document.querySelector('#ctx').hidden && document.querySelector('#ctx').innerText.includes(\"Cos'è?\")")
            assert "Cos'è?" in pg.inner_text("#ctx"), pg.inner_text("#ctx")
        step("the MS2 panel of the DDA trio has the voice «Cos'è? (cerca nelle librerie)»", open_menu)
        def result():
            pg.click("#ctx >> text=Cos'è?"); pg.wait_for_selector("#libwhat[open] #lw-list table", timeout=60000); pg.set_viewport_size({"width": 1280, "height": 800}); attendi(pg, "/Analoghi \\([1-9]/.test(document.querySelector('#lw-tab-an').textContent)")
            t = pg.inner_text("#libwhat")
            assert "Candidati (" in t and "Analoghi (" in t, t[:300]
            names = pg.locator("#libwhat tr.row[data-k=id]:not([hidden]) td:nth-child(1)").all_inner_texts()
            assert names and (names[0].startswith("Voce di prova") or names[0].startswith("Gemella")), names
            head = pg.inner_text("#lw-list thead")
            for col in ("Nome", "Entropia", "Picchi in comune", "Δ ppm", "Addotto", "Libreria"):
                assert col in head, (col, head)
            pg.click("#lw-more summary"); m = pg.inner_text("#lw-more")
            for col in ("Reverse", "Intensità spiegata", "Polarità", "Licenza", "CC BY", "InChIKey"):
                assert col in m, (col, m)
        step("identity and analogs as two tabs; six columns in the table, the rare fields behind «Dettagli»", result)
        def layout():
            g = pg.evaluate("""(() => { const d = document.querySelector('#libwhat').getBoundingClientRect(), L = document.querySelector('#lw-list'), ths = [...L.querySelectorAll('thead th')].map(h => h.getBoundingClientRect());
              const tw = c => { const r = document.createRange(); r.selectNodeContents(c); return r.getBoundingClientRect().width; }, inner = c => { const k = getComputedStyle(c); return c.getBoundingClientRect().width - parseFloat(k.paddingLeft) - parseFloat(k.paddingRight); };
              const cut = [...L.querySelectorAll('tr.row:not([hidden]) td.num')].filter(c => tw(c) > inner(c) + 0.5 || getComputedStyle(c).whiteSpace !== 'nowrap').length;
              return {ws: [...L.querySelectorAll('tr.row:not([hidden]) td.num')].slice(0, 6).map(c => [c.textContent, tw(c), inner(c)]), dw: d.width, vw: innerWidth, over: L.scrollWidth - L.clientWidth, lr: L.getBoundingClientRect().right, ths: ths.map(h => [h.left, h.right, h.width]), cut,
                page: document.querySelector('#libwhat').scrollWidth - document.querySelector('#libwhat').clientWidth}; })()""")
            assert g["dw"] >= 0.9 * g["vw"], g                      # near full-screen
            assert g["over"] <= 1 and g["page"] <= 1, g             # no horizontal overflow
            assert g["ths"][0][2] >= 160, g["ths"]                  # the name is not squeezed
            assert all(h[1] <= g["lr"] + 1 for h in g["ths"][:4]), g   # Name, Entropy, Shared peaks, Δ ppm visible at 1280 px
            assert g["cut"] == 0, (g["cut"], g["ws"])                # numbers on one line, never cut
        step("1280 px: near full-screen window, table without horizontal overflow, the first four columns visible, numbers on one line", layout)
        def groups():
            b = pg.locator("#libwhat [data-grp]"); assert b.count() == 1, b.count()          # the twin shares the first InChIKey block
            assert "stesso primo blocco" in b.first.get_attribute("title")
            assert pg.locator("#libwhat tr.mem[hidden]").count() == 1
            b.first.click(); attendi(pg, "document.querySelectorAll('#libwhat tr.mem:not([hidden])').length === 1")
        step("entries with the same first InChIKey block are grouped (stereoisomers), expandable", groups)
        def sorting():
            pg.click("#lw-list [data-sort=name]"); attendi(pg, "document.querySelector('#lw-list th[aria-sort=ascending] [data-sort=name]') !== null")
            n = pg.locator("#libwhat tr.row[data-k=id]:not(.mem) td:nth-child(1)").all_inner_texts(); assert n == sorted(n), n
            pg.click("#lw-list [data-sort=ent]"); attendi(pg, "document.querySelector('#lw-list th[aria-sort=descending] [data-sort=ent]') !== null")
        step("sortable columns (aria-sort), groups kept together", sorting)
        def level():
            v = pg.inner_text("#lw-verdict")
            assert "Miglior candidato" in v and pg.locator("#lw-verdict .lvb").count() == 1, v
            assert "è " not in pg.inner_text("#lw-verdict .lvb"), "never «it is X»"
            pg.click("#lw-verdict summary"); v = pg.inner_text("#lw-verdict")
            assert "Prove" in v and "Livello proposto" in v and "Livello deciso" in v, v
            assert "entropia" in v and "vs" in v and ("✓" in v or "✗" in v), v
            n = pg.evaluate("LIB.assess([{name:'x',ent:.9,shared:8,dprec:1}],{nScans:1}).proposed"); assert n == "3a"
        step("one headline (badge + best candidate); evidence, proposed and decided level behind «Dettagli»", level)
        def decision():
            lv = pg.evaluate("document.querySelector('#lw-verdict .lvb').textContent.replace('L','')")
            assert lv in ("2a", "3a"), lv
            pg.select_option("#libwhat #lv-dec", "2b"); attendi(pg, "document.querySelector('#lw-verdict .lvb').textContent === 'L2b'")
            assert "Livello proposto: L" + lv in pg.inner_text("#lw-verdict").replace("\n", " "), "the proposal stays"
            pg.select_option("#libwhat #lv-dec", "1"); attendi(pg, "document.querySelector('#libwhat #lv-msg').innerText.includes('standard')")
            assert pg.input_value("#libwhat #lv-dec") == "2b"
            pg.select_option("#libwhat #lv-dec", ""); frames(pg, 3)
        step("2b only by the decision of the chemist; 1 refused without a standard", decision)
        def options():
            assert not pg.is_visible("#libwhat [data-thr=entropia_min]")
            pg.click("#lw-opt summary"); assert pg.is_visible("#libwhat [data-thr=entropia_min]") and pg.is_visible("#lw-tol") and pg.is_visible("#lw-libs")
            pg.fill("#libwhat [data-thr=entropia_min]", "0,999"); pg.dispatch_event("#libwhat [data-thr=entropia_min]", "change"); attendi(pg, "(() => { try { return JSON.parse(localStorage.getItem('mzlab.lib.soglie')).mzlab.entropia_min === 0.999; } catch (e) { return false; } })()")
            wf = pg.evaluate("LIVELLI.exportState()"); assert wf["soglie"]["entropia_min"] == 0.999 and wf["formato"] == "livelli/1"
            pg.click("#lw-thr-reset"); attendi(pg, "LIVELLI.thresholds().entropia_min === 0.75")
            pg.keyboard.press("Escape"); frames(pg, 2)
            assert pg.evaluate("document.querySelector('#libwhat').open && !document.querySelector('#lw-opt').open"), "Esc closes the popup, not the window"
        step("«Opzioni…»: tolerances, thresholds (saved, exported, reset) and libraries in one popup; Esc closes it", options)
        def download():
            pg.click("#lw-dl summary")
            with pg.expect_download() as dl: pg.click("#lw-xlsx")
            assert dl.value.suggested_filename.endswith(".xlsx")
            assert not pg.evaluate("document.querySelector('#lw-dl').open")
        step("the download button holds copy and Excel", download)
        def structure():
            pg.locator("#libwhat tr.row[data-k=id]").first.click()
            pg.wait_for_selector("#lw-mol svg", timeout=20000)
            assert pg.locator("#lw-mol svg line, #lw-mol svg path").count() > 5 and pg.get_attribute("#lw-mol", "role") == "img"
        step("the 2D structure of the selected candidate (SMILES of the library entry)", structure)
        def mirror_lines():
            lines = json.loads(pg.get_attribute("#lw-cv", "data-lines")); assert len(lines) >= 4 and all(l[2] == 0 for l in lines), lines
            pg.click("#lw-tab-an"); pg.locator("#libwhat tr.row[data-k=an]").first.click(); attendi(pg, "(() => { const c = document.querySelector('#lw-cv'); if (!c) return false; try { return JSON.parse(c.getAttribute('data-lines')).some(l => l[2] === 1); } catch (e) { return false; } })()")
            assert "Analogo di prova" in pg.inner_text("#libwhat tr.row[data-k=an]")
            ln = json.loads(pg.get_attribute("#lw-cv", "data-lines")); assert any(l[2] == 1 for l in ln), ln
            if not pg.evaluate("document.querySelector('#lw-more').open"): pg.click("#lw-more summary")
            assert "Perdite neutre" in pg.inner_text("#lw-more")
            n = pg.locator("#lw-more table.lib-pairs tr").count() - 1   # the table lists the pairs behind the score, shifted ones flagged
            assert n >= len(ln) and pg.locator("#lw-more table.lib-pairs th[scope=col]").count() == 6, n
        step("the mirror joins the paired peaks (shifted ones too for an analog) and lists the shared neutral losses", mirror_lines)
        def hovering():
            x = pg.evaluate("""(() => { const c = document.querySelector('#lw-cv'), g = c._mg, b = c.getBoundingClientRect(), p = g.r.pairs[0];
              return {x: b.left + g.X(g.q.mz[p[0]]), y: b.top + g.mid - g.q.it[p[0]] * g.half / 2}; })()""")
            pg.mouse.move(x["x"], x["y"]); attendi(pg, "!document.querySelector('.lw-tip').hidden")
            tip = pg.inner_text(".lw-tip"); assert "m/z" in tip and "appaiato" in tip, tip
        step("hover on a peak of the mirror: m/z, intensity and its partner with Δ", hovering)
        def keyboard():
            pg.click("#lw-tab-id"); pg.locator("#libwhat tr.row[data-k=id]:not([hidden])").first.focus(); pg.keyboard.press("ArrowDown")
            attendi(pg, "document.activeElement.matches('tr.row.sel') && document.activeElement !== document.querySelector('#libwhat tr.row[data-k=id]:not([hidden])')")
            pg.focus("#lw-tab-id"); pg.keyboard.press("ArrowRight"); attendi(pg, "document.querySelector('#lw-tab-an').getAttribute('aria-selected') === 'true'")
            pg.keyboard.press("ArrowLeft")
        step("keyboard: arrows move along the rows and between the tabs", keyboard)
        def attribution():
            pg.locator("#libwhat tr.row[data-k=id]").first.click(); attendi(pg, "document.querySelector('#lw-more').innerText.includes('ACC001')")
            assert "Autore di prova" in pg.inner_text("#lw-more"), pg.inner_text("#lw-more")
        step("authors, accession and license behind «Dettagli»", attribution)
        def axe():
            a = axe_path()
            if not a: print("   axe-core not available: skipped"); return
            pg.add_script_tag(path=a)
            v = pg.evaluate("""() => axe.run(document.querySelector('#libwhat'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
              .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))""")
            assert not v, v
        step("axe-core: no serious or critical violation in the window", axe)
        def shot():
            out = os.environ.get("MZLAB_SHOT")
            if out:
                pg.evaluate("document.querySelectorAll('#libwhat details[open]').forEach(d => d.open = false)"); pg.screenshot(path=out)
            errs = [e for e in r.errs if e[0] in ("pageerror", "console.error")]; assert not errs, errs[:3]
            pg.click("#lw-x"); assert not pg.evaluate("document.querySelector('#libwhat').open")
        step("no page or console error; the window closes", shot)
        def golden_pairs():
            ok = 0
            for tipo in ("stesso", "analogo", "chimerico"):
                cs = [c for c in GOLD["casi"] if c["tipo"] == tipo and c["match_tol0.02"]["pairs"]]
                if tipo == "analogo": cs = [c for c in cs if c["match_tol0.02"]["n_shifted"] > 0]
                assert cs, tipo; c = cs[0]
                got = pg.evaluate("""(c) => { const cv = document.createElement('canvas'); cv.style.width = '900px'; cv.style.height = '320px'; document.body.appendChild(cv);
                  const nz = a => { const m = Math.max(...a); return a.map(v => v / m); };
                  LIB.mirror(cv, {mz: c.a.mz, it: nz(c.a.int), hr: true}, {mz: c.b.mz, it: nz(c.b.int), prec: c.b.prec, name: 'x', shared: c.match['matched'], pairs: c.match.pairs});
                  const out = JSON.parse(cv.dataset.lines); cv.remove(); return out; }""", {"a": c["a"], "b": c["b"], "match": c["match_tol0.02"]})
                assert sorted(map(tuple, got)) == sorted(tuple(x) for x in c["match_tol0.02"]["pairs"]), (c["id"], got[:5])
                ok += 1
            assert ok == 3
        step("3 cases of the golden file of Attributio: the lines of the mirror join exactly those pairs (type 0 and 1)", golden_pairs)
        def cosvis():
            q = {"it": [1, .5, .2], "mz": [100, 200, 300]}
            v = pg.evaluate("(q) => LIB.cosVis({it: q.it}, {mz: q.mz, it: [1, .5, .2], pairs: [[0,0],[1,1],[2,2]]}, 150)", q)
            assert 0 < v < 1 and abs(v - ((.5 * 1) ** 0 and (0.5 + 0.2) / ((1.7) ** .5 * (0.7) ** .5))) < 1e-9, v
            assert pg.evaluate("LIB.cosVis({it:[1]}, {mz:[100], it:[1], pairs:[[0,0]]}, 50)") is None          # nothing cut: no extra proof
        step("cosine on the visible peaks (reference peaks below the scan window are cut), a proof that does not change the level", cosvis)
    r.close()
    with sync_playwright() as p:
        r = Run(port=8982, wd="/tmp/wd_ricms2lr"); pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_MS2-t15")]); pg.wait_for_selector("text=Carica dati")
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('ms2',true)"); attendi(pg, "(() => { const p = E.panels.find(p => p.type === 'spec' && p.level === 2); return !!(p && p._a && p._a.data && p._a.data[0]); })()")
        def lr_none():
            SPEC = "E.panels.find(p => p.type === 'spec' && p.level === 2)"
            assert pg.evaluate(f"LIB.isHr({SPEC})") is False
            c = pg.evaluate(f"(() => {{ const p = {SPEC}; p.el.scrollIntoView({{block:'center'}}); const q = p.cv.getBoundingClientRect(); return {{x: q.left + q.width * 0.5, y: q.top + q.height * 0.3}}; }})()")
            pg.mouse.click(c["x"], c["y"], button="right"); attendi(pg, "!document.querySelector('#ctx').hidden && document.querySelector('#ctx').innerText.trim() !== ''")
            t = pg.inner_text("#ctx"); assert "Cos'è?" not in t, t
        step("low resolution: the voice «Cos'è?» does not exist (the program gives no answers)", lr_none)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
