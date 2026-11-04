from common import *
port=free_port(); s=serve(port); errs=[]
try:
  with sync_playwright() as p:
    b=p.chromium.launch(); c,pg=newpage(b,errs,port)
    A=os.environ["STRESS_WIFF_DIR"].rstrip("/")+"/"
    pg.set_input_files("#pick",[A+"MS2.wiff",A+"MS2.wiff.scan"])
    attendi(pg,"typeof UPBUSY!=='undefined' && UPBUSY===0 && OPENNOW.busy()===0",120000)
    pg.wait_for_timeout(2000)
    print(pg.evaluate("E.files.map(f=>[f.k,f.file,f.type,f.lv,f.vis,f.label])"))
    print(pg.evaluate("E.panels.map(q=>[q.id,q.type,q.tab,!!q.el,q.el&&q.el.className,q.el&&q.el.offsetWidth,q.el&&q.el.querySelectorAll('canvas').length,q.el&&q.el.querySelectorAll('svg').length])"))
    print(pg.evaluate("[...document.querySelectorAll('.pnl')].map(e=>e.className+'|'+e.querySelector('[data-o]')?.dataset.o)"))
    print(pg.evaluate("[...document.querySelectorAll('.pnl.chrom [data-o], .pnl.chrom button')].map(e=>(e.dataset.o||e.id||e.title)).slice(0,60)"))
    print(pg.evaluate("[...document.querySelectorAll('#flst .fgh, #flst .fl')].map(e=>e.className+':'+e.innerText.replace(/\\s+/g,' ').slice(0,50)).slice(0,30)"))
    pg.screenshot(path="p2.png")
    print(errs)
    b.close()
finally: s.terminate()
