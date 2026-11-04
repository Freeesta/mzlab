from common import *
srv, port = serve(); errs=[]
try:
  with sync_playwright() as p:
    b = p.chromium.launch(args=["--enable-precise-memory-info"]); c = b.new_context(viewport={"width":1500,"height":1100}); c.set_default_timeout(120000)
    pg = newpage(c, errs); boot(pg, port)
    print(opfs(pg))
    pg.set_input_files("#pick", pair("sim"))
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
    pg.wait_for_timeout(4000)
    print(pg.evaluate("ST.files.map(f=>[f.name,f.kind])"))
    print(pg.evaluate("E.panels.map(p=>[p.id,p.type])"), pg.evaluate("E.files.map(f=>[f.name,f.file,f.kind,f.k])"))
    r = opfs(pg); print(r["files"], r["bytes"], r["names"], r["idb"], r["heap"])
    print(pg.evaluate("[window.MZLAB_LR_MISS, typeof LR, Object.keys(window.MZLAB_LR||{})]"))
    pg.screenshot(path="/tmp/stress_soak/probe.png")
    print(errs)
finally: srv.terminate()
