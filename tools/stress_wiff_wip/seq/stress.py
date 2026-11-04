"""seeded random sequences. usage: stress.py SEED NSTEPS FILE...   (FILE = full path without .wiff)"""
from common import *
CHECK = """() => {
 const out=[]; const vis=e=>e&&e.offsetWidth>0&&e.offsetHeight>0;
 const miss=window.MZLAB_LR_MISS; if (miss && miss.length) out.push('LR_MISS '+JSON.stringify(miss).slice(0,200));
 if (window.MZLAB_LR_ERR) out.push('LR_ERR '+JSON.stringify(window.MZLAB_LR_ERR).slice(0,200));
 const fb=[...document.querySelectorAll('[data-i18n*=engineFallback],.engine-fallback')].filter(vis); if (fb.length) out.push('engineFallback banner');
 const stuck=[...document.querySelectorAll('#fl-up [aria-busy=true]')].length;
 if (typeof E==='undefined') return out;
 for (const p of E.panels) { const el=p.el; if(!vis(el)) continue;
   const nm=p.type+'#'+p.id; let ok=false, why='';
   const cvs=[...el.querySelectorAll('canvas')].filter(vis);
   for (const c of cvs){ try{ const x=c.getContext('2d'); if(!x) {ok=true;break;} const w=c.width,h=c.height; const d=x.getImageData(0,0,w,h).data; let n=0; for(let i=3;i<d.length;i+=4*5){ if(d[i]>0 && ++n>40){ok=true;break;} } if(ok)break; }catch(e){ok=true;break;} }
   if(!cvs.length){ const ps=[...el.querySelectorAll('svg path,svg polyline,svg rect,svg line')].filter(e=>vis(e.ownerSVGElement)); ok=ps.length>6; }
   if(!ok) out.push('EMPTY '+nm+' k='+p.k+' lvl='+p.level+' txt='+el.innerText.replace(/\\s+/g,' ').slice(0,90));
   if(/non disponibil|unavailable|errore|error/i.test((el.querySelector('.msg,.empty,.err')||{}).innerText||'')) out.push('MSG '+nm+' '+el.querySelector('.msg,.empty,.err').innerText.slice(0,80));
 }
 return out; }"""
def sig(pg): return pg.evaluate("[E.files.filter(f=>!f.gone).length,E.panels.length]")
def actions_view(rng, pg, js, L, step, fails, files, load):
    def a_solo():
        k=js("(()=>{const a=E.files.filter(f=>!f.gone);return a.length?a[Math.floor(%f*a.length)].k:-1})()"%rng.random())
        if k is not None and k>=0: L(f"solo k={k}"); js("k=>soloFile(E.files[k])",k)
    def a_add(sel): 
        def f(): L("add "+sel); js("s=>document.querySelector(s).click()",sel)
        return f
    def a_kind():
        r=js("""()=>{const p=E.panels.filter(q=>q.type==='chrom'&&q.el&&q.el.offsetWidth>0);if(!p.length)return null;const q=p[0];q.el.querySelector('[data-o=kind]').click();
          const it=[...q.el.querySelectorAll('.can-pop:not([hidden]) [data-kind]')];return [q.id,it.map(i=>i.dataset.kind)]}""")
        if r and r[1]:
            kd=rng.choice(r[1]); L(f"kind {kd}")
            js("""([id,kd])=>{const q=E.panels.find(x=>x.id===id);const i=q&&q.el.querySelector('.can-pop [data-kind="'+kd+'"]');i&&i.click()}""",[r[0],kd])
    def a_canvas_click():
        bb=js("""()=>{const p=E.panels.filter(q=>q.el&&q.el.offsetWidth>0&&q.el.querySelector('canvas'));if(!p.length)return null;const q=p[Math.floor(%f*p.length)];const c=q.el.querySelector('canvas');c.scrollIntoView({block:'center'});const r=c.getBoundingClientRect();return [r.x,r.y,r.width,r.height,q.type]}"""%rng.random())
        if bb:
            x=bb[0]+bb[2]*rng.uniform(.1,.9); y=bb[1]+bb[3]*rng.uniform(.2,.8); L(f"click {bb[4]} {x:.0f},{y:.0f}")
            pg.mouse.click(x,y)
    def a_zoom():
        bb=js("""()=>{const p=E.panels.filter(q=>q.el&&q.el.offsetWidth>0&&q.el.querySelector('canvas'));if(!p.length)return null;const q=p[Math.floor(%f*p.length)];const c=q.el.querySelector('canvas');c.scrollIntoView({block:'center'});const r=c.getBoundingClientRect();return [r.x,r.y,r.width,r.height,q.type]}"""%rng.random())
        if bb:
            x0=bb[0]+bb[2]*rng.uniform(.1,.5); x1=x0+bb[2]*rng.uniform(.05,.4); y=bb[1]+bb[3]*.5; L(f"zoomdrag {bb[4]}")
            pg.mouse.move(x0,y); pg.mouse.down(); pg.mouse.move(x1,y,steps=5); pg.mouse.up()
    def a_unzoom(): L("backspace"); pg.keyboard.press("Backspace")
    def a_tdati():
        r=js("""()=>{const p=E.panels.filter(q=>q.el&&q.el.offsetWidth>0&&q.cv);if(!p.length)return null;const q=p[Math.floor(%f*p.length)];q.cv.scrollIntoView({block:'center'});q.cv.focus();return [q.type,q.id,q.k,q.level]}"""%rng.random())
        if r:
            for _try in range(3):
                js("id=>{const q=E.panels.find(x=>x.id===id);if(q&&q.cv){q.cv.focus()}}",r[1]); pg.keyboard.press("t"); pg.wait_for_timeout(400)
                if js("!!document.getElementById('tdati')"): break
            t=js("(()=>{const d=document.getElementById('tdati');return d?d.querySelectorAll('tbody tr').length:-1})()")
            L(f"tdati {r} rows={t}")
            if t is not None and t<=0:
                dd=js("id=>{const q=E.panels.find(x=>x.id===id);if(!q)return null;const f=E.files[q.k];const a=q._a;return {k:q.k,file:f&&f.file,ftype:f&&f.type,vis:f&&f.vis,lv:q.level,prec:q.prec,r0:q.r0,r1:q.r1,all:q.all,link:q.link,filt:q.filt,adata:a&&a.data&&a.data.length,np:a&&a.data&&a.data.map(x=>x.d&&x.d.mz&&x.d.mz.length),x:a&&[a.x0,a.x1],txt:q.el.innerText.replace(/\\s+/g,' ').slice(0,160)}}",r[1])
                fails.append((step[0],f'TDATI {r} rows={t} {json.dumps(dd)}'))
            js("(()=>{const d=document.getElementById('tdati');if(d){d.close();d.remove()}})()")
    return {k: v for k, v in locals().items() if k.startswith('a_')}
