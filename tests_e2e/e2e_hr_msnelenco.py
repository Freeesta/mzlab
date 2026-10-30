"""MSn tree window of a DDA file, «Experiments» view (msn-elenco.js): the list of every MSn scan grouped by precursor, sort and filters, a row opens
its MS2 in the DDA panels, a precursor opens its TIC (MS2 tab, ppm window) and its XIC, the tree is one tab away, 50 000 scans stay virtualised, axe, no errors."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import lib
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_msnelenco.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
AXE = lib.axe_path()
D = synth("hrmsnl")
r = Run(port=8946, wd="/tmp/wdmsnl")
PREC = 346.1218
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2)
        attendi(pg, "!!document.querySelector('#hrbar [data-hb=tree]') && !document.querySelector('#hrbar [data-hb=tree]').hidden", 60000)
        def open_list():
            pg.evaluate("(()=>{const d=document.querySelector('#bigdlg');if(d.open)d.close()})()")
            pg.click("#hrbar [data-hb=tree]")
            attendi(pg, "document.querySelectorAll('#mt-pexp .msnl-b .msnl-r').length > 0", 60000)
        def opens():
            open_list()
            v = pg.evaluate("""(()=>({sel:String(!document.querySelector('#mt-tabs')), tree:!document.querySelector('#mt-ptree'),
                n:document.querySelector('#mt-pexp [data-n]').textContent, rows:document.querySelectorAll('#mt-pexp .msnl-r.msnl-g').length,
                cols:[...document.querySelectorAll('#mt-pexp [role=columnheader]')].map(x=>x.textContent),
                want:['msnl.col.no','msnl.col.rt','msnl.col.prec','msnl.col.act','msnl.col.bpi'].map(k=>I18N.t(k)).concat(['MS','z'])}))()"""); print(v)
            assert v["sel"] == "true" and v["tree"] and v["n"] and v["rows"] > 1, v
            assert set(v["want"]) <= set(v["cols"]), v
            d = pg.evaluate("(async()=>{const f=E.files.find(x=>x.dda);const j=await (await fetch('api/dda?k='+f.k)).json();return {lev:j.lev.slice(0,3),z:'z' in j,n:j.sid.length}})()"); print(d)
            assert d["lev"] and all(x >= 2 for x in d["lev"]) and d["z"], d
        step("the tree button of a DDA file opens on «Experiments», with the columns of the brief and /api/dda carrying lev and z", opens)
        def sort_filter():
            pg.click("#mt-pexp [data-c=prec] button")
            m = pg.evaluate("[...document.querySelectorAll('#mt-pexp .msnl-g')].map(r=>+r.children[3].textContent)")
            assert len(m) > 1 and all(m[i] >= m[i + 1] for i in range(len(m) - 1)), m
            assert pg.get_attribute("#mt-pexp [data-c=prec]", "aria-sort") == "descending"
            pg.fill("#mt-pexp [data-f=mz0]", str(PREC - 0.01)); pg.fill("#mt-pexp [data-f=mz1]", str(PREC + 0.01))
            attendi(pg, "document.querySelectorAll('#mt-pexp .msnl-g').length === 1")
            if not pg.is_checked("#mt-pexp [data-f=all]"): pg.check("#mt-pexp [data-f=all]")
            attendi(pg, "document.querySelectorAll('#mt-pexp .msnl-r:not(.msnl-g):not(.msnl-h)').length > 0")
            v = pg.evaluate("[...document.querySelectorAll('#mt-pexp .msnl-r:not(.msnl-g):not(.msnl-h)')].map(r=>[+r.children[1].textContent,r.children[3].textContent,r.children[6].textContent])"); print(v[:3])
            assert all(abs(float(x[1].split(" ")[-1]) - PREC) < 0.01 and x[2] for x in v), v
        step("sort by precursor m/z (descending), filter by m/z, every scan of the precursor with its base peak", sort_filter)
        def row_opens():
            sid = pg.evaluate("(async()=>{const f=E.files.find(x=>x.dda);const D=await DDA.get(f.k);const r=document.querySelector('#mt-pexp .msnl-r:not(.msnl-g):not(.msnl-h)');const no=+r.children[0].textContent;return D.sid[D.no.indexOf(no)]})()")
            pg.click("#mt-pexp .msnl-r:not(.msnl-g):not(.msnl-h)")
            attendi(pg, "!document.querySelector('#bigdlg').open && (()=>{const s=E.panels.find(p=>p.dda!=null);return !!(s && s.sid==" + str(sid) + " && s._a)})()", 20000)
            assert pg.evaluate("E.tab") == "full"
        step("a click on a scan closes the window and shows that MS2 next to its Full Scan", row_opens)
        def click_point(panel, level):
            # the linked spectrum is there at the apex; a click elsewhere on the trace moves it to that RT
            try: attendi(pg, "(()=>{const c=" + panel + ";const s=E.panels.find(q=>q.type==='spec'&&q.link===c.id);return !!(s && s._a && s.level===" + str(level) + " && s.r0!=null)})()", 30000)
            except Exception:
                print(pg.evaluate("(()=>{const c=" + panel + ";return E.panels.filter(q=>q.type==='spec').map(s=>[s.link===c.id,s.tab,s.level,s.r0,!!s._a,s.k,s.prec])})()")); raise
            r0 = pg.evaluate("(()=>{const c=" + panel + ";c.el.scrollIntoView({block:'center',behavior:'instant'});return E.panels.find(q=>q.type==='spec'&&q.link===c.id).r0})()")
            # an MS2-tab trace snaps the click to its nearest scan: aim at the scan farthest from the apex
            box = pg.evaluate("(()=>{const c=" + panel + ";const r=c.cv.getBoundingClientRect();const t=c._a.sr&&c._a.sr[0];let x=r.width*0.3;if(t){let b=-1;t.x.forEach((v,i)=>{if(t.y[i]>0&&Math.abs(v-c.cur)>b){b=Math.abs(v-c.cur);x=c._a.X(v);}});}return {x:r.left+x,y:r.top+r.height/2}})()")
            # the loading screen of the tab change stays up for a moment and would take the click
            attendi(pg, "document.querySelector('#loading').hidden", 20000)
            pg.mouse.click(box["x"], box["y"])
            attendi(pg, "(()=>{const c=" + panel + ";const s=E.panels.find(q=>q.type==='spec'&&q.link===c.id);return Math.abs(s.r0-(" + str(r0) + "))>1e-6})()", 20000)
        def tic():
            # the precursor with the most scans: the activation column sorts the precursors by their number of scans
            open_list()
            pg.click("#mt-pexp [data-c=act] button"); pg.click("#mt-pexp [data-c=act] button")
            attendi(pg, "document.querySelector('#mt-pexp [data-c=act]').getAttribute('aria-sort')==='descending'")
            mz = pg.evaluate("+document.querySelector('#mt-pexp .msnl-g').children[3].textContent")
            n = pg.evaluate("+document.querySelector('#mt-pexp .msnl-g').children[0].textContent.replace(/[^0-9]/g,'')")
            pg.click("#mt-pexp .msnl-g [data-a=tic]")
            attendi(pg, "E.tab==='ms2' && (()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.ptol);return !!(c && c._a)})()", 30000)
            v = pg.evaluate("(async()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.ptol);const f=E.files.find(x=>x.kind==='ms2');const j=await (await fetch(`api/chrom?k=${f.k}&kind=tic&level=2&prec=${c.prec}&ptol=${c.ptol}`)).json();const w=await (await fetch(`api/chrom?k=${f.k}&kind=tic&level=2&prec=${c.prec}`)).json();return {prec:c.prec,ptol:c.ptol,n:j.rt.length,wide:w.rt.length}})()"); print(v, n)
            assert n > 2 and abs(v["prec"] - mz) < 0.01 and v["ptol"] < 0.05 and v["n"] == n and v["n"] <= v["wide"], v
            click_point("E.panels.find(p=>p.type==='chrom'&&p.ptol)", 2)
        step("TIC of the precursor: an MS2-tab chromatogram with the file's window (only that precursor's scans), a click opens a spectrum", tic)
        def xic():
            open_list()
            pg.fill("#mt-pexp [data-f=mz0]", str(PREC - 0.01)); pg.fill("#mt-pexp [data-f=mz1]", str(PREC + 0.01))
            attendi(pg, "document.querySelectorAll('#mt-pexp .msnl-g').length === 1")
            pg.click("#mt-pexp .msnl-g [data-a=xic]")
            attendi(pg, "E.tab==='full' && (()=>{const x=E.panels.find(p=>p.type==='xic'&&p.traces.some(t=>Math.abs(t.mz-" + str(PREC) + ")<0.01));return !!(x && x._a)})()", 30000)
            click_point("E.panels.find(p=>p.type==='xic')", 1)
        step("XIC of the precursor: an XIC panel of the Full Scan at its m/z", xic)
        def ms3():
            open_list()
            # an MS3 sits under the precursor of the MS2 that started it; a file with MS3 gets the «Tree» tab, computed on its first showing, and the arrows move between the tabs
            v = pg.evaluate("""(async()=>{const m=await import(new URL('msn-elenco.js', document.querySelector('script[src*="composizione.js"]').src).href);
              const D={sid:[10,11,12,20],rt:[1,1.01,1.02,2],no:[11,12,13,21],prec:[300.1,150.05,120.02,400.2],tgt:[300.1,150.05,120.02,400.2],parent:[5,10,11,15],lev:[2,3,4,2],z:[1,1,1,2],act:['CID','CID','CID','HCD'],ce:[35,35,35,30],idx:new Map([[10,0],[11,1],[12,2],[20,3]])};
              const G=m.groups(D,()=>0.01);
              const g0=DDA.get; DDA.get=async()=>D; let done=0;
              document.querySelector('#bigbody').innerHTML='<div id="mt-tabs"></div><div id="mt-pexp" role="tabpanel" aria-labelledby="mt-texp"></div><div id="mt-ptree" role="tabpanel" aria-labelledby="mt-ttree" hidden></div>';
              try{await m.tabs(E.files.find(x=>x.dda),()=>{done++});}finally{DDA.get=g0;}
              return {g:G.map(g=>[g.mz,g.idx.length,g.lev.join(',')]),tabs:document.querySelectorAll('#mt-tabs [role=tab]').length,done}})()"""); print(v)
            assert v["g"] == [[300.1, 3, "2,3,4"], [400.2, 1, "2"]] and v["tabs"] == 2 and v["done"] == 0, v
            pg.click("#mt-ttree")
            attendi(pg, "!document.querySelector('#mt-ptree').hidden && document.querySelector('#mt-pexp').hidden")
            pg.focus("#mt-ttree"); pg.keyboard.press("ArrowLeft")
            attendi(pg, "!document.querySelector('#mt-pexp').hidden && document.activeElement.id==='mt-texp'")
        step("MS3 under its first precursor; a file with MS3 gets the «Tree» tab, arrows move between the tabs", ms3)
        def keys():
            open_list()
            pg.focus("#mt-pexp .msnl"); pg.keyboard.press("End")
            v = pg.evaluate("(()=>{const L=document.querySelector('#mt-pexp .msnl');const a=L.getAttribute('aria-activedescendant');return {a,ok:!!document.getElementById(a),last:+L.getAttribute('aria-rowcount')-2}})()"); print(v)
            assert v["ok"] and v["a"] == "msnl-%d" % v["last"], v
        step("keyboard: End goes to the last row, which is rendered and active", keys)
        def axe():
            if not AXE: print("SKIP axe-core not available"); return
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("axe.run(document.querySelector('#bigdlg'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length]))")
            assert not v, v
        step("axe: no serious violation in the window", axe)
        def big():
            # 50 000 scans of 5 000 precursors: the view mounts fast and keeps only the rows in view in the DOM
            v = pg.evaluate("""(async()=>{const m=await import(new URL('msn-elenco.js', document.querySelector('script[src*="composizione.js"]').src).href);
              const n=50000, D={sid:[],rt:[],no:[],prec:[],tgt:[],parent:[],lev:[],z:[],act:[],ce:[],nce:true,idx:new Map()};
              for(let i=0;i<n;i++){D.sid.push(i);D.rt.push(i*0.0006);D.no.push(i+1);const p=150+(i%5000)*0.173;D.prec.push(p);D.tgt.push(p);D.parent.push(null);D.lev.push(2);D.z.push(1+(i%3));D.act.push('HCD');D.ce.push(30);D.idx.set(i,i)}
              const g0=DDA.get; DDA.get=async()=>D; const host=document.createElement('div'); document.body.appendChild(host);
              const t0=performance.now(); try{await m.mount(host,E.files.find(x=>x.dda));}finally{DDA.get=g0;}
              const t1=performance.now(); host.querySelector('[data-f=all]').click(); const t2=performance.now();
              const L=host.querySelector('.msnl'); L.scrollTop=L.scrollHeight; L.dispatchEvent(new Event('scroll'));
              const out={mount:t1-t0,expand:t2-t1,rows:host.querySelectorAll('.msnl-b .msnl-r').length,count:+L.getAttribute('aria-rowcount'),lastShown:!!host.querySelector('[aria-rowindex="'+L.getAttribute('aria-rowcount')+'"]')};
              host.remove(); return out})()"""); print(v)
            assert v["count"] == 55001 and v["rows"] < 120 and v["lastShown"] and v["mount"] < 3000 and v["expand"] < 1500, v
        step("50 000 scans: mounted and expanded quickly, under 120 rows in the DOM, the last row reachable", big)
        print("MISS", pg.evaluate("window.MZLAB_LR_MISS || []"))
        step("no request is left unserved and the page has no error", lambda: (_ for _ in ()).throw(AssertionError([e for e in r.errs if e[0] != "console.warning"][:3])) if ([e for e in r.errs if e[0] != "console.warning"] or pg.evaluate("(window.MZLAB_LR_MISS || []).length")) else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
