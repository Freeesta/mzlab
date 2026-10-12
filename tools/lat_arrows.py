"""Measurement (not a test): latency of scan-by-scan navigation with the arrow keys (local server)."""
import sys, os, json; sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "tests_e2e"))
from lib import *
r = Run(port=int(os.environ.get("LAT_PORT", 8821)), wd=os.environ.get("LAT_WD", "/tmp/wd21"))
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES[:3]]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); pg.wait_for_timeout(4000)
        # raw server latency of api/spectrum for one scan (distinct windows, nothing cached), from inside the page
        raw = pg.evaluate("""async()=>{const s=E.panels.find(p=>p.type==='chrom'||p.type==='tic')||E.panels[0];const a=s._a.sr[0],xs=a.x,k=a.k,out=[];
          for(let i=300;i<340;i++){const t=performance.now();const j=await (await fetch(`api/spectrum?k=${k}&rt0=${xs[i]-0.006}&rt1=${xs[i]+0.006}&level=1&precursor=`)).json();out.push([performance.now()-t,j.mz.length,j.scans])}
          return out}""")
        ms = sorted(x[0] for x in raw); print("raw api/spectrum 1 scan: median %.1f ms, p95 %.1f, max %.1f; peaks/scan %d" % (ms[len(ms)//2], ms[int(len(ms)*.95)], ms[-1], raw[0][1]))
        # instrumentation
        pg.evaluate("""()=>{
          window.LOG={draw:[],fetch:[],stale:0,blank:0,frames:0,paints:0,final:null};
          const od=window.draw; window.draw=async function(p){const t=performance.now(),r0=p.r0,ty=p.type;const r=await od(p);
            LOG.draw.push({ty,ms:performance.now()-t,r0,end:performance.now()}); if(ty==='spec'&&r0!==p.r0)LOG.stale++; if(ty==='spec')LOG.last=r0; return r};
          const of=window.fetch; window.fetch=function(u,...a){const t=performance.now();const pr=of.call(this,u,...a);if(String(u).includes('api/spectrum'))pr.then(()=>LOG.fetch.push(performance.now()-t));return pr};
          const spec=()=>E.panels.find(p=>p.type==='spec'&&p.link!=null);
          const probe=document.createElement('canvas');probe.width=60;probe.height=12;const pc=probe.getContext('2d');
          const loop=()=>{const s=spec();if(s&&s.cv){LOG.frames++;pc.clearRect(0,0,60,12);pc.drawImage(s.cv,0,0,60,12);const d=pc.getImageData(0,0,60,12).data;let n=0;for(let i=3;i<d.length;i+=4)n+=d[i];if(n===0)LOG.blank++}requestAnimationFrame(loop)};loop();}""")
        # put the cursor at the apex of the first chromatogram and make it the active panel
        c = pg.evaluate("""()=>{const p=E.panels[0],r=p.cv.getBoundingClientRect();return {px:r.left+p._a.X(14.3),py:r.top+r.height/2}}""")
        pg.mouse.click(c["px"], c["py"]); pg.wait_for_timeout(800)
        print("spec panels:", pg.evaluate("E.panels.filter(p=>p.type==='spec').map(p=>[p.link,p.k,p.r0])"), "active:", pg.evaluate("!!E.active&&E.active.cur"))
        def run(label, n, gap):
            pg.evaluate("()=>{LOG.draw=[];LOG.fetch=[];LOG.stale=0;LOG.blank=0;LOG.frames=0}")
            t0 = pg.evaluate("performance.now()")
            for i in range(n):
                pg.keyboard.down("ArrowRight"); pg.wait_for_timeout(gap)
            pg.keyboard.up("ArrowRight"); pg.wait_for_timeout(1500)
            L = pg.evaluate("LOG"); d = L["draw"]; sp = sorted(x["ms"] for x in d if x["ty"] == "spec"); ch = sorted(x["ms"] for x in d if x["ty"] != "spec")
            f = sorted(L["fetch"])
            m = lambda a: ("median %.1f p95 %.1f max %.1f n=%d" % (a[len(a)//2], a[int(len(a)*.95)], a[-1], len(a))) if a else "-"
            print(f"[{label}] {n} keypresses every {gap} ms | spec draw ms: {m(sp)} | chrom redraw ms: {m(ch)} | fetch ms: {m(f)} | stale spectrum paints: {L['stale']} | blank frames: {L['blank']}/{L['frames']}")
            cur = pg.evaluate("[E.active.cur, E.panels.find(p=>p.type==='spec'&&p.link!=null).r0, E.panels.find(p=>p.type==='spec'&&p.link!=null).r1]"); print("   final cursor/spec window:", cur, "last painted r0:", L.get("last"))
        run("single steps, uncached", 20, 250)
        pg.keyboard.press("Home") if False else None
        # go back (cached) over the same scans
        for i in range(20): pg.keyboard.press("ArrowLeft"); pg.wait_for_timeout(60)
        pg.wait_for_timeout(500)
        run("held key 30/s, uncached", 60, 33)
        for i in range(60): pg.keyboard.press("ArrowLeft")
        pg.wait_for_timeout(1500)
        run("held key 30/s, forward again (cached)", 60, 33)
        pg.screenshot(path=SH + "lat.png")
finally:
    r.close(); r.report()