def actions_app(rng, pg, js, L, step, fails, files, load):
    def a_nav():
        v=rng.choice(["data","draw","flow","data","data"]); L("nav "+v); js("v=>document.querySelector('#nav button[data-v='+v+']').click()",v)
        pg.wait_for_timeout(600)
    def a_drawer():
        t=rng.choice(["file","scan","lists","openms2","calc","method","settings","qaqc","search","lst","idn","iso","fil","hdr"]); L("drawer "+t)
        js("t=>{try{BARRA.setTab(t,{noFocus:true,instant:true})}catch(e){}}",t)
    def a_tree():
        L("tree/extract"); js("()=>{const b=document.querySelector('[data-hb=tree]');b&&b.offsetWidth&&b.click()}")
    def a_ms2extract():
        L("ms2 extract btn"); js("()=>{const b=[...document.querySelectorAll('.pnl [data-o]')].find(e=>/ms2|extract|prec/i.test(e.dataset.o)&&e.offsetWidth>0);b&&b.click()}")
    def a_level():
        L("spec level/k"); js("""()=>{const p=E.panels.filter(q=>q.type==='spec'&&q.el);if(!p.length)return;const s=p[0].el.querySelector('select[data-o=k]');if(s&&s.options.length){s.selectedIndex=Math.floor(%f*s.options.length);s.dispatchEvent(new Event('change',{bubbles:true}))}}"""%rng.random())
    def a_sampletab(): L("samptab"); js("()=>{const b=document.getElementById('samptab');b&&b.click()}")
    def a_undo(): L("undo/redo"); js("(r)=>document.getElementById(r?'np-redo':'np-undo').click()",rng.random()<.5)
    def a_closeall(): L("closeall"); js("()=>document.getElementById('np-closeall').click()")
    def a_closeone():
        L("close one panel"); js("()=>{const q=E.panels[E.panels.length-1];const b=q&&q.el&&q.el.querySelector('[data-o=close],[data-o=x]');b&&b.click()}")
    def a_drag_file():
        L("untick/tick file"); js("""()=>{const a=document.querySelectorAll('#flst .fl .nm');if(a.length)a[Math.floor(%f*a.length)].click()}"""%rng.random())
    def a_reload():
        L("RELOAD"); pg.reload(); attendi(pg,"window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange",60000)
        f=rng.choice(files); load(f)
    def a_same_twice():
        f=rng.choice(files); L("reload-same twice"); load(f); load(f)
    def a_opfs():
        r=js("""async()=>{try{const d=await(await navigator.storage.getDirectory()).getDirectoryHandle('mzlab').then(r=>r.getDirectoryHandle('files'));const o=[];for await(const [n,h] of d.entries())o.push(n);return o}catch(e){return []}}""")
        if r:
            dup=[n for n in set(r) if r.count(n)>1]
            if dup: fails.append((step[0],'OPFS dup '+str(dup)))
    return {k: v for k, v in locals().items() if k.startswith('a_')}
