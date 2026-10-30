"""H8/H9: identify all the MS2 of a high-resolution file against a library (grouping, verdict, the 1-scan rule), the analogue search, the verdict thresholds."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_identificazione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrid")
r = Run(port=8970, wd="/tmp/wd_hrid")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        def verdicts():
            v = pg.evaluate("""[LIB.assess([{name:'x',ent:.9,shared:8,dprec:2}],{nScans:3}).proposed, LIB.assess([{name:'x',ent:.9,shared:8,dprec:9}],{nScans:3}).proposed, LIB.assess([{name:'x',ent:.65,shared:3,dprec:1}],{nScans:3}).proposed,
                LIB.assess([{name:'x',ent:.3,shared:9,dprec:1}],{nScans:3}).proposed, LIB.assess([],{}).proposed,
                LIB.groupAssess({n:1,best:{name:'x',ent:.9,shared:8,dprec:1}}).proposed, LIB.groupAssess({n:3,best:{name:'x',ent:.9,shared:8,dprec:1}}).proposed,
                LIB.groupAssess({n:3,best:{name:'x',ik:'AAAAAAAAAAAAAA-A',ent:.9,shared:8,dprec:1,alt:{name:'y',ik:'BBBBBBBBBBBBBB-A',ent:.88,shared:8,dprec:1}}}).proposed]""")
            assert v == ["2a", "3a", "3a", "5", "5", "3a", "2a", "3a"], v
        step("levels: library match 2a, thresholds, a single scan or a close competitor gives 3a, nothing 5", verdicts)
        def grouping():
            g = pg.evaluate("""LIB.groupRows([{prec:300.0000,rt:1.00,ent:.5,name:'a'},{prec:300.0010,rt:1.10,ent:.9,name:'a'},{prec:300.0020,rt:3.00,ent:.4,name:'a'},{prec:400,rt:1.0,ent:null,name:''}]).map(x=>[x.n,Math.round(x.rt*100)/100,x.best.ent])""")
            assert g == [[2, 1.1, 0.9], [1, 1.0, None], [1, 3.0, 0.4]] or sorted(map(tuple, g)) == sorted([(2, 1.1, 0.9), (1, 1.0, None), (1, 3.0, 0.4)]), g
        step("the scans of one precursor (10 ppm) and one peak (RT within 0.2 min) are one row", grouping)
        # a library made of the file itself: the most frequent precursor, and an analogue 15.9949 lighter with the same fragments
        info = pg.evaluate("""(async()=>{const k=E.files.find(f=>f.kind==='ms2').k, d=await J('api/dda?k='+k), cnt={};d.tgt.forEach(t=>{if(t)cnt[t]=(cnt[t]||0)+1});
          const t=+Object.keys(cnt).sort((a,b)=>cnt[b]-cnt[a])[0], i=d.tgt.indexOf(t), s=await J('api/scan?k='+k+'&sid='+d.sid[i]);
          return {k, t, n:cnt[t], mz:s.mz, y:s.y}})()""")
        pk = [(m, y) for m, y in zip(info["mz"], info["y"]) if m < info["t"] - 1.5 and y > 0]
        def msp(name, prec, peaks):
            return f"Name: {name}\nPrecursorMZ: {prec}\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C10H10\nSMILES: CC(=O)Oc1ccccc1C(=O)O\nInChIKey: BSYNRYMUTXBXSQ-UHFFFAOYSA-N\nNum Peaks: {len(peaks)}\n" + "\n".join(f"{m:.5f} {y:.1f}" for m, y in peaks) + "\n\n"
        text = msp("Voce di prova", info["t"], pk) + msp("Analogo di prova", info["t"] - 15.9949, pk)
        pg.evaluate("(async t=>{await LIB.call('add',{file:new File([t],'prova.msp')})})(%s)" % __import__("json").dumps(text)); pg.wait_for_timeout(500)
        def identify():
            pg.evaluate(f"LIB.identifyAll(E.files.find(f=>f.kind==='ms2').k)"); pg.wait_for_selector("#li-body table", timeout=60000)
            t = pg.inner_text("#li-body"); print(t[:300])
            assert "Voce di prova" in t and pg.locator("#li-body .lvb").count() >= 1, t
            assert "scansion" in pg.inner_text("#li-st") or "confrontate" in pg.inner_text("#li-st")
            assert pg.is_visible("#libwhat") and "Livello" in pg.inner_text("#li-list thead")
            assert pg.evaluate("(() => { const L = document.querySelector('#li-list'); return L.scrollWidth - L.clientWidth; })()") <= 1, "no horizontal overflow"
            xs = pg.evaluate("[...document.querySelectorAll('#li-list tr')].map(tr=>[...tr.children].map(c=>Math.round(c.getBoundingClientRect().x)))")
            assert len(xs) > 10 and all(len(x) == len(xs[0]) for x in xs), "header and rows have the same columns"
            assert all(all(abs(a - b) <= 1 for a, b in zip(x, xs[0])) for x in xs), ("the column x positions are the same in every row", xs[:3])
            assert pg.evaluate("(()=>{const h=document.querySelector('#li-list th'),sc=document.querySelector('#li-list');sc.scrollTop=200;return getComputedStyle(h).position==='sticky'&&Math.abs(h.getBoundingClientRect().top-sc.getBoundingClientRect().top)<=2&&getComputedStyle(document.querySelector('#li-list td.num')).textAlign==='right'})()"), "sticky header, numbers on the right"
            w0 = pg.evaluate("document.querySelector('#libwhat').offsetWidth")
            pg.click("#libwhat .lw-mx"); assert pg.evaluate("document.querySelector('#libwhat').offsetWidth") > w0 or w0 >= pg.evaluate("innerWidth*0.95"), "maximised"
            assert pg.get_attribute("#libwhat .lw-mx", "aria-pressed") == "true"
            pg.click("#libwhat .lw-mx"); assert pg.get_attribute("#libwhat .lw-mx", "aria-pressed") == "false"
            pg.evaluate("(()=>{const d=document.querySelector('#libwhat');d.style.width='900px';d.style.height='600px'})()"); pg.wait_for_function("(JSON.parse(localStorage.getItem('mzlab.libwhat.size')||'null')||{}).w===900", timeout=5000)
            pg.evaluate("(()=>{const d=document.querySelector('#libwhat');d.style.width='';d.style.height='';localStorage.removeItem('mzlab.libwhat.size')})()")
            pg.click("#lw-dl summary")
            with pg.expect_download() as dl: pg.click("#li-xlsx")
            assert dl.value.suggested_filename.startswith("identificazione") and os.path.getsize(dl.value.path()) > 1500
            pg.locator("#li-list tr.row", has_text="Voce di prova").first.click(); pg.wait_for_selector("#libwhat #lw-list tr.row[data-k=id]", timeout=60000)
            assert "Voce di prova" in pg.inner_text("#lw-list") and pg.is_visible("#lw-cv") and pg.is_visible("#lw-goto")
            pg.click("#lw-back"); pg.wait_for_selector("#li-list tr.row", timeout=10000)
            pg.evaluate("document.querySelector('#libwhat').close()")
        step("identify all the MS2 of the file: one row per precursor with the level; a row opens «Cos'è?» of its scan in the same window, «Tutte le MS2» comes back", identify)
        def excluded():
            pol = pg.evaluate("E.files.find(f=>f.kind==='ms2').polarity")
            pg.evaluate("LISTE.setOn(true)"); pg.evaluate("LISTE.ready()"); assert pg.evaluate("LISTE.isOn()"), "the contaminant lists are on"      # off by default
            prec = pg.evaluate("(async()=>{const k=E.files.find(f=>f.kind==='ms2').k;const d=await J('api/dda?k='+k);const c={};d.tgt.forEach(t=>{if(t)c[t]=(c[t]||0)+1});return +Object.keys(c).sort((a,b)=>c[b]-c[a])[0]})()")
            pg.evaluate(f"LISTE.addLab({prec}, {{name:'Rumore di prova', polarity:{1 if pol == 'positive' else -1}}})"); pg.wait_for_timeout(400)
            pg.evaluate(f"[...new Set(LISTE.compat({prec}, '{pol}', 5).all.map(x => x.list))].forEach(l => LISTE.exclSet('list', l, true))")          # every list this precursor is compatible with
            pg.evaluate("LIB.identifyAll(E.files.find(f=>f.kind==='ms2').k)"); pg.wait_for_selector("#li-body .cont-cnt", timeout=60000)
            t = pg.inner_text("#li-body")
            assert "Voce di prova" not in t and "nascosti" in pg.inner_text("#li-body .cont-cnt"), t[:300]
            pg.click("#li-chid"); pg.wait_for_function("document.querySelector('#li-body').innerText.includes('Voce di prova')", timeout=10000)
            pg.click("#li-chid"); assert "Voce di prova" not in pg.inner_text("#li-body")
            pg.evaluate("document.querySelector('#libwhat').close(); LISTE.exclReset(); LISTE.setOn(false)")
        step("a candidate whose precursor is an excluded contaminant stays out of the list, counted, until «Mostra»", excluded)
        def analogs():
            r = pg.evaluate("""(async()=>{const q=%s;const j=await LIB.call('analogs',{peaks:q.mz.map((m,i)=>[m,q.y[i]]),prec:q.t,pol:1,frag:0.01,maxDelta:200,max:10});return j.results.map(x=>({name:x.name,delta:x.delta,matched:x.matched,mcos:x.mcos}))})()""" % __import__("json").dumps(info))
            assert r and r[0]["name"] == "Analogo di prova" and abs(r[0]["delta"] - 15.9949) < 1e-3 and r[0]["matched"] >= 4 and r[0]["mcos"] > 0.9, r
            assert all(x["name"] != "Voce di prova" for x in r), "the identity (same precursor) is not an analogue"
        step("the analogue search finds the entry 16 Da lighter, with its delta and a modified cosine, not the identity", analogs)
        def extras():
            pg.set_viewport_size({"width": 1280, "height": 900})
            text = pg.evaluate("""(async()=>{const k=E.files.find(f=>f.kind==='ms2').k, d=await J('api/dda?k='+k), seen=new Set(); let out='', n=0;
              for (let i=0;i<d.sid.length&&n<40;i++){const t=d.tgt[i]; if(!t||seen.has(t))continue; seen.add(t); const s=await J('api/scan?k='+k+'&sid='+d.sid[i]); if(s.mz.length<3)continue;
                out+='Name: Fila '+(n+1)+'\\nPrecursorMZ: '+t+'\\nPrecursor_type: [M+H]+\\nIon_mode: Positive\\nFormula: C10H10\\n'+(n%5===4?'':'SMILES: '+(n%2?'CCO':'c1ccccc1O')+'\\n')+'Num Peaks: '+s.mz.length+'\\n'+s.mz.map((m,j)=>m.toFixed(5)+' '+s.y[j].toFixed(1)).join('\\n')+'\\n\\n'; n++;}
              return out})()""")
            pg.evaluate("(async t=>{await LIB.call('add',{file:new File([t],'molte.msp')})})(%s)" % __import__("json").dumps(text)); pg.wait_for_timeout(800)
            pg.evaluate("LIB.identifyAll(E.files.find(f=>f.kind==='ms2').k)"); pg.wait_for_selector("#li-list tr.row", timeout=60000)
            pg.evaluate("document.querySelector('#li-list').style.maxHeight='190px'")
            heads = pg.inner_text("#li-list thead")
            for w in ("Struttura", "Nome", "Addotto", "Libreria", "InChIKey", "Entropia migliore", "Picchi in comune", "Δ ppm"): assert w in heads, (w, heads)
            geo = pg.evaluate("""(()=>{const L=document.querySelector('#li-list'),r=L.getBoundingClientRect(); const th=[...L.querySelectorAll('th')]; const by=t=>th.find(x=>x.innerText.includes(t)).getBoundingClientRect();
              return {over:L.scrollWidth-L.clientWidth, name:by('Nome').width, right:Math.max(...['Entropia','Picchi','Δ ppm'].map(t=>by(t).right))-r.right, left:Math.min(...['Entropia','Picchi','Δ ppm'].map(t=>by(t).left))}})()""")
            assert geo["over"] <= 1 and geo["name"] >= 100 and geo["right"] <= 1, ("columns fit at 1280 px", geo)
            tot = pg.locator("#li-list .lw-th").count(); assert tot >= 8, tot
            pg.wait_for_function("document.querySelectorAll('#li-list .lw-th svg').length > 0", timeout=20000); pg.wait_for_timeout(500)
            drawn = pg.locator("#li-list .lw-th svg").count(); assert 0 < drawn < tot, ("only the rows in view are drawn", drawn, tot)
            assert pg.locator("#li-list tr.row td.st", has_text="–").count() >= 1, "rows without a SMILES show a dash"
            pg.evaluate("document.querySelector('#li-list').scrollTop = 1e6"); pg.wait_for_function(f"document.querySelectorAll('#li-list .lw-th svg').length > {drawn}", timeout=20000)
            pg.evaluate("document.querySelector('#li-list').scrollTop = 0"); pg.wait_for_timeout(300)
            btn = pg.locator("#li-list .lw-th").first; smi = btn.get_attribute("data-smi"); btn.click()
            pg.wait_for_selector("#li-big .lw-bigsvg svg", timeout=20000)
            assert pg.is_visible("#li-list tr.row") and pg.inner_text("#li-big code") == smi, "the SMILES is shown with the enlarged structure"
            assert pg.evaluate("getComputedStyle(document.querySelector('#li-big code')).userSelect") == "text" and not pg.is_visible("#lw-back"), "selectable; the row did not open"
            pg.keyboard.press("Escape"); assert pg.locator("#li-big").count() == 0 and pg.evaluate("document.querySelector('#libwhat').open"), "Esc closes the structure before the window"
            pg.evaluate("document.querySelector('#libwhat').close()")
        step("Identify all: the structure thumbnails are drawn for the rows in view only, the new columns fit at 1280 px, a click enlarges the structure with its SMILES", extras)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
