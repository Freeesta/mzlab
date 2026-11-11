"""Disegno, radicals in the mass spectrometry reading: a radical cation drawn on an amine N is labelled CH5N+• with the m/z of the ion (M+•, mass − e);
a cut of a bond next to a radical mark leaves a methyl radical CH3• and a carbene CH2•• (triplet mark), or, heterolytic, the odd-electron ion CH2+•;
the «Fragments» rows of a bond mark with • the side that keeps a drawn radical (its [F+H]+ is an odd-electron ion) and the table note says why."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
KJ = "document.querySelector('#kframe').contentWindow.ketcher"
LABELS = "[...document.querySelector('#kframe').contentDocument.querySelectorAll('svg g[id$=labels] text')].map(t=>t.textContent).join(' | ')"
def mol(atoms, bonds, props):                                             # V2000 molfile: atoms on a line, bonds 1-based, property lines
    a = "".join(f"{i * 1.0:10.4f}    0.0000    0.0000 {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n" for i, e in enumerate(atoms))
    b = "".join(f"{x:3d}{y:3d}  1  0\n" for x, y in bonds)
    return f"\n  qqq\n\n{len(atoms):3d}{len(bonds):3d}  0  0  0  0  0  0  0  0999 V2000\n{a}{b}{props}M  END\n"
r = Run(port=9138, wd="/tmp/wd_radicali_ms")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
        pg.select_option("#lb-dec", "4")
        def load(m, n):
            pg.evaluate(f"m => {KJ}.setMolecule(m)", m); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === {n}")
        def bond_pt(frame=False):                                         # middle of the first bond and its direction, in page (or frame) coordinates
            return pg.evaluate(f"""(fr=>{{const K={KJ},st=K.editor.struct(),R=K.editor.render,o1=R.page2obj({{clientX:0,clientY:0}}),o2=R.page2obj({{clientX:100,clientY:100}});
              const f=fr?{{left:0,top:0}}:document.querySelector('#kframe').getBoundingClientRect(),px=p=>[f.left+(p.x-o1.x)*100/(o2.x-o1.x),f.top+(p.y-o1.y)*100/(o2.y-o1.y)];
              const b=[...st.bonds.values()][0],p=px(st.atoms.get(b.begin).pp),q=px(st.atoms.get(b.end).pp);return [(p[0]+q[0])/2,(p[1]+q[1])/2,q[0]-p[0],q[1]-p[1]]}})""", frame)
        def slash():                                                      # a cut line across the bond, 60 px long
            x, y, dx, dy = bond_pt(); n = (dx * dx + dy * dy) ** 0.5; ux, uy = -dy / n * 30, dx / n * 30
            pg.mouse.move(x - ux, y - uy); pg.mouse.down(); pg.mouse.move(x, y, steps=4); pg.mouse.move(x + ux, y + uy, steps=4); pg.mouse.up()
        ETHYL = mol(["C", "C"], [(1, 2)], "M  RAD  1   2   2\n")             # CH3-CH2•
        def radical_cation():
            load(mol(["C", "N"], [(1, 2)], "M  CHG  1   2   1\nM  RAD  1   2   2\n"), 2)
            wait_for_js(pg, f"/CH5N\\+•.*m\\/z 31\\.0417/.test({LABELS})", timeout=10000)
        step("an amine N with charge and radical mark is labelled CH5N+• with the m/z of M+• (31.0417)", radical_cation)
        def cut_homolytic():
            load(ETHYL, 2); pg.check("input[name=cut-mode][value=homolytic]")
            pg.click("#cut-line"); slash()
            wait_for_js(pg, f"{KJ}.editor.struct().bonds.size === 0")
            wait_for_js(pg, f"/CH3•.*15\\.0235/.test({LABELS}) && /CH2••.*14\\.0157/.test({LABELS})", timeout=10000)
            rad = pg.evaluate(f"[...{KJ}.editor.struct().atoms.values()].map(a=>a.radical+':'+a.implicitH).sort().join(',')")
            assert rad == "2:3,3:2", rad                                   # a doublet methyl, and a triplet carbene where the mark was
            pg.keyboard.press("Escape")
        step("a homolytic cut next to a radical gives CH3• and the carbene CH2•• (triplet)", cut_homolytic)
        def cut_heterolytic():
            load(ETHYL, 2); pg.check("input[name=cut-mode][value=heterolytic]")
            pg.click("#cut-line"); slash()
            row = pg.locator("#ctx div", has_text="Catione CH2"); row.first.wait_for(timeout=10000); row.first.click()
            wait_for_js(pg, f"/CH2\\+•.*m\\/z 14\\.0151/.test({LABELS})", timeout=10000)
            pg.keyboard.press("Escape"); pg.check("input[name=cut-mode][value=homolytic]")
        step("a heterolytic cut that gives the + to the radical side gives the odd-electron ion CH2+• (m/z 14.0151)", cut_heterolytic)
        def fragments():
            load(mol(["C", "C", "O"], [(1, 2), (2, 3)], "M  RAD  1   2   2\n"), 3)   # CH3-CH•-OH
            for _ in range(20):
                x, y, _dx, _dy = bond_pt(frame=True)
                pg.evaluate("""([x,y])=>{const d=document.querySelector('#kframe').contentDocument;const e=d.elementFromPoint(x,y)||d.body;e.dispatchEvent(new d.defaultView.MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:x,clientY:y}))}""", [x, y])
                try: wait_for_js(pg, "[...document.querySelectorAll('#ctx:not([hidden]) div')].filter(d=>d.textContent.includes('m/z')).length >= 2", timeout=1000); break
                except Exception: pass                                    # the bridge was not ready yet: right-click again
            t = pg.locator("#ctx:not([hidden]) div", has_text="m/z").all_inner_texts()
            assert any(s.startswith("CH4 ") and "m/z 17.0386" in s for s in t), t
            assert any(s.startswith("CH3O• ") and "m/z 32.0257" in s for s in t), t
            pg.locator("#ctx:not([hidden]) div", has_text="Tutte le rotture").first.click(); pg.wait_for_selector("#tdati table tbody tr", timeout=5000)
            body = pg.inner_text("#tdati"); assert "CH3O•" in body and "elettroni dispari" in body, body[:400]
            pg.keyboard.press("Escape")
        step("the Fragments rows and table mark with • the side that keeps the radical (CH3O•, m/z 32.0257)", fragments)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