def run(seed, nsteps, files, port, b, log):
    rng=random.Random(seed); errs=[]; c,pg=newpage(b,errs,port); fails=[]; step=[0]
    def L(s): log.write(f"[{seed}:{step[0]}] {s}\n"); log.flush()
    def settle():
        try: attendi(pg,"typeof UPBUSY!=='undefined' && UPBUSY===0 && OPENNOW.busy()===0 && E.pending.length===0",120000)
        except Exception as e: fails.append((step[0],'STUCK_LOADING '+str(e)[:100]))
        pg.wait_for_timeout(500)
    def load(f, cache=False):
        L(f"load {f}"); pg.set_input_files("#pick",[f+".wiff",f+".wiff.scan"])
        try: attendi(pg,"ST.files.length>0 && ST.files.every(f=>f.kind) && UPBUSY===0 && OPENNOW.busy()===0",180000)
        except Exception as e: fails.append((step[0],'LOAD_TIMEOUT '+f.split('/')[-1]+' '+str(e)[:60]))
        settle()
    def js(code, *a):
        try: return pg.evaluate(code,*a)
        except Exception as e: fails.append((step[0],'JSERR '+str(e)[:150])); return None
    def ph(): return rng.choice([p for p in js("E.panels.map(q=>q.id)") or [None]])
    A = {**actions_view(rng, pg, js, L, step, fails, files, load), **actions_app(rng, pg, js, L, step, fails, files, load)}
    acts=[(A["a_solo"],6),(A["a_add"]("#np-chrom"),2),(A["a_add"]("#np-spec"),3),(A["a_add"]("#np-xic"),2),(A["a_add"]("#np-map"),1),(A["a_add"]("#np-mrm"),1),(A["a_kind"],3),(A["a_canvas_click"],6),(A["a_zoom"],4),(A["a_unzoom"],2),(A["a_tdati"],3),(A["a_nav"],3),(A["a_drawer"],4),(A["a_tree"],2),(A["a_ms2extract"],1),(A["a_level"],3),(A["a_sampletab"],1),(A["a_undo"],1),(A["a_closeall"],1),(A["a_closeone"],1),(A["a_drag_file"],3),(A["a_reload"],2),(A["a_same_twice"],1),(A["a_opfs"],1)]
    fn=[a for a,w in acts for _ in range(w)]
    load(rng.choice(files))
    for i in range(1,nsteps+1):
        step[0]=i; n0=len(errs); fa0=len(fails)
        a=rng.choice(fn)
        try: a(); 
        except Exception as e: fails.append((i,'ACTIONERR '+a.__name__+' '+str(e)[:150]))
        if a.__name__ not in ("a_reload","a_same_twice"): pg.wait_for_timeout(300); 
        try: settle()
        except Exception: pass
        # only check in Data view
        inData=js("(()=>{const b=document.querySelector('#nav button.on');return b&&b.dataset.v})()")
        res=js(CHECK) if inData=='data' else []
        for r in res or []: fails.append((i,r))
        for e in errs[n0:]: fails.append((i,'PAGEERR '+str(e)))
        if len(fails)>fa0:
            L("FAIL "+json.dumps(fails[fa0:])[:600]); pg.screenshot(path=f"fail_{seed}_{i}.png")
            if len(fails)>40: break
    c.close(); return fails
if __name__=="__main__":
    seed=int(sys.argv[1]); n=int(sys.argv[2]); files=sys.argv[3:]
    port=free_port(); s=serve(port)
    try:
      with sync_playwright() as p:
        b=p.chromium.launch(); log=open(f"log_{seed}.txt","w")
        f=run(seed,n,files,port,b,log); log.write("DONE fails=%d\n"%len(f)); 
        print(seed,len(f)); [print("  ",x) for x in f[:25]]
        b.close()
    finally: s.terminate()
