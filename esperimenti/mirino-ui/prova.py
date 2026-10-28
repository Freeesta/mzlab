"""Draws the XIC lens plot (XICDRAW) on synthetic traces in three widths and checks the pixels; writes PNGs next to this file. No app, no lab data."""
import pathlib, math, json
from playwright.sync_api import sync_playwright
HERE = pathlib.Path(__file__).resolve().parent; JS = (HERE.parent.parent / "mzlab/web/xic-lente-disegno.js").read_text(encoding="utf-8")
CASES = {"stretto": (0.05, True), "largo": (1.2, True), "rumore": (None, False)}
PAGE = "<body style='margin:0;background:#fff'><div id=o style='display:flex;flex-wrap:wrap;gap:8px;padding:8px;align-items:flex-start'></div></body>"
RUN = """([widths,cases])=>{const o=document.getElementById('o'),out={};
 for(const [name,[wd,pk]] of Object.entries(cases)){const n=4000,rt=[],y=[];let s=7;const r=()=>(s=(s*16807)%2147483647)/2147483647;
  for(let i=0;i<n;i++){const t=20*i/(n-1);rt.push(t);y.push((pk?Math.exp(-(((t-14.3)/(wd/3.3))**2)/2)*1000:0)+20*r())}
  for(const w of widths){const c=document.createElement('canvas');c.width=w;c.height=96;c.title=name+' '+w;const g=c.getContext('2d');
   const d=XICDRAW.draw(g,w,96,{rt,y},{line:'#bbb',ink:'#111',muted:'#555',accent:'#0072B2',scanRt:null,label:pk});
   const im=g.getImageData(0,0,w,96).data;let top=96;for(let yy=0;yy<96;yy++)for(let xx=0;xx<w;xx++){const i=(yy*w+xx)*4;if(im[i+2]>150&&im[i]<80&&im[i+3]>60){top=Math.min(top,yy);break}}
   out[name+w]={top,h:(d.y1-top)/(d.y1-d.y0)};o.appendChild(c)}}
 return out}"""
with sync_playwright() as p:
    b = p.chromium.launch(executable_path=next(iter(__import__('glob').glob('/opt/pw-browsers/chromium-*/chrome-linux*/chrome')), '/opt/pw-browsers/chromium')); pg = b.new_page(viewport={"width": 1000, "height": 420}); pg.set_content(PAGE); pg.add_script_tag(content=JS)
    res = pg.evaluate(RUN, [[160, 240, 400], CASES]); print(json.dumps(res)); pg.screenshot(path=str(HERE / "dopo.png"))
    assert res["stretto160"]["h"] >= .6, res["stretto160"]
    b.close()
