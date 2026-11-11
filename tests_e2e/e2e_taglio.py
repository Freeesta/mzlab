"""Disegno, «Bond cuts»: a cut line drawn across the aryl-N bond of 4'-fluoroacetanilide breaks it homolytically (default), moves the two pieces apart,
puts a radical mark on each cut atom and labels each piece with formula (radical dot) and exact mass, like the fragmentation tool of a drawing program;
a charge put on a piece afterwards changes its label and gives an m/z; heterolytic asks which piece takes the +; two bonds of a ring give two pieces; undo brings the molecule back.
Needs site/static/wasm-feat/grafo (tools/build_feat_wasm.sh); SKIP without it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
if not os.path.exists(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "site", "static", "wasm-feat", "manifest.json")):
    print("SKIP site/static/wasm-feat is missing (tools/build_feat_wasm.sh site/static)"); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
LABELS = "[...document.querySelector('#kframe').contentDocument.querySelectorAll('svg g[id$=labels] text')].map(t=>t.textContent).join(' | ')"
r = Run(port=9131, wd="/tmp/wd_taglio")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        pg.select_option("#lb-dec", "4")
        def load(smi, n):
            pg.evaluate(f"{KJ}.setMolecule({smi!r})"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === {n}")
        def bond_pt(a_lbl, b_lbl):                                         # page point of the middle of the first bond between two element labels (for C-N: the aryl one, its C has no O), and its direction
            return pg.evaluate(f"""(()=>{{const K={KJ},st=K.editor.struct(),R=K.editor.render,o1=R.page2obj({{clientX:0,clientY:0}}),o2=R.page2obj({{clientX:100,clientY:100}});
              const f=document.querySelector('#kframe').getBoundingClientRect(),px=p=>[f.left+(p.x-o1.x)*100/(o2.x-o1.x),f.top+(p.y-o1.y)*100/(o2.y-o1.y)];
              for(const b of st.bonds.values()){{const a=st.atoms.get(b.begin),c=st.atoms.get(b.end);const l=[a.label,c.label];
                const nb=n=>[...st.bonds.values()].filter(x=>x.begin===n||x.end===n).map(x=>st.atoms.get(x.begin===n?x.end:x.begin).label);
                if((l[0]==={a_lbl!r}&&l[1]==={b_lbl!r}||l[1]==={a_lbl!r}&&l[0]==={b_lbl!r})&&(({a_lbl!r}!=='C'||{b_lbl!r}!=='N')||!nb(a.label==='C'?b.begin:b.end).includes('O'))){{
                  const p=px(a.pp),q=px(c.pp);return [(p[0]+q[0])/2,(p[1]+q[1])/2,q[0]-p[0],q[1]-p[1]]}}}}return null}})()""")
        def slash(pt):                                                    # a cut line across the bond: perpendicular, 60 px long
            x, y, dx, dy = pt; n = (dx * dx + dy * dy) ** 0.5; ux, uy = -dy / n * 30, dx / n * 30
            pg.mouse.move(x - ux, y - uy); pg.mouse.down(); pg.mouse.move(x, y, steps=4); pg.mouse.move(x + ux, y + uy, steps=4); pg.mouse.up()
        def homolytic():
            load("CC(=O)Nc1ccc(F)cc1", 11)
            pg.click("#cut-line"); assert pg.get_attribute("#cut-line", "aria-pressed") == "true"
            slash(bond_pt("C", "N"))
            wait_for_js(pg, f"{KJ}.editor.struct().bonds.size === 10")
            wait_for_js(pg, f"/C6H4F•.*95\\.0297/.test({LABELS}) && /C2H4NO•.*58\\.0293/.test({LABELS})", timeout=10000)
            rad = pg.evaluate(f"[...{KJ}.editor.struct().atoms.values()].filter(a=>a.radical===2).map(a=>a.label+a.implicitH).sort().join(',')")
            assert rad == "C0,N1", rad                                     # the aryl carbon and the amide N carry the mark, H unchanged
            gap = pg.evaluate(f"""(()=>{{const st={KJ}.editor.struct(),r=[...st.atoms.values()].filter(a=>a.radical===2);return Math.hypot(r[0].pp.x-r[1].pp.x,r[0].pp.y-r[1].pp.y)}})()""")
            assert gap > 1.6, gap                                          # moved apart along the cut (a bond is about 1)
            note = pg.inner_text("#cut-note"); assert "radicale" in note and "95.0297" in note, note
        step("a cut line across the aryl-N bond gives two radical pieces with formula and exact mass", homolytic)
        def oxidised():
            pg.evaluate(f"""(async()=>{{const K={KJ},j=JSON.parse(await K.getKet());for(const k in j)if(j[k].type==='molecule')for(const a of j[k].atoms)if(a.label==='N'){{a.charge=1;delete a.radical;a.implicitHCount=1}}await K.setMolecule(JSON.stringify(j))}})()""")
            wait_for_js(pg, f"/C2H4NO\\+ \\|.*m\\/z.*58\\.0287/.test({LABELS}) && /C6H4F•.*95\\.0297/.test({LABELS})", timeout=10000)
        step("the radical mark swapped for a + on the N afterwards: the label follows (even-electron cation, m/z)", oxidised)
        def undo():
            pg.keyboard.press("Escape"); assert pg.get_attribute("#cut-line", "aria-pressed") == "false"
            pg.evaluate("TPDraw.undo()"); pg.evaluate("TPDraw.undo()")
            wait_for_js(pg, f"{KJ}.editor.struct().bonds.size === 11 && ![...{KJ}.editor.struct().atoms.values()].some(a=>a.radical)")
        step("undo brings the bond back without radical marks", undo)
        def hetero():
            load("CC(=O)Nc1ccc(F)cc1", 11)
            pg.check("input[name=cut-mode][value=heterolytic]")
            pg.click("#cut-line"); slash(bond_pt("C", "N"))
            pg.wait_for_selector("#ctx:not([hidden]) div:has-text('Catione')", timeout=10000)
            rows = pg.locator("#ctx div", has_text="Catione").all_inner_texts(); assert len(rows) == 2, rows
            pg.locator("#ctx div", has_text="Catione C6H4F+").first.click()
            wait_for_js(pg, f"/C6H4F\\+.*m\\/z.*95\\.0292/.test({LABELS}) && /C2H4NO−.*m\\/z.*58\\.0298/.test({LABELS})", timeout=10000)
            assert "•" not in pg.evaluate(LABELS)
            pg.keyboard.press("Escape")
        step("heterolytic asks which piece takes the + and labels a cation and an anion", hetero)
        def ring():
            load("C1CCCCC1", 6); pg.check("input[name=cut-mode][value=homolytic]")
            ids = pg.evaluate(f"""(()=>{{const st={KJ}.editor.struct(),b=[...st.bonds.keys()];return [b[0],b[3]]}})()""")
            pg.evaluate(f"{KJ}.editor.selection({{bonds:{ids}}})")
            pt = bond_pt("C", "C")
            pg.evaluate("""([x,y])=>{const fr=document.querySelector('#kframe'),f=fr.getBoundingClientRect(),d=fr.contentDocument;const e=d.elementFromPoint(x-f.left,y-f.top)||d.body;e.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:x-f.left,clientY:y-f.top}))}""", pt[:2])
            pg.locator("#ctx div", has_text="Taglia i 2 legami").first.click()
            wait_for_js(pg, f"{KJ}.editor.struct().bonds.size === 4")
            wait_for_js(pg, f"({LABELS}).split('C3H6').length === 3", timeout=10000)
        step("two selected bonds of a ring, cut from the right-click menu, give two pieces", ring)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
