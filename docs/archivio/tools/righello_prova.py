"""Draws a synthetic spectrum (peaks 152.2, 181.0, 194.2 with number labels) and the ruler 152.2→194.2 and 181.0→194.2 with the old rows (before) and RIGHELLO.layout (after), at 360 and 800 px; PNGs in esperimenti/mirino-ui/. No app, no lab data."""
import glob, pathlib
from playwright.sync_api import sync_playwright
ROOT = pathlib.Path(__file__).resolve().parent.parent; OUT = ROOT / "esperimenti/mirino-ui"
JS = (ROOT / "mzlab/web/spettro-righello.js").read_text(encoding="utf-8")
RUN = """([w,after])=>{const h=240,c=document.createElement('canvas');c.width=w;c.height=h;document.body.appendChild(c);const g=c.getContext('2d');
 const x0=30,x1=w-10,top=14,base=h-26,X=m=>x0+(m-120)/(220-120)*(x1-x0),Y=v=>base-v/100*(base-top-6);
 const pk=[[152.2,92],[181.0,30],[194.2,100],[170.1,40]];g.font='12px sans-serif';g.textAlign='center';g.strokeStyle='#111';
 g.beginPath();g.moveTo(x0,base+.5);g.lineTo(x1,base+.5);g.stroke();const obs=[];
 for(const [m,v] of pk){g.beginPath();g.moveTo(X(m),Y(0));g.lineTo(X(m),Y(v));g.stroke();const t=m.toFixed(1),tw=g.measureText(t).width;g.fillStyle='#111';g.fillText(t,X(m),Y(v)-4);obs.push({x:X(m)-tw/2-3,y:Y(v)-16,w:tw+6,h:15})}
 const ref=null,items=[{xa:X(152.2),xb:X(194.2),w:70},{xa:X(181.0),xb:X(194.2),w:62}],ac='#0072B2';
 const L=RIGHELLO.layout({obs:after?obs:[],x0,x1,top,base,lh:13,ref:{x:X(194.2),w:20},items});
 g.strokeStyle=ac;g.fillStyle=ac;
 const vert=(x,segs)=>{g.setLineDash([2,3]);g.beginPath();for(const [a,b] of segs){g.moveTo(x,a);g.lineTo(x,b)}g.stroke();g.setLineDash([])};
 g.textAlign='left';g.fillText('rif.',L.ref.x,L.ref.y);
 items.forEach((q,i)=>{const {y,bx,va,vb}=L.items[i];vert(q.xa,va);vert(q.xb,vb);g.lineWidth=1.4;g.beginPath();g.moveTo(q.xa,y-4);g.lineTo(q.xa,y+4);g.moveTo(q.xa,y);g.lineTo(q.xb,y);g.moveTo(q.xb,y-4);g.lineTo(q.xb,y+4);g.stroke();g.lineWidth=1;
  g.fillStyle=ac;g.fillText('Δm/z '+(i?'13.2':'42.0'),bx,y-5)});
 return after?L.rects.every(r=>obs.every(o=>!RIGHELLO.hit(r,o))):null}"""
exe = next(iter(glob.glob("/opt/pw-browsers/chromium-*/chrome-linux*/chrome")), "/opt/pw-browsers/chromium")
with sync_playwright() as p:
    b = p.chromium.launch(executable_path=exe)
    for w in (360, 800):
        for tag, after in (("prima", False), ("dopo", True)):
            pg = b.new_page(viewport={"width": w, "height": 240}); pg.set_content("<body style='margin:0;background:#fff'>"); pg.add_script_tag(content=JS)
            ok = pg.evaluate(RUN, [w, after]); pg.screenshot(path=str(OUT / f"righello-{tag}-{w}.png")); pg.close()
            if after: assert ok, "the ruler crosses a peak label"
    b.close()
